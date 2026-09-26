use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::{collections::HashMap, env, fs, path::{Path, PathBuf}};

const LAST_MODE_KEY: &str = "last_mode";
const SCHEMA: &str = "
    PRAGMA journal_mode = WAL;
    CREATE TABLE IF NOT EXISTS usage (
        command_id TEXT PRIMARY KEY,
        use_count  INTEGER NOT NULL DEFAULT 0,
        last_used  TEXT NOT NULL
    );
    CREATE TABLE IF NOT EXISTS meta (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );";

pub struct Usage {
    pub use_count: i64,
    pub last_used: DateTime<Utc>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open_default() -> Result<Self> {
        let path = env::var_os("CMDPAL_DB")
            .map(PathBuf::from)
            .or_else(|| dirs::data_dir().map(|dir| dir.join("cmdpal").join("db.sqlite")))
            .ok_or_else(|| anyhow::anyhow!("no data directory available"))?;
        Self::open(&path)
    }

    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        match Self::init(path) {
            Err(_) if path.exists() => {
                let backup = format!("{}.bak-{}", path.display(), Utc::now().timestamp());
                fs::rename(path, backup)?;
                Self::init(path)
            }
            result => result,
        }
    }

    fn init(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn record_use(&self, command_id: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO usage (command_id, use_count, last_used) VALUES (?1, 1, ?2)
             ON CONFLICT(command_id) DO UPDATE SET use_count = use_count + 1, last_used = ?2",
            params![command_id, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn last_mode(&self) -> Option<String> {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [LAST_MODE_KEY], |row| row.get(0))
            .ok()
    }

    pub fn set_last_mode(&self, mode: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            params![LAST_MODE_KEY, mode],
        )?;
        Ok(())
    }

    pub fn usage_map(&self) -> Result<HashMap<String, Usage>> {
        let mut statement = self.conn.prepare("SELECT command_id, use_count, last_used FROM usage")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?))
        })?;
        Ok(rows
            .flatten()
            .filter_map(|(id, use_count, stamp)| {
                let last_used = DateTime::parse_from_rfc3339(&stamp).ok()?.with_timezone(&Utc);
                Some((id, Usage { use_count, last_used }))
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_use_increments() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("db.sqlite")).unwrap();
        db.record_use("rm-rf").unwrap();
        db.record_use("rm-rf").unwrap();
        assert_eq!(db.usage_map().unwrap()["rm-rf"].use_count, 2);
    }

    #[test]
    fn last_mode_round_trips_and_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("db.sqlite")).unwrap();
        assert_eq!(db.last_mode(), None);
        db.set_last_mode("git").unwrap();
        db.set_last_mode("docker").unwrap();
        assert_eq!(db.last_mode().as_deref(), Some("docker"));
    }

    #[test]
    fn corrupt_file_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("db.sqlite");
        fs::write(&path, "not a database, definitely longer than a sqlite header ........").unwrap();
        let db = Database::open(&path).unwrap();
        assert!(db.usage_map().unwrap().is_empty());
        assert_eq!(fs::read_dir(dir.path()).unwrap().flatten().filter(|e| e.file_name().to_string_lossy().contains(".bak-")).count(), 1);
    }
}
