use crate::{error::Result, model::Library};
use rusqlite::Connection;
use std::path::Path;

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             CREATE TABLE IF NOT EXISTS library (id INTEGER PRIMARY KEY CHECK (id = 1), data TEXT NOT NULL);",
        )?;
        Ok(Self { connection })
    }

    pub fn load(&self) -> Result<Library> {
        use rusqlite::OptionalExtension;
        let data: Option<String> = self
            .connection
            .query_row("SELECT data FROM library WHERE id = 1", [], |row| {
                row.get(0)
            })
            .optional()?;
        match data {
            Some(data) => Ok(serde_json::from_str(&data)?),
            None => Ok(Library::default()),
        }
    }

    pub fn save(&self, library: &Library) -> Result<()> {
        let data = serde_json::to_string(library)?;
        self.connection.execute(
            "INSERT INTO library (id, data) VALUES (1, ?1) ON CONFLICT(id) DO UPDATE SET data = excluded.data",
            [data],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_survives_restart() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("library.db");
        let store = Store::open(&path).unwrap();
        let mut library = store.load().unwrap();
        library.settings.sync_minutes = 30;
        store.save(&library).unwrap();
        drop(store);
        assert_eq!(
            Store::open(&path)
                .unwrap()
                .load()
                .unwrap()
                .settings
                .sync_minutes,
            30
        );
    }

    #[test]
    fn corrupted_data_is_not_replaced_with_an_empty_library() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        store
            .connection
            .execute("INSERT INTO library VALUES (1, 'broken')", [])
            .unwrap();
        assert!(store.load().is_err());
    }
}
