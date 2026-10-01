use crate::{
    error::{message, Result},
    model::{Library, Playlist, Rule},
    rules::{self, PlaylistDiff},
    store::Store,
    youtube::YoutubeClient,
};

pub struct Core {
    pub library: Library,
    pub store: Store,
    pub sync_error: Option<String>,
}

impl Core {
    pub fn commit(&mut self, library: Library) -> Result<()> {
        self.store.save(&library)?;
        self.library = library;
        Ok(())
    }

    pub fn playlist(&self, id: &str) -> Result<&Playlist> {
        self.library
            .playlists
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| message("This playlist is no longer in your library."))
    }

    pub fn previews(&self) -> Result<Vec<PlaylistDiff>> {
        self.library
            .rules
            .iter()
            .filter(|r| r.enabled)
            .map(|r| rules::diff(&self.library, r))
            .collect()
    }

    pub fn save_rule(&mut self, mut rule: Rule) -> Result<()> {
        rule.name = rule.name.trim().into();
        if rule.id.is_empty() {
            rule.id = uuid::Uuid::new_v4().to_string();
        }
        rules::validate(&self.library, &rule)?;
        let mut next = self.library.clone();
        if let Some(old) = next.rules.iter_mut().find(|r| r.id == rule.id) {
            if old.target != rule.target {
                return Err(message(
                    "Create a new rule to use a different target playlist.",
                ));
            }
            rule.managed = old.managed.clone();
            *old = rule;
        } else {
            rule.managed.clear();
            next.rules.push(rule);
        }
        self.commit(next)
    }

    pub async fn refresh(&mut self, youtube: &YoutubeClient) -> Result<()> {
        // Fetch every page before replacing the cache; a partial response must not cause removals.
        let remote = youtube.playlists().await?;
        let mut next = self.library.clone();
        next.playlists.retain(|p| p.remote_id.is_none());
        next.playlists.extend(remote);
        self.commit(next)
    }

    pub async fn apply_rules(&mut self, youtube: Option<&YoutubeClient>) -> Result<()> {
        let previews = self.previews()?;
        for preview in previews {
            let rule = self
                .library
                .rules
                .iter()
                .find(|r| r.id == preview.rule_id)
                .unwrap()
                .clone();
            let target = self.playlist(&rule.target)?.clone();
            for mut track in preview.add {
                let item = if let Some(remote_id) = &target.remote_id {
                    youtube
                        .ok_or_else(|| message("Sign in to apply rules to YouTube playlists."))?
                        .insert(remote_id, &track.id)
                        .await?
                } else {
                    String::new()
                };
                let mut next = self.library.clone();
                if target.remote_id.is_some() {
                    track.item_id = Some(item.clone());
                }
                next.rules
                    .iter_mut()
                    .find(|r| r.id == rule.id)
                    .unwrap()
                    .managed
                    .insert(track.id.clone(), item);
                next.playlists
                    .iter_mut()
                    .find(|p| p.id == target.id)
                    .unwrap()
                    .tracks
                    .push(track);
                // Persist each successful remote write so retries do not repeat completed operations.
                self.commit(next)?;
            }
            for track in preview.remove {
                if target.remote_id.is_some() {
                    let item = track
                        .item_id
                        .as_deref()
                        .ok_or_else(|| message("Refresh this playlist before removing tracks."))?;
                    youtube
                        .ok_or_else(|| message("Sign in to apply rules to YouTube playlists."))?
                        .remove(item)
                        .await?;
                }
                let mut next = self.library.clone();
                next.playlists
                    .iter_mut()
                    .find(|p| p.id == target.id)
                    .unwrap()
                    .tracks
                    .retain(|t| {
                        if target.remote_id.is_some() {
                            t.item_id != track.item_id
                        } else {
                            t.id != track.id
                        }
                    });
                next.rules
                    .iter_mut()
                    .find(|r| r.id == rule.id)
                    .unwrap()
                    .managed
                    .remove(&track.id);
                self.commit(next)?;
            }
        }
        Ok(())
    }

    pub async fn synchronize(&mut self, apply: bool) -> Result<()> {
        let credentials = crate::auth::load()?;
        let youtube = if credentials.refresh_token.is_empty() {
            None
        } else {
            Some(YoutubeClient::connect(&self.library.settings.client_id).await?)
        };
        if let Some(youtube) = &youtube {
            self.refresh(youtube).await?;
        }
        if apply {
            self.apply_rules(youtube.as_ref()).await?;
        }
        let mut next = self.library.clone();
        next.last_sync = Some(crate::model::now());
        self.commit(next)?;
        self.sync_error = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Track;

    #[test]
    fn applying_rules_is_idempotent_and_preserves_manual_tracks() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let directory = tempfile::tempdir().unwrap();
            let store = Store::open(&directory.path().join("library.db")).unwrap();
            let track = |id: &str| Track {
                id: id.into(),
                title: id.into(),
                artist: String::new(),
                item_id: None,
            };
            let playlist = |id: &str, tracks| Playlist {
                id: id.into(),
                name: id.into(),
                remote_id: None,
                tracks,
            };
            let library = Library {
                playlists: vec![
                    playlist("source", vec![track("one"), track("two")]),
                    playlist("other", vec![track("two")]),
                    playlist("target", vec![track("manual")]),
                ],
                rules: vec![Rule {
                    id: "rule".into(),
                    name: "Auto All".into(),
                    target: "target".into(),
                    sources: vec!["source".into(), "other".into()],
                    enabled: true,
                    managed: Default::default(),
                }],
                ..Default::default()
            };
            let mut core = Core {
                store,
                library,
                sync_error: None,
            };
            core.apply_rules(None).await.unwrap();
            core.apply_rules(None).await.unwrap();
            assert_eq!(core.playlist("target").unwrap().tracks.len(), 3);
            core.library.playlists[0].tracks.clear();
            core.apply_rules(None).await.unwrap();
            assert_eq!(
                core.playlist("target")
                    .unwrap()
                    .tracks
                    .iter()
                    .map(|t| t.id.as_str())
                    .collect::<Vec<_>>(),
                ["manual", "two"]
            );
            assert_eq!(core.store.load().unwrap().rules[0].managed.len(), 1);
            core.library.playlists[1].tracks.clear();
            core.apply_rules(None).await.unwrap();
            assert_eq!(core.playlist("target").unwrap().tracks[0].id, "manual");
            assert_eq!(core.playlist("target").unwrap().tracks.len(), 1);
        });
    }
}
