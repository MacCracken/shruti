use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

use crate::session::Session;

/// Handles session persistence using SQLite + audio file pool.
///
/// Session directory layout:
/// ```text
/// my-project.shruti/
///   session.db        # SQLite database (metadata, tracks, regions, undo)
///   audio/            # Pool of audio files
///     file1.wav
///     file2.flac
/// ```
pub struct SessionStore {
    pub path: PathBuf,
    db: Connection,
}

impl SessionStore {
    /// Create a new session directory and database.
    pub fn create(path: &Path, session: &Session) -> Result<Self, Box<dyn std::error::Error>> {
        fs::create_dir_all(path)?;
        fs::create_dir_all(path.join("audio"))?;

        let db_path = path.join("session.db");
        let db = Connection::open(&db_path)?;

        init_schema(&db)?;

        let store = Self {
            path: path.to_owned(),
            db,
        };
        store.save(session)?;

        Ok(store)
    }

    /// Open an existing session directory.
    pub fn open(path: &Path) -> Result<(Self, Session), Box<dyn std::error::Error>> {
        let db_path = path.join("session.db");
        let db = Connection::open(&db_path)?;

        let store = Self {
            path: path.to_owned(),
            db,
        };
        let session = store.load()?;

        Ok((store, session))
    }

    /// Save session metadata to the database.
    pub fn save(&self, session: &Session) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string(session)?;
        self.db.execute(
            "INSERT OR REPLACE INTO session (id, data) VALUES (1, ?1)",
            params![json],
        )?;
        Ok(())
    }

    /// Load session metadata from the database.
    pub fn load(&self) -> Result<Session, Box<dyn std::error::Error>> {
        let json: String =
            self.db
                .query_row("SELECT data FROM session WHERE id = 1", [], |row| {
                    row.get(0)
                })?;
        let session: Session = serde_json::from_str(&json)?;
        Ok(session)
    }

    /// Path to the audio pool directory.
    pub fn audio_dir(&self) -> PathBuf {
        self.path.join("audio")
    }
}

fn init_schema(db: &Connection) -> Result<(), rusqlite::Error> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS session (
            id INTEGER PRIMARY KEY,
            data TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS undo_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            command TEXT NOT NULL,
            timestamp TEXT DEFAULT CURRENT_TIMESTAMP
        );",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_save_load() {
        let tmp = std::env::temp_dir().join("shruti_test_session");
        let _ = fs::remove_dir_all(&tmp);

        let mut session = Session::new("Test Project", 48000, 256);
        session.add_audio_track("Guitar");
        session.add_audio_track("Vocals");
        session.transport.bpm = 140.0;

        let store = SessionStore::create(&tmp, &session).unwrap();
        let loaded = store.load().unwrap();

        assert_eq!(loaded.name, "Test Project");
        assert_eq!(loaded.sample_rate, 48000);
        assert_eq!(loaded.transport.bpm, 140.0);
        assert_eq!(loaded.track_count(), 3); // 2 audio + master

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_session_open() {
        let tmp = std::env::temp_dir().join("shruti_test_session_open");
        let _ = fs::remove_dir_all(&tmp);

        let session = Session::new("Reopen Test", 44100, 512);
        SessionStore::create(&tmp, &session).unwrap();

        let (_store, loaded) = SessionStore::open(&tmp).unwrap();
        assert_eq!(loaded.name, "Reopen Test");
        assert_eq!(loaded.sample_rate, 44100);

        let _ = fs::remove_dir_all(&tmp);
    }
}
