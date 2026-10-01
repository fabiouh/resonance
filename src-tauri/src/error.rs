use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Message(String),
    #[error("Couldn't save the library. Check available disk space and restart Resonance.")]
    Database(#[from] rusqlite::Error),
    #[error("Couldn't read the library data. Restore a backup before continuing.")]
    Json(#[from] serde_json::Error),
    #[error("Couldn't reach YouTube. Check your connection and try again.")]
    Network(#[from] reqwest::Error),
    #[error("Couldn't access local application data. Check your folder permissions.")]
    Io(#[from] std::io::Error),
}

impl Serialize for Error {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn message(text: &str) -> Error {
    Error::Message(text.to_owned())
}
