//! Minimal SQLite spike storage: schema_migrations + spike_probes.
//! Full product schema (design doc §15) is intentionally deferred.

use rusqlite::{params, Connection};
use rusqlite::OptionalExtension;
use crate::workflow_engine::model::WorkflowDef;
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
/// One day of the focus heatmap (calendar date -> focus minutes).
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapDay {
    pub date: String,
    pub minutes: i64,
}

/// Today's focus totals.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodaySummary {
    pub total_sec: i64,
    pub rounds: i64,
}

/// Full stats-window payload (v1.8). `distraction`/`idle`/`genres` have no
/// data source yet and are intentionally `null` (UI shows "暂无数据").
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardPayload {
    pub today: TodaySummary,
    pub heatmap30: Vec<HeatmapDay>,
    pub hours24: Vec<i64>,
    pub streak_days: i64,
    pub distraction: Option<()>,
    pub idle: Option<()>,
    pub genres: Option<()>,
}

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        // v1.8.1: wait up to 5s for a concurrent writer instead of failing
        // immediately with SQLITE_BUSY (previously dropped inserts when two
        // app instances shared the same DB).
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
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

        // 0005: M4 workflow engine (ADR-0012) — characters / workflows / runs
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS characters (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                persona TEXT NOT NULL DEFAULT '',
                pet_pack_id TEXT,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                character_id TEXT NOT NULL,
                name TEXT NOT NULL,
                trigger TEXT NOT NULL DEFAULT 'manual',
                schedule_type TEXT,
                interval_minutes INTEGER,
                daily_time TEXT,
                guard TEXT NOT NULL DEFAULT 'none',
                nodes_json TEXT NOT NULL DEFAULT '[]',
                edges_json TEXT NOT NULL DEFAULT '[]',
                enabled INTEGER NOT NULL DEFAULT 1,
                next_run_at INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS workflow_runs (
                id TEXT PRIMARY KEY,
                workflow_id TEXT NOT NULL,
                triggered_by TEXT NOT NULL,
                started_at INTEGER NOT NULL,
                finished_at INTEGER,
                status TEXT NOT NULL DEFAULT 'running',
                error TEXT,
                node_log TEXT NOT NULL DEFAULT '[]'
            );
            CREATE TABLE IF NOT EXISTS automation_threads (
                thread_id TEXT PRIMARY KEY,
                character_id TEXT NOT NULL,
                workflow_id TEXT,
                created_at INTEGER NOT NULL,
                hidden INTEGER NOT NULL DEFAULT 0
            );
            INSERT OR IGNORE INTO schema_migrations (name, applied_at)
                VALUES ('0005_m4_workflow_engine', datetime('now'));
            ",
        )?;

        // 0006 (v1.10, #30): internal-page cards are removed — internal pages
        // open from the leftmost views tray only. One-time cleanup.
        let has0006: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE name = '0006_remove_internal_shortcuts')",
            [],
            |r| r.get(0),
        )?;
        if !has0006 {
            self.conn
                .execute("DELETE FROM app_shortcuts WHERE type = 'internal'", [])?;
            self.conn.execute(
                "INSERT OR IGNORE INTO schema_migrations (name, applied_at)
                 VALUES ('0006_remove_internal_shortcuts', datetime('now'))",
                [],
            )?;
        }
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
             FROM focus_sessions WHERE date(ended_at, 'localtime') = date('now','localtime')",
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
            "SELECT date(ended_at, 'localtime'), COALESCE(SUM(duration_sec),0)
             FROM focus_sessions
             WHERE date(ended_at, 'localtime') >= date('now','localtime','-6 days')
             GROUP BY date(ended_at, 'localtime') ORDER BY date(ended_at, 'localtime')",
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


    // ---- v1.8 stats dashboard (real data) ----

    /// Focus minutes per calendar day for the last `days` days (inclusive of
    /// today), zero-filled so the frontend always gets a complete sequence.
    /// Attribution follows the local `ended_at` date (same as today/week).
    pub fn heatmap_days(&self, days: u32) -> rusqlite::Result<Vec<HeatmapDay>> {
        let mut stmt = self.conn.prepare(
            "SELECT date(ended_at, 'localtime'), COALESCE(SUM(duration_sec),0)
             FROM focus_sessions
             WHERE date(ended_at, 'localtime') >= date('now','localtime', ?1)
             GROUP BY date(ended_at, 'localtime')",
        )?;
        let days_back = days.saturating_sub(1) as i64;
        let cutoff = format!("-{days_back} days");
        let rows = stmt.query_map([&cutoff], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        let mut minutes_by_date: std::collections::HashMap<String, i64> =
            std::collections::HashMap::new();
        for row in rows {
            let (date, sec) = row?;
            minutes_by_date.insert(date, sec / 60);
        }
        let today = chrono::Local::now().date_naive();
        let mut out = Vec::with_capacity(days as usize);
        for i in (0..days).rev() {
            let date = (today - chrono::Duration::days(i as i64))
                .format("%Y-%m-%d")
                .to_string();
            out.push(HeatmapDay {
                date: date.clone(),
                minutes: minutes_by_date.get(&date).copied().unwrap_or(0),
            });
        }
        Ok(out)
    }

    /// Today's focus minutes bucketed by hour (index 0..=23), attributed to
    /// the local `ended_at` hour.
    pub fn hours24_today(&self) -> rusqlite::Result<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT CAST(strftime('%H', ended_at, 'localtime') AS INTEGER), COALESCE(SUM(duration_sec),0)
             FROM focus_sessions
             WHERE date(ended_at, 'localtime') = date('now','localtime')
             GROUP BY strftime('%H', ended_at, 'localtime')",
        )?;
        let mut hours = vec![0i64; 24];
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (h, sec) = row?;
            if (0..24).contains(&h) {
                hours[h as usize] = sec / 60;
            }
        }
        Ok(hours)
    }

    /// Consecutive-day streak ending today: a day counts when it has at least
    /// one completed session of >= 1 minute; a day without one (starting at
    /// today) breaks the streak.
    pub fn streak_days(&self) -> rusqlite::Result<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT date(ended_at, 'localtime') FROM focus_sessions
             WHERE date(ended_at, 'localtime') <= date('now','localtime') AND duration_sec >= 60
             ORDER BY date(ended_at, 'localtime') DESC",
        )?;
        let dates: Vec<String> = stmt
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        let today = chrono::Local::now().date_naive();
        let mut streak = 0i64;
        let mut expect = today;
        for d in dates {
            let Ok(parsed) = chrono::NaiveDate::parse_from_str(&d, "%Y-%m-%d") else {
                continue;
            };
            if parsed == expect {
                streak += 1;
                expect -= chrono::Duration::days(1);
            } else if parsed < expect {
                break;
            }
        }
        Ok(streak)
    }

    /// One payload for the stats window / `focus-cli stats dashboard`.
    pub fn dashboard(&self) -> rusqlite::Result<DashboardPayload> {
        let (total_sec, rounds) = self.today_focus_summary()?;
        Ok(DashboardPayload {
            today: TodaySummary { total_sec, rounds },
            heatmap30: self.heatmap_days(30)?,
            hours24: self.hours24_today()?,
            streak_days: self.streak_days()?,
            distraction: None,
            idle: None,
            genres: None,
        })
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

// ---- M4 workflow engine (ADR-0012) ----

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterRow {
    pub id: String,
    pub name: String,
    pub persona: String,
    pub pet_pack_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRunRow {
    pub id: String,
    pub workflow_id: String,
    pub triggered_by: String,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub status: String,
    pub error: Option<String>,
    pub node_log: String,
}

impl Store {
    // ---- characters ----

    pub fn list_characters(&self) -> rusqlite::Result<Vec<CharacterRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, persona, pet_pack_id FROM characters ORDER BY created_at, id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(CharacterRow {
                id: r.get(0)?,
                name: r.get(1)?,
                persona: r.get(2)?,
                pet_pack_id: r.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn get_character(&self, id: &str) -> rusqlite::Result<Option<CharacterRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, persona, pet_pack_id FROM characters WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |r| {
            Ok(CharacterRow {
                id: r.get(0)?,
                name: r.get(1)?,
                persona: r.get(2)?,
                pet_pack_id: r.get(3)?,
            })
        })?;
        rows.next().transpose()
    }

    pub fn insert_character(&self, row: &CharacterRow) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO characters (id, name, persona, pet_pack_id, created_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now','localtime'))",
            params![row.id, row.name, row.persona, row.pet_pack_id],
        )?;
        Ok(())
    }

    /// Lazy-create a character for a pet pack; returns the existing id when
    /// the pack already has one (ADR-0012: one character per imported pet).
    pub fn ensure_character(&self, pet_pack_id: &str, name: &str) -> rusqlite::Result<String> {
        let existing: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM characters WHERE pet_pack_id = ?1",
                params![pet_pack_id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            return Ok(id);
        }
        let id = format!("char-{}", crate::workflow::new_id());
        let persona = format!("你是「{name}」的桌面角色助手（Focus 桌宠人格）。请用简洁中文回答，围绕帮助用户保持专注、整理任务与状态自检。");
        self.insert_character(&CharacterRow {
            id: id.clone(),
            name: name.to_string(),
            persona,
            pet_pack_id: Some(pet_pack_id.to_string()),
        })?;
        Ok(id)
    }

    // ---- workflows ----

    pub fn list_workflows(&self) -> rusqlite::Result<Vec<WorkflowDef>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, character_id, name, trigger, schedule_type, interval_minutes,
                    daily_time, guard, nodes_json, edges_json, enabled, next_run_at
             FROM workflows ORDER BY created_at, id",
        )?;
        let rows = stmt.query_map([], |r| row_to_workflow(r))?;
        rows.collect()
    }

    pub fn get_workflow(&self, id: &str) -> rusqlite::Result<Option<WorkflowDef>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, character_id, name, trigger, schedule_type, interval_minutes,
                    daily_time, guard, nodes_json, edges_json, enabled, next_run_at
             FROM workflows WHERE id = ?1",
        )?;
        let mut rows = stmt.query_map(params![id], |r| row_to_workflow(r))?;
        rows.next().transpose()
    }

    pub fn save_workflow(&self, wf: &WorkflowDef) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO workflows (id, character_id, name, trigger, schedule_type,
                                    interval_minutes, daily_time, guard, nodes_json,
                                    edges_json, enabled, next_run_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, datetime('now','localtime'), datetime('now','localtime'))
             ON CONFLICT(id) DO UPDATE SET
               character_id=excluded.character_id, name=excluded.name, trigger=excluded.trigger,
               schedule_type=excluded.schedule_type, interval_minutes=excluded.interval_minutes,
               daily_time=excluded.daily_time, guard=excluded.guard, nodes_json=excluded.nodes_json,
               edges_json=excluded.edges_json, enabled=excluded.enabled, next_run_at=excluded.next_run_at,
               updated_at=datetime('now','localtime')",
            params![
                wf.id,
                wf.character_id,
                wf.name,
                wf.trigger,
                wf.schedule_type,
                wf.interval_minutes,
                wf.daily_time,
                wf.guard,
                serde_json::to_string(&wf.nodes).unwrap_or_else(|_| "[]".into()),
                serde_json::to_string(&wf.edges).unwrap_or_else(|_| "[]".into()),
                if wf.enabled { 1 } else { 0 },
                wf.next_run_at,
            ],
        )?;
        Ok(())
    }

    pub fn delete_workflow(&self, id: &str) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM workflows WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// v1.10.5.1 (#66): one-time data recovery - rebind workflows whose
    /// character_id is empty or points to a missing character to default_id.
    /// Returns the number of affected rows. Not a compatibility layer (#62).
    pub fn rebind_orphan_workflows(&self, default_id: &str) -> rusqlite::Result<usize> {
        let mut stmt = self.conn.prepare(
            "UPDATE workflows SET character_id = ?1, updated_at = datetime('now','localtime')
             WHERE character_id = '' OR character_id IS NULL
                OR NOT EXISTS (SELECT 1 FROM characters WHERE characters.id = workflows.character_id)",
        )?;
        stmt.execute([default_id])
    }
    // ---- workflow runs ----

    pub fn insert_workflow_run(
        &self,
        id: &str,
        workflow_id: &str,
        triggered_by: &str,
    ) -> rusqlite::Result<()> {
        let now = crate::workflow_engine::model::now_ts();
        self.conn.execute(
            "INSERT INTO workflow_runs (id, workflow_id, triggered_by, started_at, status)
             VALUES (?1, ?2, ?3, ?4, 'running')",
            params![id, workflow_id, triggered_by, now],
        )?;
        Ok(())
    }

    pub fn finish_workflow_run(
        &self,
        id: &str,
        status: &str,
        error: Option<&str>,
        node_log: &str,
    ) -> rusqlite::Result<()> {
        let now = crate::workflow_engine::model::now_ts();
        self.conn.execute(
            "UPDATE workflow_runs SET finished_at = ?2, status = ?3, error = ?4, node_log = ?5
             WHERE id = ?1",
            params![id, now, status, error, node_log],
        )?;
        Ok(())
    }

    /// v1.10.4 (#51): most recent runs across all workflows (settings page).
    pub fn list_recent_workflow_runs(&self, limit: i64) -> rusqlite::Result<Vec<WorkflowRunRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, workflow_id, triggered_by, started_at, finished_at, status, error, node_log
             FROM workflow_runs ORDER BY started_at DESC, rowid DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit], |r| {
            Ok(WorkflowRunRow {
                id: r.get(0)?,
                workflow_id: r.get(1)?,
                triggered_by: r.get(2)?,
                started_at: r.get(3)?,
                finished_at: r.get(4)?,
                status: r.get(5)?,
                error: r.get(6)?,
                node_log: r.get(7)?,
            })
        })?;
        rows.collect()
    }

    /// v1.10.4 (#51): clear all workflow run history.
    pub fn clear_workflow_runs(&self) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM workflow_runs", [])?;
        Ok(())
    }

    pub fn list_workflow_runs(&self, workflow_id: &str, limit: i64) -> rusqlite::Result<Vec<WorkflowRunRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, workflow_id, triggered_by, started_at, finished_at, status, error, node_log
             FROM workflow_runs WHERE workflow_id = ?1 ORDER BY started_at DESC, rowid DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![workflow_id, limit], |r| {
            Ok(WorkflowRunRow {
                id: r.get(0)?,
                workflow_id: r.get(1)?,
                triggered_by: r.get(2)?,
                started_at: r.get(3)?,
                finished_at: r.get(4)?,
                status: r.get(5)?,
                error: r.get(6)?,
                node_log: r.get(7)?,
            })
        })?;
        rows.collect()
    }

    // ---- automation threads ----

    pub fn record_automation_thread(
        &self,
        thread_id: &str,
        character_id: &str,
        workflow_id: Option<&str>,
    ) -> rusqlite::Result<()> {
        let now = crate::workflow_engine::model::now_ts();
        self.conn.execute(
            "INSERT OR IGNORE INTO automation_threads (thread_id, character_id, workflow_id, created_at, hidden)
             VALUES (?1, ?2, ?3, ?4, 0)",
            params![thread_id, character_id, workflow_id, now],
        )?;
        Ok(())
    }

    /// Thread ids marked as automation and not hidden (used to annotate the
    /// chat thread list; hidden threads are filtered out).
    pub fn visible_automation_thread_ids(&self) -> rusqlite::Result<std::collections::HashSet<String>> {
        let mut stmt = self.conn.prepare("SELECT thread_id FROM automation_threads WHERE hidden = 0")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect()
    }

    pub fn hide_automation_threads(&self) -> rusqlite::Result<()> {
        self.conn.execute("UPDATE automation_threads SET hidden = 1 WHERE hidden = 0", [])?;
        Ok(())
    }

    /// Thread ids hidden by "cleanup" (kept for the chat list filter).
    pub fn hidden_automation_thread_ids(&self) -> rusqlite::Result<std::collections::HashSet<String>> {
        let mut stmt = self.conn.prepare("SELECT thread_id FROM automation_threads WHERE hidden = 1")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect()
    }
}

fn row_to_workflow(r: &rusqlite::Row<'_>) -> rusqlite::Result<WorkflowDef> {
    let nodes_json: String = r.get(8)?;
    let edges_json: String = r.get(9)?;
    let enabled: i64 = r.get(10)?;
    Ok(WorkflowDef {
        id: r.get(0)?,
        character_id: r.get(1)?,
        name: r.get(2)?,
        trigger: r.get(3)?,
        schedule_type: r.get(4)?,
        interval_minutes: r.get(5)?,
        daily_time: r.get(6)?,
        guard: r.get(7)?,
        nodes: serde_json::from_str(&nodes_json).unwrap_or_default(),
        edges: serde_json::from_str(&edges_json).unwrap_or_default(),
        enabled: enabled != 0,
        next_run_at: r.get(11)?,
    })
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
    fn migration_0006_removes_internal_shortcuts_once() {
        let s = temp_store();
        // Simulate a pre-0006 database: temp_store already applied 0006 as a
        // no-op, so drop the marker to re-test the one-time cleanup itself.
        s.conn
            .execute("DELETE FROM schema_migrations WHERE name = '0006_remove_internal_shortcuts'", [])
            .unwrap();
        s.insert_shortcut(&ShortcutRow {
            id: "a1".into(),
            name: "应用".into(),
            kind: "application".into(),
            target: "C:/x.exe".into(),
            col: 0,
            row: 0,
            fit_col: None,
            fit_row: None,
            fit_cols: None,
            fit_rows: None,
        })
        .unwrap();
        s.insert_shortcut(&ShortcutRow {
            id: "i1".into(),
            name: "音乐".into(),
            kind: "internal".into(),
            target: "music".into(),
            col: 1,
            row: 0,
            fit_col: None,
            fit_row: None,
            fit_cols: None,
            fit_rows: None,
        })
        .unwrap();
        // run full migration twice: internal removed once, app card kept
        s.migrate().unwrap();
        let rows = s.list_shortcuts().unwrap();
        assert_eq!(rows.len(), 1, "internal card must be removed by 0006");
        assert_eq!(rows[0].kind, "application");
        s.migrate().unwrap();
        let rows2 = s.list_shortcuts().unwrap();
        assert_eq!(rows2.len(), 1);
        let has0006: bool = s
            .conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE name='0006_remove_internal_shortcuts')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(has0006);
    }

    #[test]
    fn workflow_crud_runs_and_automation_threads() {
        use crate::workflow_engine::model::{EdgeDef, NodeDef, WorkflowDef};
        let s = temp_store();
        let cid = s.ensure_character("pet-1", "测试宠").unwrap();
        let c = s.get_character(&cid).unwrap().unwrap();
        assert_eq!(c.pet_pack_id.as_deref(), Some("pet-1"));
        assert!(c.persona.contains("测试宠"));
        // ensure is idempotent
        let cid2 = s.ensure_character("pet-1", "测试宠").unwrap();
        assert_eq!(cid, cid2);
        assert_eq!(s.list_characters().unwrap().len(), 1);

        let wf = WorkflowDef {
            id: "wf1".into(),
            character_id: cid.clone(),
            name: "测试".into(),
            trigger: "manual".into(),
            schedule_type: None,
            interval_minutes: None,
            daily_time: None,
            guard: "none".into(),
            nodes: vec![
                NodeDef { id: "n1".into(), kind: "bubble".into(), params: serde_json::json!({"text":"hi"}), x: 0.0, y: 0.0 },
            ],
            edges: vec![EdgeDef { id: "e1".into(), source: "n1".into(), source_handle: "out".into(), target: "n1".into() }],
            enabled: true,
            next_run_at: None,
        };
        s.save_workflow(&wf).unwrap();
        let list = s.list_workflows().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].nodes.len(), 1);
        let got = s.get_workflow("wf1").unwrap().unwrap();
        assert_eq!(got.name, "测试");

        s.insert_workflow_run("r1", "wf1", "manual").unwrap();
        s.finish_workflow_run("r1", "success", None, "[]").unwrap();
        let runs = s.list_workflow_runs("wf1", 10).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].status, "success");
        assert!(runs[0].finished_at.is_some());

        s.record_automation_thread("th-1", &cid, Some("wf1")).unwrap();
        s.record_automation_thread("th-1", &cid, Some("wf1")).unwrap();
        assert!(s.visible_automation_thread_ids().unwrap().contains("th-1"));
        s.hide_automation_threads().unwrap();
        assert!(!s.visible_automation_thread_ids().unwrap().contains("th-1"));

        s.delete_workflow("wf1").unwrap();
        assert!(s.get_workflow("wf1").unwrap().is_none());
    }

    #[test]
    fn recent_runs_ordered_and_clear() {
        use crate::workflow_engine::model::{EdgeDef, NodeDef, WorkflowDef};
        let s = temp_store();
        let cid = s.ensure_character("pet-r", "测试宠").unwrap();
        let wf = WorkflowDef {
            id: "wf-r".into(),
            character_id: cid.clone(),
            name: "定时自检".into(),
            trigger: "scheduled".into(),
            schedule_type: Some("interval".into()),
            interval_minutes: Some(30),
            daily_time: None,
            guard: "focusing".into(),
            nodes: vec![NodeDef {
                id: "n1".into(),
                kind: "bubble".into(),
                params: serde_json::json!({"text":"hi"}),
                x: 0.0,
                y: 0.0,
            }],
            edges: vec![],
            enabled: true,
            next_run_at: None,
        };
        s.save_workflow(&wf).unwrap();
        for i in 0..3 {
            s.insert_workflow_run(&format!("r{i}"), "wf-r", "schedule").unwrap();
            s.finish_workflow_run(&format!("r{i}"), "success", None, "[]").unwrap();
        }
        let recent = s.list_recent_workflow_runs(2).unwrap();
        assert_eq!(recent.len(), 2);
        // newest first
        assert_eq!(recent[0].id, "r2");
        assert_eq!(recent[1].id, "r1");
        s.clear_workflow_runs().unwrap();
        assert!(s.list_recent_workflow_runs(10).unwrap().is_empty());
        assert!(s.list_workflow_runs("wf-r", 10).unwrap().is_empty());
    }

    #[test]
    fn rebind_orphan_workflows_rebinds_empty_and_missing() {
        use crate::workflow_engine::model::{NodeDef, WorkflowDef};
        let s = temp_store();
        let cid = s.ensure_character("pet-1", "test-pet").unwrap();
        let mk = |id: &str, c: &str| WorkflowDef {
            id: id.into(),
            character_id: c.into(),
            name: "t".into(),
            trigger: "manual".into(),
            schedule_type: None,
            interval_minutes: None,
            daily_time: None,
            guard: "none".into(),
            nodes: [NodeDef {
                id: "n1".into(),
                kind: "wait".into(),
                params: serde_json::from_str(r#"{"seconds":1}"#).unwrap(),
                x: 0.0,
                y: 0.0,
            }]
            .to_vec(),
            edges: Vec::new(),
            enabled: true,
            next_run_at: None,
        };
        s.save_workflow(&mk("w-empty", "")).unwrap();
        s.save_workflow(&mk("w-missing", "no-such-char")).unwrap();
        s.save_workflow(&mk("w-ok", &cid)).unwrap();
        let n = s.rebind_orphan_workflows(&cid).unwrap();
        if n != 2 {
            std::process::exit(101);
        }
        if s.get_workflow("w-empty").unwrap().unwrap().character_id != cid {
            std::process::exit(102);
        }
        if s.get_workflow("w-missing").unwrap().unwrap().character_id != cid {
            std::process::exit(103);
        }
        if s.get_workflow("w-ok").unwrap().unwrap().character_id != cid {
            std::process::exit(104);
        }
        if s.rebind_orphan_workflows(&cid).unwrap() != 0 {
            std::process::exit(105);
        }
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

    #[test]
    fn dashboard_aggregates_real_sessions() {
        let s = temp_store();
        let today = chrono::Local::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);
        let off = chrono::Local::now().format("%:z").to_string();
        let fmt = |d: chrono::NaiveDate, h: u32| {
            format!("{}T{:02}:00:00{}", d.format("%Y-%m-%d"), h, off)
        };
        s.record_focus_session(&fmt(today, 10), &fmt(today, 10), 1500, None)
            .unwrap();
        s.record_focus_session(&fmt(today, 14), &fmt(today, 14), 300, None)
            .unwrap();
        s.record_focus_session(&fmt(yesterday, 9), &fmt(yesterday, 9), 1800, None)
            .unwrap();

        let d = s.dashboard().unwrap();
        assert_eq!(d.today.total_sec, 1800);
        assert_eq!(d.today.rounds, 2);
        assert_eq!(d.hours24.len(), 24);
        assert_eq!(d.hours24[10], 25);
        assert_eq!(d.hours24[14], 5);
        assert_eq!(d.heatmap30.len(), 30);
        let today_str = today.format("%Y-%m-%d").to_string();
        let yesterday_str = yesterday.format("%Y-%m-%d").to_string();
        assert_eq!(d.heatmap30[29].date, today_str);
        assert_eq!(d.heatmap30[29].minutes, 30);
        assert_eq!(d.heatmap30[28].date, yesterday_str);
        assert_eq!(d.heatmap30[28].minutes, 30);
        assert_eq!(d.streak_days, 2);
        assert!(d.distraction.is_none());
        assert!(d.idle.is_none());
        assert!(d.genres.is_none());
    }

    #[test]
    fn streak_breaks_when_today_missing() {
        let s = temp_store();
        let today = chrono::Local::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);
        let day_before = today - chrono::Duration::days(2);
        let off = chrono::Local::now().format("%:z").to_string();
        let fmt = |d: chrono::NaiveDate| format!("{}T09:00:00{}", d.format("%Y-%m-%d"), off);
        s.record_focus_session(&fmt(yesterday), &fmt(yesterday), 1500, None)
            .unwrap();
        s.record_focus_session(&fmt(day_before), &fmt(day_before), 1500, None)
            .unwrap();
        // today has no session -> the streak ends at today
        assert_eq!(s.streak_days().unwrap(), 0);
    }

    #[test]
    fn streak_ignores_subminute_sessions() {
        let s = temp_store();
        let today = chrono::Local::now().date_naive();
        let off = chrono::Local::now().format("%:z").to_string();
        let fmt = |d: chrono::NaiveDate| format!("{}T09:00:00{}", d.format("%Y-%m-%d"), off);
        s.record_focus_session(&fmt(today), &fmt(today), 30, None).unwrap();
        assert_eq!(s.streak_days().unwrap(), 0);
    }
}
