use crate::{
    error::Result,
    history::ImportedPlay,
    model::{Library, Track},
};
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
             CREATE TABLE IF NOT EXISTS library (id INTEGER PRIMARY KEY CHECK (id = 1), data TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS imported_history (video_id TEXT NOT NULL, played_at INTEGER NOT NULL, title TEXT NOT NULL, artist TEXT NOT NULL, PRIMARY KEY(video_id, played_at));
             CREATE INDEX IF NOT EXISTS history_date ON imported_history(played_at DESC);",
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

    pub fn import_history(&mut self, entries: &[ImportedPlay]) -> Result<usize> {
        let transaction = self.connection.transaction()?;
        let mut inserted = 0;
        {
            let mut statement = transaction.prepare("INSERT OR IGNORE INTO imported_history (video_id, played_at, title, artist) VALUES (?1, ?2, ?3, ?4)")?;
            for entry in entries {
                inserted += statement.execute(rusqlite::params![
                    entry.track.id,
                    entry.played_at,
                    entry.track.title,
                    entry.track.artist
                ])?;
            }
        }
        transaction.commit()?;
        Ok(inserted)
    }

    pub fn history_count(&self) -> Result<i64> {
        Ok(self
            .connection
            .query_row("SELECT COUNT(*) FROM imported_history", [], |row| {
                row.get(0)
            })?)
    }

    pub fn history_page(&self, offset: u32) -> Result<Vec<ImportedPlay>> {
        let mut statement = self.connection.prepare("SELECT video_id, played_at, title, artist FROM imported_history ORDER BY played_at DESC, video_id LIMIT 100 OFFSET ?1")?;
        let entries = statement.query_map([offset], |row| {
            Ok(ImportedPlay {
                track: Track {
                    id: row.get(0)?,
                    title: row.get(2)?,
                    artist: row.get(3)?,
                    item_id: None,
                },
                played_at: row.get(1)?,
            })
        })?;
        Ok(entries.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub fn clear_history(&mut self, library: &Library) -> Result<()> {
        let data = serde_json::to_string(library)?;
        let transaction = self.connection.transaction()?;
        transaction.execute("INSERT INTO library (id, data) VALUES (1, ?1) ON CONFLICT(id) DO UPDATE SET data = excluded.data", [data])?;
        transaction.execute("DELETE FROM imported_history", [])?;
        transaction.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_import_is_deduplicated_persistent_and_separate_from_listening_time() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("library.db");
        let mut store = Store::open(&path).unwrap();
        let entries = [ImportedPlay {
            track: Track {
                id: "abcdefghijk".into(),
                title: "Example".into(),
                ..Default::default()
            },
            played_at: 1234,
        }];
        assert_eq!(store.import_history(&entries).unwrap(), 1);
        assert_eq!(store.import_history(&entries).unwrap(), 0);
        drop(store);
        let mut store = Store::open(&path).unwrap();
        assert_eq!(store.history_count().unwrap(), 1);
        assert_eq!(store.history_page(0).unwrap()[0].played_at, 1234);
        assert!(store.history_page(100).unwrap().is_empty());
        assert!(store.load().unwrap().listening.is_empty());
        store.clear_history(&Library::default()).unwrap();
        assert_eq!(store.history_count().unwrap(), 0);
    }

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
