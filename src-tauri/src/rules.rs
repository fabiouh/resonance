use crate::{
    error::{message, Result},
    model::{Library, Rule, Track},
};
use serde::Serialize;
use std::collections::BTreeSet;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistDiff {
    pub rule_id: String,
    pub add: Vec<Track>,
    pub remove: Vec<Track>,
}

pub fn validate(library: &Library, rule: &Rule) -> Result<()> {
    if rule.name.trim().is_empty() || rule.name.len() > 120 || rule.sources.is_empty() {
        return Err(message(
            "Give the rule a name and select at least one source playlist.",
        ));
    }
    if rule.sources.contains(&rule.target) {
        return Err(message("A rule's target cannot also be its source."));
    }
    for id in rule.sources.iter().chain(std::iter::once(&rule.target)) {
        if !library.playlists.iter().any(|playlist| &playlist.id == id) {
            return Err(message(
                "One of this rule's playlists is no longer in the library.",
            ));
        }
    }
    // Targets are exclusive and cannot feed another rule, so execution order is immaterial.
    for other in library.rules.iter().filter(|other| other.id != rule.id) {
        if other.target == rule.target
            || other.sources.contains(&rule.target)
            || rule.sources.contains(&other.target)
        {
            return Err(message("Rule targets must be independent. Choose a target that is not used by another rule."));
        }
    }
    Ok(())
}

pub fn diff(library: &Library, rule: &Rule) -> Result<PlaylistDiff> {
    validate(library, rule)?;
    let target = library
        .playlists
        .iter()
        .find(|p| p.id == rule.target)
        .unwrap();
    let existing: BTreeSet<_> = target.tracks.iter().map(|t| t.id.as_str()).collect();
    let mut desired = BTreeSet::new();
    let mut add = Vec::new();
    for source in &rule.sources {
        let playlist = library.playlists.iter().find(|p| &p.id == source).unwrap();
        for track in &playlist.tracks {
            if desired.insert(track.id.as_str()) && !existing.contains(track.id.as_str()) {
                let mut track = track.clone();
                track.item_id = None;
                add.push(track);
            }
        }
    }
    // Only entries inserted by this rule may be removed; manual entries remain untouched.
    let remove = target
        .tracks
        .iter()
        .filter(|track| {
            !desired.contains(track.id.as_str())
                && rule.managed.get(&track.id).is_some_and(|item| {
                    target.remote_id.is_none() || track.item_id.as_ref() == Some(item)
                })
        })
        .cloned()
        .collect();
    Ok(PlaylistDiff {
        rule_id: rule.id.clone(),
        add,
        remove,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Playlist;

    fn track(id: &str) -> Track {
        Track {
            id: id.into(),
            title: id.into(),
            artist: String::new(),
            item_id: None,
        }
    }
    fn playlist(id: &str, ids: &[&str]) -> Playlist {
        Playlist {
            id: id.into(),
            name: id.into(),
            remote_id: None,
            tracks: ids.iter().map(|id| track(id)).collect(),
        }
    }
    fn fixture() -> (Library, Rule) {
        let library = Library {
            playlists: vec![
                playlist("a", &["shared", "first"]),
                playlist("b", &["shared"]),
                playlist("target", &["manual", "shared", "old"]),
            ],
            ..Default::default()
        };
        let rule = Rule {
            id: "rule".into(),
            name: "Auto All".into(),
            sources: vec!["a".into(), "b".into()],
            target: "target".into(),
            enabled: true,
            managed: [("shared".into(), "".into()), ("old".into(), "".into())].into(),
        };
        (library, rule)
    }
    #[test]
    fn preserves_shared_and_manual_tracks() {
        let (mut library, rule) = fixture();
        library.playlists[0].tracks.retain(|t| t.id != "shared");
        let result = diff(&library, &rule).unwrap();
        assert_eq!(
            result.add.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["first"]
        );
        assert_eq!(
            result
                .remove
                .iter()
                .map(|t| t.id.as_str())
                .collect::<Vec<_>>(),
            ["old"]
        );
    }
    #[test]
    fn adds_each_track_once_in_source_order() {
        let (mut library, rule) = fixture();
        library.playlists[2].tracks.clear();
        assert_eq!(
            diff(&library, &rule)
                .unwrap()
                .add
                .iter()
                .map(|t| t.id.as_str())
                .collect::<Vec<_>>(),
            ["shared", "first"]
        );
    }
    #[test]
    fn does_not_remove_a_manually_readded_remote_entry() {
        let (mut library, mut rule) = fixture();
        library.playlists[2].remote_id = Some("remote".into());
        library.playlists[2].tracks[2].item_id = Some("replacement".into());
        rule.managed.insert("old".into(), "original".into());
        assert!(diff(&library, &rule).unwrap().remove.is_empty());
    }
    #[test]
    fn refuses_missing_sources_and_cycles() {
        let (mut library, mut rule) = fixture();
        rule.sources.push("missing".into());
        assert!(diff(&library, &rule).is_err());
        rule.sources = vec!["target".into()];
        assert!(diff(&library, &rule).is_err());
        rule.sources = vec!["a".into()];
        library.rules.push(Rule {
            target: "a".into(),
            id: "other".into(),
            ..rule.clone()
        });
        assert!(diff(&library, &rule).is_err());
    }
}
