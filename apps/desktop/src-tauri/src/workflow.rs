//! M4 workflow engine app layer (ADR-0012): persistence wiring, scheduler,
//! triggers, Tauri commands and the agent/event/window sinks that feed the
//! independent workflow_engine module. The engine itself stays Tauri-free;
//! this module is the only place that touches AppState / windows.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::event_bus::CoreEvent;
use crate::storage::{CharacterRow, Store, WorkflowRunRow};
use crate::workflow_engine::engine::{
    execute_run, AgentCall, EventSink, RunOutcome, SystemActions, WindowOps,
};
use crate::workflow_engine::model::{
    guard_matches, next_daily_run, next_interval_run, next_weekly_run, now_ts, validate_workflow,
    CharacterInfo, RunStatus, WorkflowDef,
};
use crate::AppState;

/// Default wait timeout for a workflow agent node (10 minutes).
pub const AGENT_TIMEOUT_SEC: u64 = 600;

/// Scheduler heartbeat.
pub const SCHEDULER_TICK_SEC: u64 = 15;

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn claim_workflow_run(running: &Mutex<HashSet<String>>, workflow_id: &str) -> bool {
    running.lock().unwrap().insert(workflow_id.to_string())
}

fn initial_provider_for_pet(pet_pack_id: &str, display_name: &str) -> &'static str {
    if pet_pack_id == "focus-demo-pet" && display_name == "Focus Demo Pet" {
        "claude"
    } else {
        "codex"
    }
}

/// Short unique id (millis + counter) for workflows/runs/characters.
pub fn new_id() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let c = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{t}-{c}")
}

/// Compute the next scheduled run timestamp, or None when not schedulable.
pub fn compute_next_run(wf: &WorkflowDef) -> Option<i64> {
    if !wf.enabled || wf.trigger != "scheduled" {
        return None;
    }
    let now = now_ts();
    match wf.schedule_type.as_deref() {
        Some("interval") => wf.interval_minutes.map(|m| next_interval_run(now, m)),
        Some("daily") => wf
            .daily_time
            .as_deref()
            .and_then(|t| next_daily_run(now, t).ok()),
        Some("weekly") => wf
            .weekly_day
            .zip(wf.weekly_time.as_deref())
            .and_then(|(day, time)| next_weekly_run(now, day.into(), time).ok()),
        _ => None,
    }
}

pub struct WorkflowManager {
    pub app: AppHandle,
    pub store: Arc<Mutex<Store>>,
    running: Mutex<HashSet<String>>,
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl WorkflowManager {
    pub fn new(app: AppHandle, store: Arc<Mutex<Store>>) -> Self {
        Self {
            app,
            store,
            running: Mutex::new(HashSet::new()),
            cancels: Mutex::new(HashMap::new()),
        }
    }

    // ---- characters ----

    /// One character per imported pet pack (lazy creation, ADR-0012); falls
    /// back to a default "Focus 助手" character when no packs exist.
    pub fn ensure_characters(&self) -> Vec<CharacterRow> {
        let data_dir = self.app.state::<AppState>().data_dir.clone();
        let packs = crate::pets::list(&data_dir).unwrap_or_default();
        let mut out = Vec::new();
        {
            // v1.10.5.1 (#66): never silently return an empty character
            // list on a poisoned lock — recover the guard so defaults are ensured.
            let store = self.store.lock().unwrap_or_else(|e| e.into_inner());
            for p in &packs {
                let existed = store
                    .list_characters()
                    .unwrap_or_default()
                    .iter()
                    .any(|character| character.pet_pack_id.as_deref() == Some(p.id.as_str()));
                if let Ok(id) = store.ensure_character(&p.id, &p.display_name) {
                    if !existed {
                        let _ = store.update_character_tool(
                            &id,
                            initial_provider_for_pet(&p.id, &p.display_name),
                        );
                    }
                    if let Ok(Some(c)) = store.get_character(&id) {
                        out.push(c);
                    }
                }
            }
            if out.is_empty() {
                if let Ok(Some(c)) = store.get_character("char-default") {
                    out.push(c);
                } else {
                    let row = CharacterRow {
                        id: "char-default".into(),
                        name: "Focus 助手".into(),
                        persona: "你是 Focus 桌面助手。请用简洁中文回答，围绕帮助用户保持专注、整理任务与状态自检。".into(),
                        pet_pack_id: None,
                        // M5 (ADR-0022): default Agent = codex carrier.
                        tool: "codex".into(),
                        workspace_dir: None,
                        current_session_hash: None,
                        session_date: None,
                    };
                    let _ = store.insert_character(&row);
                    out.push(row);
                }
            }
        }
        out
    }

    pub fn list_characters(&self) -> Vec<CharacterRow> {
        self.ensure_characters()
    }

    // ---- workflows ----

    /// v1.10.5 (ADR-0018, #62): delete workflows that fail structural
    /// validation (removed node kinds such as `if`/`bubble`). No backward
    /// compatibility — no placeholders, no migration.
    pub fn purge_incompatible(&self) {
        let Ok(store) = self.store.lock() else { return };
        let Ok(all) = store.list_workflows() else {
            return;
        };
        for w in all {
            if let Err(e) = validate_workflow(&w) {
                eprintln!("[workflow] purge incompatible {} ({}): {e}", w.id, w.name);
                let _ = store.delete_workflow(&w.id);
            }
        }
    }

    /// v1.11 (ADR-0020): empty character_id lists all workflows (including
    /// unbound ones); otherwise filter by character as before.
    pub fn list_workflows(&self, character_id: &str) -> Vec<WorkflowDef> {
        self.purge_incompatible();
        let Ok(store) = self.store.lock() else {
            return vec![];
        };
        store
            .list_workflows()
            .unwrap_or_default()
            .into_iter()
            .filter(|w| character_id.is_empty() || w.character_id == character_id)
            .collect()
    }

    /// v1.11 (ADR-0020): empty character_id is a legitimate unbound workflow
    /// (mechanical schedule tool) — save it as-is, never rebind.
    pub fn save_workflow(&self, mut wf: WorkflowDef) -> Result<WorkflowDef, String> {
        validate_workflow(&wf)?;
        let created = wf.id.is_empty();
        if wf.id.is_empty() {
            wf.id = new_id();
        }
        wf.next_run_at = compute_next_run(&wf);
        let store = self.store.lock().map_err(|_| "store 锁异常".to_string())?;
        store.save_workflow(&wf).map_err(|e| e.to_string())?;
        drop(store);
        self.emit_changed(if created { "created" } else { "updated" }, &wf.id);
        Ok(wf)
    }

    pub fn delete_workflow(&self, id: &str) -> Result<(), String> {
        let store = self.store.lock().map_err(|_| "store 锁异常".to_string())?;
        store.delete_workflow(id).map_err(|e| e.to_string())?;
        drop(store);
        self.emit_changed("deleted", id);
        Ok(())
    }

    pub fn get_workflow(&self, id: &str) -> Result<Option<WorkflowDef>, String> {
        let store = self.store.lock().map_err(|_| "store 锁异常".to_string())?;
        store.get_workflow(id).map_err(|e| e.to_string())
    }

    pub fn list_runs(&self, workflow_id: &str) -> Vec<WorkflowRunRow> {
        let Ok(store) = self.store.lock() else {
            return vec![];
        };
        store
            .list_workflow_runs(workflow_id, 20)
            .unwrap_or_default()
    }

    /// Copy a workflow to another character (auto-rebind = new character_id)
    /// and optionally delete the source ("migrate").
    pub fn copy_workflow(
        &self,
        id: &str,
        target_character_id: &str,
        move_source: bool,
    ) -> Result<WorkflowDef, String> {
        let store = self.store.lock().map_err(|_| "store 锁异常".to_string())?;
        let src = store
            .get_workflow(id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "工作流不存在".to_string())?;
        let mut copy = src.clone();
        copy.id = new_id();
        copy.character_id = target_character_id.to_string();
        copy.name = format!("{}（副本）", src.name);
        copy.next_run_at = compute_next_run(&copy);
        store.save_workflow(&copy).map_err(|e| e.to_string())?;
        if move_source {
            store.delete_workflow(id).map_err(|e| e.to_string())?;
        }
        Ok(copy)
    }

    // ---- runs ----

    /// Run a workflow. pply_guard=true enforces the pre-run guard (timer /
    /// event triggers); manual runs bypass it (ADR-0012).
    pub fn run_workflow(
        &self,
        id: &str,
        triggered_by: &str,
        apply_guard: bool,
    ) -> Result<String, String> {
        let wf = self
            .get_workflow(id)?
            .ok_or_else(|| "工作流不存在".to_string())?;
        if !wf.enabled && triggered_by != "manual" {
            return Err("工作流已停用".into());
        }
        if apply_guard {
            let state = self
                .app
                .state::<AppState>()
                .focus_state
                .lock()
                .unwrap()
                .clone();
            if !guard_matches(&wf.guard, &state) {
                let run_id = new_id();
                let store = self.store.lock().map_err(|_| "store 锁异常".to_string())?;
                store
                    .insert_workflow_run(&run_id, &wf.id, triggered_by)
                    .map_err(|e| e.to_string())?;
                store
                    .finish_workflow_run(&run_id, "skipped", Some("条件守卫未满足"), "[]")
                    .map_err(|e| e.to_string())?;
                drop(store);
                self.emit_run_changed(&wf.id, &run_id, "skipped", Some("条件守卫未满足".into()));
                self.reschedule(&wf);
                return Ok(run_id);
            }
        }
        if !claim_workflow_run(&self.running, &wf.id) {
            return Err("工作流正在运行".into());
        }
        let run_id = new_id();
        let store = self.store.lock().map_err(|_| "store 锁异常".to_string())?;
        store
            .insert_workflow_run(&run_id, &wf.id, triggered_by)
            .map_err(|error| {
                self.running.lock().unwrap().remove(&wf.id);
                error.to_string()
            })?;
        drop(store);
        self.start_claimed_run(wf, triggered_by, run_id.clone())?;
        Ok(run_id)
    }

    pub fn cancel_workflow(&self, id: &str) -> Result<(), String> {
        if let Some(c) = self.cancels.lock().unwrap().get(id) {
            c.store(true, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn cleanup_automation_threads(&self) -> Result<(), String> {
        let store = self.store.lock().map_err(|_| "store 锁异常".to_string())?;
        store.hide_automation_threads().map_err(|e| e.to_string())
    }

    /// Automation thread ids still visible (chat window badge/cleanup).
    pub fn visible_automation_thread_ids(&self) -> HashSet<String> {
        let Ok(store) = self.store.lock() else {
            return HashSet::new();
        };
        store.visible_automation_thread_ids().unwrap_or_default()
    }

    /// Automation thread ids hidden by cleanup (chat list filters them out).
    pub fn hidden_automation_thread_ids(&self) -> HashSet<String> {
        let Ok(store) = self.store.lock() else {
            return HashSet::new();
        };
        store.hidden_automation_thread_ids().unwrap_or_default()
    }

    // ---- scheduler / triggers ----

    pub fn scheduler_tick(&self) {
        let now = now_ts();
        let workflows = {
            let Ok(store) = self.store.lock() else { return };
            store.list_workflows().unwrap_or_default()
        };
        for wf in workflows {
            if !wf.enabled || wf.trigger != "scheduled" {
                continue;
            }
            let Some(nx) = wf.next_run_at else { continue };
            if nx > now {
                continue;
            }
            let _ = self.run_workflow(&wf.id, "schedule", true);
        }
    }

    fn trigger_workflows(&self, trigger: &str) {
        let workflows = {
            let Ok(store) = self.store.lock() else { return };
            store.list_workflows().unwrap_or_default()
        };
        for wf in workflows {
            if wf.enabled && wf.trigger == trigger {
                let _ = self.run_workflow(&wf.id, trigger, true);
            }
        }
    }

    /// Route core bus events into workflow triggers (ADR-0012: only focus end
    /// and supervision alerts; agent replies never trigger workflows).
    pub fn on_core_event(&self, event: &CoreEvent) {
        match event {
            CoreEvent::FocusStateChanged { state, completed } => {
                if state == "rest" && *completed {
                    self.trigger_workflows("focus_end");
                }
            }
            CoreEvent::SupervisionAlert { .. } => {
                self.trigger_workflows("supervision_alert");
            }
            _ => {}
        }
    }

    fn emit_run_changed(
        &self,
        workflow_id: &str,
        run_id: &str,
        status: &str,
        error: Option<String>,
    ) {
        let _ = self
            .app
            .state::<AppState>()
            .events_tx
            .send(CoreEvent::WorkflowRunChanged {
                workflow_id: workflow_id.to_string(),
                run_id: run_id.to_string(),
                status: status.to_string(),
                error,
            });
    }

    /// v1.11 (ADR-0020): broadcast workflow create/update/delete so frontends
    /// (and later the M5 Agent dashboard) can reload.
    fn emit_changed(&self, action: &str, workflow_id: &str) {
        let _ = self
            .app
            .state::<AppState>()
            .events_tx
            .send(CoreEvent::WorkflowChanged {
                action: action.to_string(),
                workflow_id: workflow_id.to_string(),
            });
    }

    /// Recompute + persist next_run_at (used after a guard skip or run).
    fn reschedule(&self, wf: &WorkflowDef) {
        let mut wf = wf.clone();
        wf.next_run_at = compute_next_run(&wf);
        if let Ok(store) = self.store.lock() {
            let _ = store.save_workflow(&wf);
        }
    }

    fn start_claimed_run(
        &self,
        wf: WorkflowDef,
        triggered_by: &str,
        run_id: String,
    ) -> Result<(), String> {
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancels
            .lock()
            .unwrap()
            .insert(wf.id.clone(), cancel.clone());
        let store = self.store.clone();
        let app = self.app.clone();
        let trigger = triggered_by.to_string();
        std::thread::spawn(move || {
            let manager = app.state::<AppState>().workflow.lock().unwrap().clone();
            let character = store
                .lock()
                .ok()
                .and_then(|s| s.get_character(&wf.character_id).ok().flatten())
                .map(|c: CharacterRow| CharacterInfo {
                    id: c.id,
                    name: c.name,
                    persona: c.persona,
                })
                .unwrap_or_else(|| CharacterInfo {
                    id: wf.character_id.clone(),
                    name: wf.name.clone(),
                    persona: String::new(),
                });
            let outcome = if let Some(m) = &manager {
                execute_run(
                    &wf,
                    &character,
                    m.as_ref() as &dyn AgentCall,
                    m.as_ref() as &dyn EventSink,
                    m.as_ref() as &dyn WindowOps,
                    m.as_ref() as &dyn SystemActions,
                    &cancel,
                )
            } else {
                RunOutcome {
                    status: RunStatus::Failed,
                    error: Some("工作流引擎未初始化".into()),
                    node_log: vec![],
                }
            };
            let log_json = serde_json::to_string(&outcome.node_log).unwrap_or_else(|_| "[]".into());
            if let Ok(s) = store.lock() {
                let _ = s.finish_workflow_run(
                    &run_id,
                    outcome.status.as_str(),
                    outcome.error.as_deref(),
                    &log_json,
                );
            }
            if let Some(m) = &manager {
                m.emit_run_changed(
                    &wf.id,
                    &run_id,
                    outcome.status.as_str(),
                    outcome.error.clone(),
                );
                if wf.trigger == "scheduled" {
                    m.reschedule(&wf);
                }
                m.running.lock().unwrap().remove(&wf.id);
                m.cancels.lock().unwrap().remove(&wf.id);
            }
            let _ = trigger;
        });
        Ok(())
    }
}

/// Default workspace when a character has none yet (shouldn't happen — the
/// lazy workspace creation runs before this; defensive fallback).
fn user_home_default() -> String {
    std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string())
}

/// Grab the initialized workflow manager from app state.
pub fn manager(app: &AppHandle) -> Result<Arc<WorkflowManager>, String> {
    app.state::<AppState>()
        .workflow
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "工作流引擎未初始化".to_string())
}

// ---------------------------------------------------------------------------
// Sinks (injected into the Tauri-free engine)
// ---------------------------------------------------------------------------

impl AgentCall for WorkflowManager {
    fn resolve_character(&self, id: &str) -> Option<CharacterInfo> {
        let store = self.store.lock().ok()?;
        store
            .get_character(id)
            .ok()
            .flatten()
            .map(|c: CharacterRow| CharacterInfo {
                id: c.id,
                name: c.name,
                persona: c.persona,
            })
    }

    fn call_one_shot(
        &self,
        character: &CharacterInfo,
        prompt: &str,
        wait: bool,
        cancel: &AtomicBool,
        display: crate::workflow_engine::engine::AgentDisplay,
    ) -> Result<Option<(String, String)>, String> {
        // M5 (ADR-0022): output discipline is injected system-wide by the
        // provider (agents::OUTPUT_DISCIPLINE); the AGENTS.md identity lives
        // in the workspace — persona is no longer injected here.
        let full = prompt.to_string();
        let (info, mut rx) = crate::with_agent_for(&self.app, &character.id, |runtime| {
            // M5 (ADR-0022): per-character Agent runtime (lazy-built; creates
            // workspace + AGENTS.md on first use). The workspace for this call
            // is the character's own Focus-Agents/<id>/ dir.
            let rx = runtime
                .subscribe_turn_done()
                .ok_or_else(|| "Agent 运行时未就绪".to_string())?;
            let workspace = {
                let store = self.store.lock().unwrap();
                store
                    .get_character(&character.id)
                    .ok()
                    .flatten()
                    .and_then(|c| c.workspace_dir)
                    .unwrap_or_else(user_home_default)
            };
            let info = runtime.start_thread(&workspace, &full, display)?;
            if let Ok(s) = self.store.lock() {
                let _ = s.record_automation_thread(&info.id, &character.id, None);
            }
            Ok((info, rx))
        })?;
        if !wait {
            return Ok(None);
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(AGENT_TIMEOUT_SEC);
        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = crate::with_agent_for(&self.app, &character.id, |a| a.interrupt(&info.id));
                return Err("已取消".into());
            }
            if std::time::Instant::now() >= deadline {
                let _ = crate::with_agent_for(&self.app, &character.id, |a| a.interrupt(&info.id));
                return Err("Agent 等待超时（已中断）".into());
            }
            match rx.try_recv() {
                Ok(td) => {
                    let matches = td
                        .thread_id
                        .as_deref()
                        .map(|t| t == info.id)
                        .unwrap_or(true);
                    if !matches {
                        continue;
                    }
                    return match td.status.as_str() {
                        "completed" => Ok(Some((td.result.unwrap_or_default(), info.id))),
                        "interrupted" => Err("Agent 已中断".into()),
                        other => Err(format!("Agent 执行失败: {other}")),
                    };
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                    return Err("Agent 通道已断开".into());
                }
            }
        }
    }
}

impl EventSink for WorkflowManager {
    fn bubble(&self, text: &str, priority: &str, agent_id: Option<&str>) {
        let _ = self
            .app
            .state::<AppState>()
            .events_tx
            .send(CoreEvent::BubbleRequested {
                text: text.to_string(),
                priority: priority.to_string(),
                agent_id: agent_id.map(str::to_owned),
            });
    }

    fn agent_result(&self, workflow_id: &str, workflow_name: &str, agent_id: &str, text: &str) {
        let _ = self
            .app
            .state::<AppState>()
            .events_tx
            .send(CoreEvent::WorkflowAgentResult {
                workflow_id: workflow_id.to_string(),
                workflow_name: workflow_name.to_string(),
                agent_id: agent_id.to_string(),
                text: text.to_string(),
            });
    }
}

impl WindowOps for WorkflowManager {
    fn show_window(&self, label: &str) -> Result<(), String> {
        crate::restore_window(&self.app, label)
    }
}

impl SystemActions for WorkflowManager {
    fn focus(&self, seconds: i64) -> Result<(), String> {
        let _ = self
            .app
            .state::<AppState>()
            .events_tx
            .send(CoreEvent::WorkflowSystemAction {
                action: "focus".into(),
                seconds,
            });
        Ok(())
    }
    fn idle(&self, seconds: i64) -> Result<(), String> {
        let _ = self
            .app
            .state::<AppState>()
            .events_tx
            .send(CoreEvent::WorkflowSystemAction {
                action: "idle".into(),
                seconds,
            });
        Ok(())
    }
    fn ring(&self, seconds: i64) -> Result<(), String> {
        let _ = self
            .app
            .state::<AppState>()
            .events_tx
            .send(CoreEvent::WorkflowSystemAction {
                action: "ring".into(),
                seconds,
            });
        Ok(())
    }
    fn focus_state(&self) -> String {
        self.app
            .state::<AppState>()
            .focus_state
            .lock()
            .unwrap()
            .clone()
    }
    fn now_hhmm(&self) -> String {
        chrono::Local::now().format("%H:%M").to_string()
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn characters_list(app: tauri::AppHandle) -> Vec<CharacterRow> {
    manager(&app)
        .map(|m| m.list_characters())
        .unwrap_or_default()
}

#[tauri::command]
pub fn workflow_list(app: tauri::AppHandle, character_id: String) -> Vec<WorkflowDef> {
    manager(&app)
        .map(|m| m.list_workflows(&character_id))
        .unwrap_or_default()
}

#[tauri::command]
pub fn workflow_save(app: tauri::AppHandle, workflow: WorkflowDef) -> Result<WorkflowDef, String> {
    manager(&app)?.save_workflow(workflow)
}

#[tauri::command]
pub fn workflow_delete(app: tauri::AppHandle, id: String) -> Result<(), String> {
    manager(&app)?.delete_workflow(&id)
}

#[tauri::command]
pub fn workflow_run(app: tauri::AppHandle, id: String) -> Result<String, String> {
    manager(&app)?.run_workflow(&id, "manual", false)
}

#[tauri::command]
pub fn workflow_cancel(app: tauri::AppHandle, id: String) -> Result<(), String> {
    manager(&app)?.cancel_workflow(&id)
}

#[tauri::command]
pub fn workflow_copy(
    app: tauri::AppHandle,
    id: String,
    target_character_id: String,
    move_source: bool,
) -> Result<WorkflowDef, String> {
    manager(&app)?.copy_workflow(&id, &target_character_id, move_source)
}

#[tauri::command]
pub fn workflow_runs(app: tauri::AppHandle, id: String) -> Vec<WorkflowRunRow> {
    manager(&app).map(|m| m.list_runs(&id)).unwrap_or_default()
}

#[tauri::command]
pub fn workflow_cleanup_threads(app: tauri::AppHandle) -> Result<(), String> {
    manager(&app)?.cleanup_automation_threads()
}

#[tauri::command]
pub fn workflow_automation_threads(app: tauri::AppHandle) -> Vec<String> {
    manager(&app)
        .map(|m| m.visible_automation_thread_ids().into_iter().collect())
        .unwrap_or_default()
}

/// v1.10.4 (#51): recent run row with workflow display name (settings page).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentRunRow {
    pub id: String,
    pub workflow_id: String,
    pub workflow_name: String,
    pub triggered_by: String,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub status: String,
    pub error: Option<String>,
    pub node_log: String,
}

#[tauri::command]
pub fn workflow_runs_recent(app: tauri::AppHandle, limit: i64) -> Vec<RecentRunRow> {
    let Ok(m) = manager(&app) else { return vec![] };
    let Ok(store) = m.store.lock() else {
        return vec![];
    };
    let Ok(runs) = store.list_recent_workflow_runs(limit) else {
        return vec![];
    };
    let mut names: HashMap<String, String> = HashMap::new();
    if let Ok(all) = store.list_workflows() {
        for w in all {
            names.insert(w.id.clone(), w.name);
        }
    }
    runs.into_iter()
        .map(|r| RecentRunRow {
            workflow_name: names
                .get(&r.workflow_id)
                .cloned()
                .unwrap_or_else(|| r.workflow_id.clone()),
            id: r.id,
            workflow_id: r.workflow_id,
            triggered_by: r.triggered_by,
            started_at: r.started_at,
            finished_at: r.finished_at,
            status: r.status,
            error: r.error,
            node_log: r.node_log,
        })
        .collect()
}

#[tauri::command]
pub fn workflow_runs_clear(app: tauri::AppHandle) -> Result<(), String> {
    let m = manager(&app)?;
    let store = m.store.lock().map_err(|_| "store 锁异常".to_string())?;
    store.clear_workflow_runs().map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Focus-cli integration (local control plane; NOT in the agent whitelist)
// ---------------------------------------------------------------------------

/// Handle ocus-cli workflow ... from cli.rs. Returns a JSON value.
pub fn cli_handle(
    app: &AppHandle,
    parts: &[&str],
    payload: Option<&serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let m = manager(app)?;
    match parts {
        // v1.11 (ADR-0020): list all workflows incl. unbound (empty
        // character_id); the Agent-facing view of the schedule board.
        ["workflow", "list"] => {
            let workflows = m.list_workflows("");
            let chars: Vec<CharacterRow> = m.list_characters();
            let name_of = |id: &str| -> String {
                if id.is_empty() {
                    return String::new();
                }
                chars
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.name.clone())
                    .unwrap_or_default()
            };
            let workflows: Vec<serde_json::Value> = workflows
                .into_iter()
                .map(|w| {
                    serde_json::json!({
                        "id": w.id,
                        "characterId": w.character_id,
                        "character": name_of(&w.character_id),
                        "name": w.name,
                        "trigger": w.trigger,
                        "guard": w.guard,
                        "enabled": w.enabled,
                        "nextRunAt": w.next_run_at,
                    })
                })
                .collect();
            Ok(serde_json::json!({ "workflows": workflows }))
        }
        ["workflow", "run", id] => {
            let run_id = m.run_workflow(id, "manual", false)?;
            Ok(serde_json::json!({ "runId": run_id, "status": "started" }))
        }
        ["workflow", "runs", id] => {
            let runs: Vec<serde_json::Value> = m
                .list_runs(id)
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "id": r.id,
                        "triggeredBy": r.triggered_by,
                        "startedAt": r.started_at,
                        "finishedAt": r.finished_at,
                        "status": r.status,
                        "error": r.error,
                        "nodeLog": r.node_log,
                    })
                })
                .collect();
            Ok(serde_json::json!({ "runs": runs }))
        }
        ["workflow", "cancel", id] => {
            m.cancel_workflow(id)?;
            Ok(serde_json::json!({ "cancelled": true }))
        }
        // v1.11 (ADR-0020): Agent-facing JSON document channel. The workflow
        // JSON is the single source of truth; the canvas renders it.
        ["workflow", "read", id] => {
            let wf = m
                .get_workflow(id)?
                .ok_or_else(|| "工作流不存在".to_string())?;
            serde_json::to_value(&wf).map_err(|e| e.to_string())
        }
        ["workflow", "create"] => {
            let wf = workflow_from_payload(payload)?;
            let saved = m.save_workflow(wf)?;
            serde_json::to_value(&saved).map_err(|e| e.to_string())
        }
        ["workflow", "update", id] => {
            let mut wf = workflow_from_payload(payload)?;
            // The id comes from the URL, not the body (idempotent updates).
            wf.id = id.to_string();
            let saved = m.save_workflow(wf)?;
            serde_json::to_value(&saved).map_err(|e| e.to_string())
        }
        ["workflow", "delete", id] => {
            m.delete_workflow(id)?;
            Ok(serde_json::json!({ "deleted": true, "id": id }))
        }
        _ => Err(format!("未知 workflow 子命令: {}", parts.join(" "))),
    }
}

/// v1.11 (ADR-0020): parse the create/update body as a WorkflowDef JSON.
fn workflow_from_payload(payload: Option<&serde_json::Value>) -> Result<WorkflowDef, String> {
    let v = payload.ok_or_else(|| "缺少 --payload（工作流 JSON）".to_string())?;
    serde_json::from_value(v.clone()).map_err(|e| format!("工作流 JSON 无效: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow_engine::model::{EdgeDef, NodeDef, WorkflowDef};

    fn wf(trigger: &str) -> WorkflowDef {
        WorkflowDef {
            id: "w".into(),
            character_id: "c".into(),
            name: "t".into(),
            trigger: trigger.into(),
            schedule_type: None,
            interval_minutes: None,
            daily_time: None,
            weekly_day: None,
            weekly_time: None,
            guard: "none".into(),
            nodes: vec![NodeDef {
                id: "n1".into(),
                kind: "bubble".into(),
                params: serde_json::json!({"text":"hi"}),
                x: 0.0,
                y: 0.0,
            }],
            edges: vec![EdgeDef {
                id: "e1".into(),
                source: "n1".into(),
                source_handle: "out".into(),
                target: "n1".into(),
            }],
            enabled: true,
            next_run_at: None,
        }
    }

    #[test]
    fn workflow_from_payload_parses_camel_case_json() {
        // v1.11 (ADR-0020): Agent-facing JSON documents use the same camelCase
        // wire shape as the canvas editor.
        let v: serde_json::Value = serde_json::json!({
            "id": "",
            "characterId": "",
            "name": "简报",
            "trigger": "scheduled",
            "scheduleType": "daily",
            "dailyTime": "09:00",
            "guard": "none",
            "enabled": true,
            "nodes": [
                { "id": "n1", "kind": "agent", "params": { "prompt": "写简报", "characterId": "char-a" }, "x": 0, "y": 0 }
            ],
            "edges": []
        });
        let wf = workflow_from_payload(Some(&v)).unwrap();
        assert_eq!(wf.name, "简报");
        assert_eq!(wf.trigger, "scheduled");
        assert_eq!(wf.nodes[0].params["characterId"], "char-a");
        assert_eq!(wf.character_id, "");
    }

    #[test]
    fn workflow_from_payload_rejects_missing_or_invalid() {
        assert!(workflow_from_payload(None).is_err());
        assert!(workflow_from_payload(Some(&serde_json::json!({"nodes": 1}))).is_err());
    }

    #[test]
    fn focus_demo_pet_alone_gets_claude_as_its_initial_provider() {
        assert_eq!(
            initial_provider_for_pet("focus-demo-pet", "Focus Demo Pet"),
            "claude"
        );
        assert_eq!(initial_provider_for_pet("other-pet", "Other Pet"), "codex");
        assert_eq!(
            initial_provider_for_pet("focus-demo-pet", "Renamed Pet"),
            "codex"
        );
        assert_eq!(initial_provider_for_pet("", "Focus 助手"), "codex");
    }

    #[test]
    fn compute_next_run_manual_or_disabled_is_none() {
        let mut w = wf("manual");
        assert_eq!(compute_next_run(&w), None);
        w.trigger = "scheduled".into();
        w.schedule_type = Some("interval".into());
        w.interval_minutes = Some(30);
        w.enabled = false;
        assert_eq!(compute_next_run(&w), None);
    }

    #[test]
    fn compute_next_run_interval_advances_minutes() {
        let mut w = wf("scheduled");
        w.schedule_type = Some("interval".into());
        w.interval_minutes = Some(30);
        let now = now_ts();
        let nx = compute_next_run(&w).unwrap();
        assert!(nx >= now + 30 * 60 - 2 && nx <= now + 30 * 60 + 2);
    }

    #[test]
    fn compute_next_run_daily_is_in_future() {
        let mut w = wf("scheduled");
        w.schedule_type = Some("daily".into());
        w.daily_time = Some("23:59".into());
        let now = now_ts();
        let nx = compute_next_run(&w).unwrap();
        assert!(nx > now);
    }

    #[test]
    fn compute_next_run_weekly_is_in_future() {
        let mut w = wf("scheduled");
        w.schedule_type = Some("weekly".into());
        w.weekly_day = Some(0);
        w.weekly_time = Some("00:00".into());
        assert!(compute_next_run(&w).is_some());
    }

    #[test]
    fn workflow_run_claim_prevents_scheduler_reentry_until_released() {
        let running = Mutex::new(HashSet::new());
        assert!(claim_workflow_run(&running, "weekly"));
        assert!(!claim_workflow_run(&running, "weekly"));
        running.lock().unwrap().remove("weekly");
        assert!(claim_workflow_run(&running, "weekly"));
    }
}
