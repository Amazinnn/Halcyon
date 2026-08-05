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
            CREATE TABLE IF NOT EXISTS focus_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                started_at TEXT NOT NULL,
                ended_at TEXT NOT NULL,
                duration_sec INTEGER NOT NULL,
                task_id TEXT
            );
            CREATE TABLE IF NOT EXISTS supervision_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                occurred_at TEXT NOT NULL,
                rule TEXT NOT NULL,
                app TEXT,
                level INTEGER NOT NULL
            );
            INSERT OR IGNORE INTO schema_migrations (name, applied_at)
                VALUES ('0001_spike_probes', datetime('now'));
            INSERT OR IGNORE INTO schema_migrations (name, applied_at)
                VALUES ('0002_focus_supervision', datetime('now'));
            ",
        )
    }

    /// Record a completed focus round (pomodoro) with local-time timestamps.
    pub fn record_focus_session(
        &self,
        started_at: &str,
        ended_at: &str,
        duration_sec: i64,
        task_id: Option<&str>,
    ) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO focus_sessions (started_at, ended_at, duration_sec, task_id)
             VALUES (?1, ?2, ?3, ?4)",
            params![started_at, ended_at, duration_sec, task_id],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Record one supervision reminder.
    pub fn record_supervision_event(&self, rule: &str, app: Option<&str>, level: i64) -> rusqlite::Result<i64> {
        self.conn.execute(
            "INSERT INTO supervision_events (occurred_at, rule, app, level)
             VALUES (datetime('now','localtime'), ?1, ?2, ?3)",
            params![rule, app, level],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Today's focus summary: (total seconds, completed rounds) for the local day.
    pub fn today_focus_summary(&self) -> rusqlite::Result<(i64, i64)> {
        let row = self.conn.query_row(
            "SELECT COALESCE(SUM(duration_sec),0), COUNT(*)
             FROM focus_sessions WHERE date(ended_at) = date('now','localtime')",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        Ok(row)
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