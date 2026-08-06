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

/// One desktop shortcut (v1.5): definition lives in `app_shortcuts`, its free
/// placement + window-fit slot in `ui_layouts`.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutRow {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String, // file|folder|application|url|internal
    pub target: String,
    pub col: i64,
    pub row: i64,
    pub fit_col: Option<i64>,
    pub fit_row: Option<i64>,
    pub fit_cols: Option<i64>,
    pub fit_rows: Option<i64>,
}

/// A completed focus session for CLI/reporting.
#[derive(Debug, Clone)]
pub struct FocusSessionRow {
    pub id: i64,
    pub started_at: String,
    pub ended_at: String,
    pub duration_sec: i64,
    pub task_id: Option<String>,
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
            CREATE TABLE IF NOT EXISTS app_shortcuts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                target TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS ui_layouts (
                shortcut_id TEXT PRIMARY KEY,
                col INTEGER NOT NULL,
                row INTEGER NOT NULL,
                fit_col INTEGER,
                fit_row INTEGER,
                fit_cols INTEGER,
                fit_rows INTEGER
            );
            INSERT OR IGNORE INTO schema_migrations (name, applied_at)
                VALUES ('0003_shortcuts_layouts', datetime('now'));
            ",
        )?;

        // 0004: agent CLI audit payload column (ADR-0007)
        let cols: Vec<String> = self
            .conn
            .prepare("PRAGMA table_info(supervision_events)")?
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<Result<_, _>>()?;
        if !cols.iter().any(|c| c == "payload") {
            self.conn
                .execute_batch("ALTER TABLE supervision_events ADD COLUMN payload TEXT;")?;
        }
        self.conn.execute(
            "INSERT OR IGNORE INTO schema_migrations (name, applied_at)
             VALUES ('0004_agent_cli_audit', datetime('now'))",
            [],
        )?;
        Ok(())
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

    /// Audit one agent-triggered focus-cli call (ADR-0007 whitelist+audit).
    pub fn record_agent_cli_call(
        &self,
        thread_id: &str,
        command: &str,
        allowed: bool,
        result: &str,
    ) -> rusqlite::Result<i64> {
        let payload = serde_json::json!({
            "threadId": thread_id,
            "command": command,
            "allowed": allowed,
            "result": result,
        })
        .to_string();
        self.conn.execute(
            "INSERT INTO supervision_events (occurred_at, rule, app, level, payload)
             VALUES (datetime('now','localtime'), 'agent_cli_call', ?1, 0, ?2)",
            params![thread_id, payload],
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

    // ---- v1.5 desktop shortcuts + layouts (DB-backed) ----

    /// All shortcuts joined with their layout (free placement + fit slot).
    pub fn list_shortcuts(&self) -> rusqlite::Result<Vec<ShortcutRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT s.id, s.name, s.type, s.target,
                    l.col, l.row, l.fit_col, l.fit_row, l.fit_cols, l.fit_rows
             FROM app_shortcuts s
             LEFT JOIN ui_layouts l ON l.shortcut_id = s.id
             ORDER BY s.created_at",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(ShortcutRow {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: r.get(2)?,
                target: r.get(3)?,
                col: r.get::<_, Option<i64>>(4)?.unwrap_or(0),
                row: r.get::<_, Option<i64>>(5)?.unwrap_or(0),
                fit_col: r.get(6)?,
                fit_row: r.get(7)?,
                fit_cols: r.get(8)?,
                fit_rows: r.get(9)?,
            })
        })?;
        rows.collect()
    }

    pub fn insert_shortcut(&self, row: &ShortcutRow) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO app_shortcuts (id, name, type, target, created_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now','localtime'))",
            params![row.id, row.name, row.kind, row.target],
        )?;
        self.conn.execute(
            "INSERT INTO ui_layouts (shortcut_id, col, row)
             VALUES (?1, ?2, ?3)",
            params![row.id, row.col, row.row],
        )?;
        Ok(())
    }

    pub fn delete_shortcut(&self, id: &str) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM app_shortcuts WHERE id = ?1", params![id])?;
        self.conn.execute("DELETE FROM ui_layouts WHERE shortcut_id = ?1", params![id])?;
        Ok(())
    }

    pub fn move_shortcut(&self, id: &str, col: i64, row: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO ui_layouts (shortcut_id, col, row)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(shortcut_id) DO UPDATE SET col = excluded.col, row = excluded.row",
            params![id, col, row],
        )?;
        Ok(())
    }

    pub fn set_shortcut_fit(
        &self,
        id: &str,
        fit_col: i64,
        fit_row: i64,
        fit_cols: i64,
        fit_rows: i64,
    ) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO ui_layouts (shortcut_id, col, row, fit_col, fit_row, fit_cols, fit_rows)
             VALUES (?1, 0, 0, ?2, ?3, ?4, ?5)
             ON CONFLICT(shortcut_id) DO UPDATE SET
                fit_col = excluded.fit_col, fit_row = excluded.fit_row,
                fit_cols = excluded.fit_cols, fit_rows = excluded.fit_rows",
            params![id, fit_col, fit_row, fit_cols, fit_rows],
        )?;
        Ok(())
    }

    /// True when the shortcuts tables already hold rows (migration guard).
    pub fn has_shortcuts(&self) -> rusqlite::Result<bool> {
        let n: i64 = self.conn.query_row("SELECT COUNT(*) FROM app_shortcuts", [], |r| r.get(0))?;
        Ok(n > 0)
    }

    /// One-time migration of the legacy `settings.json.shortcuts` list into the
    /// DB (free cells are assigned left-to-right, top-to-bottom skipping the
    /// reserved hero area). No-op once the DB has rows.
    pub fn migrate_shortcuts_from_settings(&self, legacy: &[crate::settings::Shortcut]) -> rusqlite::Result<()> {
        if self.has_shortcuts()? || legacy.is_empty() {
            return Ok(());
        }
        let mut col = 0i64;
        let mut row = 4i64; // below the hero reserved zone (rows 0-3)
        for sc in legacy {
            let sc_row = ShortcutRow {
                id: sc.id.clone(),
                name: sc.name.clone(),
                kind: sc.kind.as_str().to_string(),
                target: sc.target.clone(),
                col,
                row,
                fit_col: None,
                fit_row: None,
                fit_cols: None,
                fit_rows: None,
            };
            self.insert_shortcut(&sc_row)?;
            col += 1;
            if col >= 12 {
                col = 0;
                row += 1;
            }
        }
        Ok(())
    }

    // ---- CLI / reporting queries ----

    /// Last 7 days (inclusive of today) focus totals: Vec<(date, total_sec)>.
    pub fn week_focus_summary(&self) -> rusqlite::Result<Vec<(String, i64)>> {
        let mut stmt = self.conn.prepare(
            "SELECT date(ended_at), COALESCE(SUM(duration_sec),0)
             FROM focus_sessions
             WHERE date(ended_at) >= date('now','localtime','-6 days')
             GROUP BY date(ended_at) ORDER BY date(ended_at)",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect()
    }

    /// Most recent completed focus sessions (newest first).
    pub fn recent_sessions(&self, limit: usize) -> rusqlite::Result<Vec<FocusSessionRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, started_at, ended_at, duration_sec, task_id
             FROM focus_sessions ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], |r| {
            Ok(FocusSessionRow {
                id: r.get(0)?,
                started_at: r.get(1)?,
                ended_at: r.get(2)?,
                duration_sec: r.get(3)?,
                task_id: r.get(4)?,
            })
        })?;
        rows.collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ShortcutType;

    static TEST_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    fn temp_store() -> Store {
        let seq = TEST_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "focus-store-test-{}-{}",
            std::process::id(),
            seq
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.db");
        let _ = std::fs::remove_file(&path);
        let s = Store::open(&path).unwrap();
        s.migrate().unwrap();
        s
    }

    #[test]
    fn migrate_is_idempotent() {
        let s = temp_store();
        s.migrate().unwrap();
        let n: i64 = s
            .conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert!(n >= 3, "expected >=3 migrations, got {n}");
    }

    #[test]
    fn shortcut_crud_and_layout() {
        let s = temp_store();
        let row = ShortcutRow {
            id: "a".into(),
            name: "A".into(),
            kind: "file".into(),
            target: "x".into(),
            col: 2,
            row: 3,
            fit_col: None,
            fit_row: None,
            fit_cols: None,
            fit_rows: None,
        };
        s.insert_shortcut(&row).unwrap();
        let list = s.list_shortcuts().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!((list[0].col, list[0].row), (2, 3));
        assert_eq!(list[0].kind, "file");
        s.move_shortcut("a", 5, 6).unwrap();
        assert_eq!((s.list_shortcuts().unwrap()[0].col, s.list_shortcuts().unwrap()[0].row), (5, 6));
        s.set_shortcut_fit("a", 1, 1, 4, 3).unwrap();
        let l = s.list_shortcuts().unwrap();
        assert_eq!((l[0].fit_col, l[0].fit_row, l[0].fit_cols, l[0].fit_rows), (Some(1), Some(1), Some(4), Some(3)));
        s.delete_shortcut("a").unwrap();
        assert!(s.list_shortcuts().unwrap().is_empty());
    }

    #[test]
    fn migrate_legacy_shortcuts_only_once() {
        let s = temp_store();
        let legacy = vec![crate::settings::Shortcut {
            id: "l1".into(),
            name: "L1".into(),
            kind: ShortcutType::File,
            target: "t".into(),
            order: 0,
        }];
        s.migrate_shortcuts_from_settings(&legacy).unwrap();
        assert_eq!(s.list_shortcuts().unwrap().len(), 1);
        s.migrate_shortcuts_from_settings(&legacy).unwrap();
        assert_eq!(s.list_shortcuts().unwrap().len(), 1);
    }

    #[test]
    fn week_and_sessions_queries() {
        let s = temp_store();
        let now = chrono::Local::now().to_rfc3339();
        s.record_focus_session(&now, &now, 1500, None).unwrap();
        let week = s.week_focus_summary().unwrap();
        assert!(!week.is_empty());
        let total: i64 = week.iter().map(|(_, sec)| sec).sum();
        assert_eq!(total, 1500);
        let sessions = s.recent_sessions(5).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].duration_sec, 1500);
    }

    #[test]
    fn agent_cli_audit_row_written() {
        let s = temp_store();
        let id = s
            .record_agent_cli_call("th-1", "timer status", true, "{\"pong\":true}")
            .unwrap();
        assert!(id > 0);
        let row: (String, String, i64, Option<String>) = s
            .conn
            .query_row(
                "SELECT rule, app, level, payload FROM supervision_events WHERE id = ?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(row.0, "agent_cli_call");
        assert_eq!(row.1, "th-1");
        assert_eq!(row.2, 0);
        assert!(row.3.as_deref().unwrap_or("").contains("timer status"));
    }
}
