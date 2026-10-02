use crate::{
    error::{message, Result},
    model::{valid_video_id, Track},
};
use serde::Serialize;
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use url::Url;

pub const MAX_IMPORT_BYTES: usize = 20 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedPlay {
    pub track: Track,
    pub played_at: i64,
}

pub struct ParsedHistory {
    pub entries: Vec<ImportedPlay>,
    pub skipped: usize,
}

pub fn parse(contents: &str) -> Result<ParsedHistory> {
    if contents.len() > MAX_IMPORT_BYTES {
        return Err(message("Choose a history file smaller than 20 MB. Export a shorter date range from Google Takeout."));
    }
    let value: Value = serde_json::from_str(contents).map_err(|_| message("This file is not valid JSON. Export your YouTube watch history as JSON from Google Takeout."))?;
    let items = value.as_array().ok_or_else(|| {
        message("Choose the watch-history.json file from a Google Takeout export.")
    })?;
    if items.len() > 50_000 {
        return Err(message(
            "Import at most 50,000 history entries at a time. Export a shorter date range.",
        ));
    }
    let entries: Vec<_> = items.iter().filter_map(parse_entry).collect();
    let skipped = items.len() - entries.len();
    Ok(ParsedHistory { entries, skipped })
}

fn parse_entry(item: &Value) -> Option<ImportedPlay> {
    let url = Url::parse(item["titleUrl"].as_str()?).ok()?;
    if !matches!(url.scheme(), "https" | "http")
        || !matches!(
            url.host_str(),
            Some("www.youtube.com" | "youtube.com" | "music.youtube.com" | "m.youtube.com")
        )
        || url.path() != "/watch"
    {
        return None;
    }
    let id = url
        .query_pairs()
        .find(|(key, _)| key == "v")?
        .1
        .into_owned();
    if !valid_video_id(&id) {
        return None;
    }
    let time = OffsetDateTime::parse(item["time"].as_str()?, &Rfc3339).ok()?;
    let played_at = i64::try_from(time.unix_timestamp_nanos() / 1_000_000).ok()?;
    if played_at < 0 {
        return None;
    }
    let title = item["title"].as_str()?;
    let title = ["Watched ", "Angesehen: ", "Vous avez regardé "]
        .iter()
        .find_map(|prefix| title.strip_prefix(prefix))
        .unwrap_or(title);
    if title.trim().is_empty() {
        return None;
    }
    let artist = item["subtitles"][0]["name"].as_str().unwrap_or_default();
    Some(ImportedPlay {
        track: Track {
            id,
            title: title.trim().chars().take(1000).collect(),
            artist: artist.chars().take(1000).collect(),
            item_id: None,
        },
        played_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn imports_watch_events_without_inventing_listening_duration() {
        let data = json!([
            {"title": "Watched Example", "titleUrl": "https://www.youtube.com/watch?v=abcdefghijk", "time": "2024-01-01T12:00:00.123Z", "subtitles": [{"name": "Channel"}]},
            {"title": "Search", "titleUrl": "https://www.youtube.com/results?search_query=music", "time": "2024-01-01T12:00:00Z"},
            {"title": "Invalid host", "titleUrl": "https://youtube.com.example.test/watch?v=abcdefghijk", "time": "2024-01-01T12:00:00Z"},
            {"title": "Invalid date", "titleUrl": "https://www.youtube.com/watch?v=abcdefghijk", "time": "not-a-date"}
        ]);
        let result = parse(&data.to_string()).unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.skipped, 3);
        assert_eq!(result.entries[0].track.title, "Example");
        assert_eq!(result.entries[0].track.artist, "Channel");
        assert_eq!(result.entries[0].played_at, 1_704_110_400_123);
    }

    #[test]
    fn rejects_wrong_files_and_limits_input_size() {
        assert!(parse("not json").is_err());
        assert!(parse("{}").is_err());
        assert!(parse(&" ".repeat(MAX_IMPORT_BYTES + 1)).is_err());
        assert_eq!(parse("[]").unwrap().entries.len(), 0);
    }
}
