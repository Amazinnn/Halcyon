//! Minimal SQLite spike storage: schema_migrations + spike_probes.
//! Full product schema (design doc §15) is intentionally deferred.

use rusqlite::{params, Connection};
use std::path::Path;

pub struct Store {
    conn: Connection,
}

#[derive(Debug)]
#[allow(dead_code)] // used by the full Activity Tracker later; write path is exercised by probe inserts
pub struct ProbeRow {
    pub id: i64,
    pub name: String,
    pub value: String,
    pub recorded_at: String,
}

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn migrate(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                applied_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS spike_probes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                value TEXT NOT NULL,
                recorded_at TEXT NOT NULL
            );
            INSERT OR IGNORE INTO schema_migrations (name, applied_at)
                VALUES ('0001_spike_probes', datetime('now'));
            ",
        )
    }

    pub fn insert_probe(&self, name: &str, value: &str) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO spike_probes (name, value, recorded_at) VALUES (?1, ?2, datetime('now'))",
            params![name, value],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    #[allow(dead_code)]
    pub fn recent_probes(&self, limit: usize) -> rusqlite::Result<Vec<ProbeRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, value, recorded_at FROM spike_probes ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], |r| {
            Ok(ProbeRow {
                id: r.get(0)?,
                name: r.get(1)?,
                value: r.get(2)?,
                recorded_at: r.get(3)?,
            })
        })?;
        rows.collect()
    }
}