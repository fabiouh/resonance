use crate::{
    auth,
    error::{message, Result},
    model::{valid_video_id, Playlist, Track},
};
use reqwest::Method;
use serde::Deserialize;
use serde_json::{json, Value};

pub struct YoutubeClient {
    http: reqwest::Client,
    token: String,
}

impl YoutubeClient {
    pub async fn connect(client_id: &str) -> Result<Self> {
        Ok(Self {
            http: auth::http()?,
            token: auth::access_token(client_id).await?,
        })
    }

    async fn request(
        &self,
        method: Method,
        endpoint: &str,
        query: &[(&str, &str)],
        body: Option<Value>,
    ) -> Result<Value> {
        let mut request = self
            .http
            .request(
                method,
                format!("https://www.googleapis.com/youtube/v3/{endpoint}"),
            )
            .bearer_auth(&self.token)
            .query(query);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await?;
        let status = response.status();
        if status == reqwest::StatusCode::NO_CONTENT {
            return Ok(Value::Null);
        }
        let value: Value = response.json().await?;
        if !status.is_success() {
            let reason = value
                .pointer("/error/errors/0/reason")
                .and_then(Value::as_str)
                .unwrap_or_default();
            return Err(message(match reason {
                "quotaExceeded" | "dailyLimitExceeded" => "YouTube's daily API quota is exhausted. Sync will be available after the quota resets.",
                "rateLimitExceeded" | "userRateLimitExceeded" => "YouTube is limiting requests. Try syncing again later.",
                "playlistNotFound" => "This YouTube playlist is no longer available. Refresh your library.",
                "playlistItemsNotAccessible" | "forbidden" => "Google denied access to this playlist. Check that your account owns it.",
                "videoNotFound" => "This video is unavailable or private.",
                _ if status.as_u16() == 401 => "Google session expired. Sign in again in Settings.",
                _ => "YouTube couldn't complete the operation. Check your account permissions and API configuration.",
            }));
        }
        Ok(value)
    }

    async fn list(&self, endpoint: &str, query: &[(&str, &str)]) -> Result<Vec<Value>> {
        let mut items = Vec::new();
        let mut page = String::new();
        loop {
            let mut query = query.to_vec();
            query.extend([("maxResults", "50"), ("pageToken", page.as_str())]);
            let value = self.request(Method::GET, endpoint, &query, None).await?;
            let batch = value["items"].as_array().ok_or_else(|| message("YouTube returned an incomplete playlist response. No library changes were applied."))?;
            items.extend(batch.iter().cloned());
            match value["nextPageToken"].as_str() {
                Some(next) if next != page => page = next.into(),
                Some(_) => {
                    return Err(message(
                        "YouTube returned a repeated page. Try syncing again.",
                    ))
                }
                None => return Ok(items),
            }
        }
    }

    pub async fn playlists(&self) -> Result<Vec<Playlist>> {
        let items = self
            .list("playlists", &[("part", "snippet"), ("mine", "true")])
            .await?;
        let mut playlists = Vec::new();
        for item in items {
            let id = item["id"]
                .as_str()
                .ok_or_else(|| message("YouTube returned a playlist without an ID."))?;
            playlists.push(Playlist {
                id: format!("yt:{id}"),
                name: item["snippet"]["title"]
                    .as_str()
                    .unwrap_or("Untitled playlist")
                    .into(),
                remote_id: Some(id.into()),
                tracks: self.tracks(id).await?,
            });
        }
        Ok(playlists)
    }

    pub async fn tracks(&self, playlist_id: &str) -> Result<Vec<Track>> {
        let items = self
            .list(
                "playlistItems",
                &[("part", "snippet"), ("playlistId", playlist_id)],
            )
            .await?;
        items.iter().map(parse_track).collect()
    }

    pub async fn create_playlist(&self, name: &str) -> Result<Playlist> {
        let item = self.request(Method::POST, "playlists", &[("part", "snippet,status")], Some(json!({ "snippet": { "title": name }, "status": { "privacyStatus": "private" } }))).await?;
        let id = item["id"].as_str().ok_or_else(|| {
            message(
                "YouTube didn't return the new playlist ID. Sync your library before trying again.",
            )
        })?;
        Ok(Playlist {
            id: format!("yt:{id}"),
            name: name.into(),
            remote_id: Some(id.into()),
            tracks: Vec::new(),
        })
    }

    pub async fn rename_playlist(&self, id: &str, name: &str) -> Result<()> {
        let response = self
            .request(
                Method::GET,
                "playlists",
                &[("part", "snippet"), ("id", id)],
                None,
            )
            .await?;
        let current = response["items"]
            .as_array()
            .and_then(|items| items.first())
            .ok_or_else(|| {
                message("This YouTube playlist is no longer available. Refresh your library.")
            })?;
        let body = rename_body(current, name)?;
        self.request(Method::PUT, "playlists", &[("part", "snippet")], Some(body))
            .await?;
        Ok(())
    }

    pub async fn delete_playlist(&self, id: &str) -> Result<()> {
        self.request(Method::DELETE, "playlists", &[("id", id)], None)
            .await?;
        Ok(())
    }

    pub async fn insert(&self, playlist_id: &str, video_id: &str) -> Result<String> {
        let item = self.request(Method::POST, "playlistItems", &[("part", "snippet")], Some(json!({ "snippet": { "playlistId": playlist_id, "resourceId": { "kind": "youtube#video", "videoId": video_id } } }))).await?;
        item["id"].as_str().map(String::from).ok_or_else(|| message("YouTube didn't return the inserted item ID. Sync your library before trying again."))
    }

    pub async fn remove(&self, item_id: &str) -> Result<()> {
        self.request(Method::DELETE, "playlistItems", &[("id", item_id)], None)
            .await?;
        Ok(())
    }
}

fn rename_body(current: &Value, name: &str) -> Result<Value> {
    let id = current["id"].as_str().ok_or_else(|| {
        message("YouTube returned an incomplete playlist. Refresh and try again.")
    })?;
    let snippet = current["snippet"].as_object().ok_or_else(|| {
        message("YouTube returned an incomplete playlist. Refresh and try again.")
    })?;
    // Updating snippet replaces omitted writable fields, including the description.
    let mut next = serde_json::Map::new();
    next.insert("title".into(), Value::String(name.into()));
    for key in ["description", "defaultLanguage"] {
        if let Some(value) = snippet.get(key) {
            next.insert(key.into(), value.clone());
        }
    }
    Ok(json!({ "id": id, "snippet": next }))
}

fn parse_track(item: &Value) -> Result<Track> {
    let incomplete = || {
        message("YouTube returned an incomplete playlist entry. No library changes were applied.")
    };
    let snippet = &item["snippet"];
    let id = snippet["resourceId"]["videoId"]
        .as_str()
        .filter(|id| valid_video_id(id))
        .ok_or_else(incomplete)?;
    let item_id = item["id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or_else(incomplete)?;
    // Unavailable videos still reference a source entry and must not trigger target removals.
    Ok(Track {
        id: id.into(),
        title: snippet["title"]
            .as_str()
            .unwrap_or("Unavailable video")
            .into(),
        artist: snippet["videoOwnerChannelTitle"]
            .as_str()
            .unwrap_or_default()
            .into(),
        item_id: Some(item_id.into()),
    })
}

pub async fn metadata(id: &str) -> Result<Track> {
    if !valid_video_id(id) {
        return Err(message(
            "Enter a valid YouTube video URL or 11-character video ID.",
        ));
    }
    #[derive(Deserialize)]
    struct Embed {
        title: String,
        author_name: String,
    }
    let response = auth::http()?
        .get("https://www.youtube.com/oembed")
        .query(&[
            ("url", format!("https://www.youtube.com/watch?v={id}")),
            ("format", "json".into()),
        ])
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(message(
            "Couldn't load this video. It may be private, removed, or unavailable for embedding.",
        ));
    }
    let embed: Embed = response.json().await?;
    Ok(Track {
        id: id.into(),
        title: embed.title,
        artist: embed.author_name,
        item_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renaming_preserves_writable_metadata_without_changing_privacy() {
        let current = json!({"id": "playlist", "snippet": {"title": "Old", "description": "Keep this", "defaultLanguage": "de", "channelId": "owner"}, "status": {"privacyStatus": "private"}});
        assert_eq!(
            rename_body(&current, "New").unwrap(),
            json!({"id": "playlist", "snippet": {"title": "New", "description": "Keep this", "defaultLanguage": "de"}})
        );
        assert!(rename_body(&json!({"id": "playlist"}), "New").is_err());
    }

    #[test]
    fn preserves_private_video_references_and_item_ids() {
        let item = json!({"id": "playlist-item", "snippet": {"title": "Private video", "resourceId": {"videoId": "abcdefghijk"}}});
        let track = parse_track(&item).unwrap();
        assert_eq!(track.id, "abcdefghijk");
        assert_eq!(track.item_id.as_deref(), Some("playlist-item"));
    }

    #[test]
    fn refuses_incomplete_entries_instead_of_silently_dropping_them() {
        assert!(parse_track(&json!({"id": "item", "snippet": {}})).is_err());
        assert!(
            parse_track(&json!({"snippet": {"resourceId": {"videoId": "abcdefghijk"}}})).is_err()
        );
    }
}
