use crate::model::Settings;

pub struct Config {
    google_client_id: String,
    google_client_secret: String,
    discord_application_id: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self::read(|name| std::env::var(name).ok())
    }

    fn read(value: impl Fn(&str) -> Option<String>) -> Self {
        let get = |name| value(name).unwrap_or_default().trim().to_owned();
        Self {
            google_client_id: get("GOOGLE_CLIENT_ID"),
            google_client_secret: get("GOOGLE_CLIENT_SECRET"),
            discord_application_id: get("DISCORD_APPLICATION_ID"),
        }
    }

    pub fn apply_defaults(&self, settings: &mut Settings) {
        if settings.client_id.is_empty() {
            settings.client_id.clone_from(&self.google_client_id);
        }
        if settings.discord_application_id.is_empty() {
            settings
                .discord_application_id
                .clone_from(&self.discord_application_id);
        }
    }

    pub fn client_secret(&self, client_id: &str) -> Option<&str> {
        (!client_id.is_empty()
            && client_id == self.google_client_id
            && !self.google_client_secret.is_empty())
        .then_some(self.google_client_secret.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_settings_take_precedence_and_sharing_stays_opt_in() {
        let config = Config::read(|key| {
            Some(
                match key {
                    "GOOGLE_CLIENT_ID" => "environment-client",
                    "DISCORD_APPLICATION_ID" => "configured-application",
                    _ => "",
                }
                .into(),
            )
        });
        let mut settings = Settings {
            client_id: "saved-client".into(),
            ..Default::default()
        };
        config.apply_defaults(&mut settings);
        assert_eq!(settings.client_id, "saved-client");
        assert_eq!(settings.discord_application_id, "configured-application");
        assert!(!settings.discord_enabled);
        assert_eq!(settings.sync_minutes, 0);
    }

    #[test]
    fn secret_is_only_used_for_its_matching_client() {
        let config = Config {
            google_client_id: "client".into(),
            google_client_secret: "fixture".into(),
            discord_application_id: String::new(),
        };
        assert!(config.client_secret("different-client").is_none());
        assert!(config.client_secret("").is_none());
        assert_eq!(config.client_secret("client"), Some("fixture"));
    }
}
