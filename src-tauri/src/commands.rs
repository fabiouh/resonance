use crate::{
    auth,
    error::{message, Result},
    library::Core,
    model::{self, Library, Listening, Playlist, Rule, Settings, Track},
    presence::DiscordClient,
    rules::PlaylistDiff,
    youtube::{self, YoutubeClient},
};
use serde::Serialize;
use tauri::{Emitter, Manager, State};
use tokio::sync::Mutex;

pub struct AppState {
    pub config: crate::config::Config,
    pub core: Mutex<Core>,
    pub discord: Mutex<DiscordClient>,
    pub signing_in: Mutex<()>,
    pub playback: Mutex<Option<(String, std::time::Instant)>>,
    pub close_to_tray: std::sync::atomic::AtomicBool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    library: Library,
    connected: bool,
    sync_error: Option<String>,
    updater_configured: bool,
    google_secret_configured: bool,
    imported_history_count: i64,
}

#[tauri::command]
pub async fn snapshot(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<Snapshot> {
    let core = state.core.lock().await;
    Ok(Snapshot {
        imported_history_count: core.store.history_count()?,
        library: core.library.clone(),
        connected: !auth::load()?.refresh_token.is_empty(),
        sync_error: core.sync_error.clone(),
        updater_configured: app.config().plugins.0.contains_key("updater"),
        google_secret_configured: state
            .config
            .client_secret(&core.library.settings.client_id)
            .is_some(),
    })
}

#[tauri::command]
pub async fn save_settings(settings: Settings, state: State<'_, AppState>) -> Result<()> {
    if settings.sync_minutes != 0 && !(5..=1440).contains(&settings.sync_minutes) {
        return Err(message(
            "Background sync must be off or between 5 and 1440 minutes.",
        ));
    }
    if settings.discord_enabled
        && (settings.discord_application_id.len() < 17
            || settings.discord_application_id.len() > 20
            || !settings
                .discord_application_id
                .bytes()
                .all(|b| b.is_ascii_digit()))
    {
        return Err(message(
            "Enter a valid Discord application ID to enable Rich Presence.",
        ));
    }
    let mut core = state.core.lock().await;
    if settings.client_id != core.library.settings.client_id
        && !auth::load()?.refresh_token.is_empty()
    {
        return Err(message(
            "Disconnect Google before changing your OAuth client ID.",
        ));
    }
    let mut next = core.library.clone();
    next.settings = settings.clone();
    core.commit(next)?;
    state
        .close_to_tray
        .store(settings.close_to_tray, std::sync::atomic::Ordering::Relaxed);
    state.discord.lock().await.clear();
    Ok(())
}

#[tauri::command]
pub async fn sign_in(client_secret: String, state: State<'_, AppState>) -> Result<()> {
    let _guard = state
        .signing_in
        .try_lock()
        .map_err(|_| message("Google sign-in is already open in your browser."))?;
    let id = state.core.lock().await.library.settings.client_id.clone();
    if !auth::load()?.refresh_token.is_empty() {
        return Err(message(
            "Disconnect your current Google account before signing in again.",
        ));
    }
    let secret = if client_secret.is_empty() {
        state
            .config
            .client_secret(&id)
            .map(String::from)
            .unwrap_or(auth::load()?.client_secret)
    } else {
        client_secret
    };
    let credentials = auth::sign_in(&id, &secret).await?;
    let core = state.core.lock().await;
    if core.library.settings.client_id != id {
        return Err(message(
            "OAuth configuration changed during sign-in. Try again.",
        ));
    }
    auth::save(&credentials)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>) -> Result<()> {
    let _sign_in = state
        .signing_in
        .try_lock()
        .map_err(|_| message("Wait for the current sign-in attempt to finish."))?;
    let mut core = state.core.lock().await;
    auth::clear()?;
    // Remote identifiers are account-specific; retain the cached music as independent local playlists.
    let mut next = core.library.clone();
    let mut renamed = std::collections::BTreeMap::new();
    for playlist in &mut next.playlists {
        if playlist.remote_id.is_some() {
            let id = uuid::Uuid::new_v4().to_string();
            renamed.insert(playlist.id.clone(), id.clone());
            playlist.id = id;
        }
        playlist.remote_id = None;
        for track in &mut playlist.tracks {
            track.item_id = None;
        }
    }
    for rule in &mut next.rules {
        if let Some(id) = renamed.get(&rule.target) {
            rule.target.clone_from(id);
        }
        for source in &mut rule.sources {
            if let Some(id) = renamed.get(source) {
                source.clone_from(id);
            }
        }
        rule.enabled = false;
        rule.managed.clear();
    }
    next.settings.sync_minutes = 0;
    core.commit(next)
}

#[tauri::command]
pub async fn create_playlist(name: String, remote: bool, state: State<'_, AppState>) -> Result<()> {
    let name = name.trim();
    if name.is_empty() || name.len() > 150 {
        return Err(message("Use a playlist name between 1 and 150 characters."));
    }
    let mut core = state.core.lock().await;
    let playlist = if remote {
        YoutubeClient::connect(&core.library.settings.client_id)
            .await?
            .create_playlist(name)
            .await?
    } else {
        Playlist {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            remote_id: None,
            tracks: Vec::new(),
        }
    };
    let mut next = core.library.clone();
    next.playlists.push(playlist);
    core.commit(next)
}

#[tauri::command]
pub async fn rename_playlist(id: String, name: String, state: State<'_, AppState>) -> Result<()> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 150 {
        return Err(message("Use a playlist name between 1 and 150 characters."));
    }
    let mut core = state.core.lock().await;
    if let Some(remote_id) = &core.playlist(&id)?.remote_id {
        YoutubeClient::connect(&core.library.settings.client_id)
            .await?
            .rename_playlist(remote_id, name)
            .await?;
    }
    let mut next = core.library.clone();
    next.playlists
        .iter_mut()
        .find(|playlist| playlist.id == id)
        .unwrap()
        .name = name.into();
    core.commit(next)
}

#[tauri::command]
pub async fn delete_playlist(
    id: String,
    confirmation: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let mut core = state.core.lock().await;
    let playlist = core.playlist(&id)?;
    if playlist.remote_id.is_some() && confirmation != playlist.name {
        return Err(message(
            "Type the playlist name exactly to confirm deletion from YouTube.",
        ));
    }
    if core
        .library
        .rules
        .iter()
        .any(|r| r.target == id || r.sources.contains(&id))
    {
        return Err(message(
            "Remove rules referencing this playlist before deleting it.",
        ));
    }
    if let Some(remote_id) = &playlist.remote_id {
        YoutubeClient::connect(&core.library.settings.client_id)
            .await?
            .delete_playlist(remote_id)
            .await?;
    }
    let mut next = core.library.clone();
    next.playlists.retain(|p| p.id != id);
    core.commit(next)
}

#[tauri::command]
pub async fn add_track(
    playlist_id: String,
    video_id: String,
    state: State<'_, AppState>,
) -> Result<()> {
    let mut track = youtube::metadata(&video_id).await?;
    let mut core = state.core.lock().await;
    let playlist = core.playlist(&playlist_id)?;
    if playlist.tracks.iter().any(|t| t.id == track.id) {
        return Err(message("This track is already in the playlist."));
    }
    if let Some(remote_id) = &playlist.remote_id {
        track.item_id = Some(
            YoutubeClient::connect(&core.library.settings.client_id)
                .await?
                .insert(remote_id, &video_id)
                .await?,
        );
    }
    let mut next = core.library.clone();
    next.playlists
        .iter_mut()
        .find(|p| p.id == playlist_id)
        .unwrap()
        .tracks
        .push(track);
    core.commit(next)
}

#[tauri::command]
pub async fn remove_track(
    playlist_id: String,
    video_id: String,
    item_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<()> {
    let mut core = state.core.lock().await;
    let playlist = core.playlist(&playlist_id)?;
    let track = playlist
        .tracks
        .iter()
        .find(|t| t.id == video_id && t.item_id == item_id)
        .ok_or_else(|| message("This track is no longer in the playlist."))?;
    if playlist.remote_id.is_some() {
        let item = track
            .item_id
            .as_deref()
            .ok_or_else(|| message("Refresh the playlist before removing this track."))?;
        YoutubeClient::connect(&core.library.settings.client_id)
            .await?
            .remove(item)
            .await?;
    }
    let mut next = core.library.clone();
    next.playlists
        .iter_mut()
        .find(|p| p.id == playlist_id)
        .unwrap()
        .tracks
        .retain(|t| !(t.id == video_id && t.item_id == item_id));
    for rule in next
        .rules
        .iter_mut()
        .filter(|rule| rule.target == playlist_id)
    {
        rule.managed.remove(&video_id);
    }
    core.commit(next)
}

#[tauri::command]
pub async fn save_rule(rule: Rule, state: State<'_, AppState>) -> Result<()> {
    state.core.lock().await.save_rule(rule)
}

#[tauri::command]
pub async fn delete_rule(id: String, state: State<'_, AppState>) -> Result<()> {
    let mut core = state.core.lock().await;
    let mut next = core.library.clone();
    next.rules.retain(|r| r.id != id);
    core.commit(next)
}

#[tauri::command]
pub async fn preview_rules(state: State<'_, AppState>) -> Result<Vec<PlaylistDiff>> {
    state.core.lock().await.previews()
}

#[tauri::command]
pub async fn sync_library(
    app: tauri::AppHandle,
    apply: bool,
    state: State<'_, AppState>,
) -> Result<()> {
    let mut core = state
        .core
        .try_lock()
        .map_err(|_| message("A library operation is already running. Wait for it to finish."))?;
    let result = core.synchronize(apply).await;
    if let Err(error) = &result {
        core.sync_error = Some(error.to_string());
    }
    let _ = app.emit("library-changed", ());
    result
}

#[tauri::command]
pub async fn playback_tick(track: Option<Track>, state: State<'_, AppState>) -> Result<bool> {
    let mut playback = state.playback.lock().await;
    let mut core = state.core.lock().await;
    let settings = core.library.settings.clone();
    if let Some(track) = &track {
        if !model::valid_video_id(&track.id)
            || track.title.len() > 1000
            || track.artist.len() > 1000
        {
            return Err(message("Invalid playback metadata."));
        }
        let now = std::time::Instant::now();
        if let Some((id, previous)) = playback.as_ref() {
            let elapsed = now.duration_since(*previous).as_secs();
            if id == &track.id && (1..=15).contains(&elapsed) {
                let mut next = core.library.clone();
                let entry = next
                    .listening
                    .entry(track.id.clone())
                    .or_insert_with(|| Listening {
                        track: track.clone(),
                        seconds: 0,
                        last_played: 0,
                    });
                entry.seconds += elapsed;
                entry.last_played = model::now();
                core.commit(next)?;
            }
        }
        *playback = Some((track.id.clone(), now));
    } else {
        *playback = None;
    }
    drop(core);
    Ok(state.discord.lock().await.update(&settings, track.as_ref()))
}

#[tauri::command]
pub async fn clear_history(state: State<'_, AppState>) -> Result<()> {
    let mut core = state.core.lock().await;
    let mut next = core.library.clone();
    next.listening.clear();
    core.store.clear_history(&next)?;
    core.library = next;
    Ok(())
}

#[derive(Serialize)]
pub struct ImportResult {
    imported: usize,
    duplicates: usize,
    skipped: usize,
}

#[tauri::command]
pub async fn import_history(contents: String, state: State<'_, AppState>) -> Result<ImportResult> {
    let parsed = crate::history::parse(&contents)?;
    let mut core = state.core.lock().await;
    let imported = core.store.import_history(&parsed.entries)?;
    Ok(ImportResult {
        imported,
        duplicates: parsed.entries.len() - imported,
        skipped: parsed.skipped,
    })
}

#[tauri::command]
pub async fn imported_history(
    offset: u32,
    state: State<'_, AppState>,
) -> Result<Vec<crate::history::ImportedPlay>> {
    state.core.lock().await.store.history_page(offset)
}

#[tauri::command]
pub fn open_youtube(video_id: Option<String>) -> Result<()> {
    let url = match video_id {
        Some(id) if model::valid_video_id(&id) => format!("https://music.youtube.com/watch?v={id}"),
        Some(_) => return Err(message("Invalid YouTube video ID.")),
        None => "https://music.youtube.com".into(),
    };
    open::that(url).map_err(|_| message("Couldn't open YouTube Music in your browser."))
}

pub fn start_background_sync(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_attempt = model::now();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            let state = app.state::<AppState>();
            let Ok(mut core) = state.core.try_lock() else {
                continue;
            };
            let minutes = core.library.settings.sync_minutes;
            if minutes == 0 || model::now().saturating_sub(last_attempt) < u64::from(minutes) * 60 {
                continue;
            }
            last_attempt = model::now();
            if let Err(error) = core.synchronize(true).await {
                core.sync_error = Some(error.to_string());
            }
            let _ = app.emit("library-changed", ());
        }
    });
}
