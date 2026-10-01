use crate::model::{Settings, Track};
use discord_rich_presence::{activity, DiscordIpc, DiscordIpcClient};

#[derive(Default)]
pub struct DiscordClient {
    client: Option<DiscordIpcClient>,
    application_id: String,
}

impl DiscordClient {
    pub fn update(&mut self, settings: &Settings, track: Option<&Track>) -> bool {
        if !settings.discord_enabled
            || track.is_none()
            || settings.discord_application_id != self.application_id
        {
            self.clear();
        }
        if !settings.discord_enabled {
            return false;
        }
        let Some(track) = track else {
            return false;
        };
        if self.client.is_none() {
            let mut client = DiscordIpcClient::new(&settings.discord_application_id);
            if client.connect().is_err() {
                return false;
            }
            self.application_id = settings.discord_application_id.clone();
            self.client = Some(client);
        }
        let title: String = track.title.chars().take(100).collect();
        let artist: String = track.artist.chars().take(100).collect();
        let url = format!("https://www.youtube.com/watch?v={}", track.id);
        let activity = activity::Activity::new()
            .activity_type(activity::ActivityType::Listening)
            .details(&title)
            .state(if artist.is_empty() {
                "YouTube"
            } else {
                &artist
            })
            .buttons(vec![activity::Button::new("Listen on YouTube", &url)]);
        if self
            .client
            .as_mut()
            .unwrap()
            .set_activity(activity)
            .is_err()
        {
            self.clear();
            return false;
        }
        true
    }

    pub fn clear(&mut self) {
        if let Some(mut client) = self.client.take() {
            let _ = client.clear_activity();
            let _ = client.close();
        }
    }
}
