//! M4 workflow engine app layer (ADR-0012): persistence wiring, scheduler,
//! triggers, Tauri commands and the agent/event/window sinks that feed the
//! independent workflow_engine module. The engine itself stays Tauri-free;
//! this module is the only place that touches AppState / windows.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::event_bus::CoreEvent;
use crate::storage::{CharacterRow, Store, WorkflowRunRow};
use crate::workflow_engine::engine::{execute_run, AgentCall, EventSink, RunOutcome, SystemActions, WindowOps};
use crate::workflow_engine::model::{
    CharacterInfo, RunStatus, WorkflowDef, guard_matches, next_daily_run, next_interval_run,
    now_ts, validate_workflow,
};
use crate::AppState;

/// Default wait timeout for a workflow agent node (10 minutes).
pub const AGENT_TIMEOUT_SEC: u64 = 600;
/// Scheduler heartbeat.
pub const SCHEDULER_TICK_SEC: u64 = 15;

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

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
            let Ok(store) = self.store.lock() else { return out };
            for p in &packs {
                if let Ok(id) = store.ensure_character(&p.id, &p.display_name) {
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

    pub fn list_workflows(&self, character_id: &str) -> Vec<WorkflowDef> {
        let Ok(store) = self.store.lock() else { return vec![] };
        store
            .list_workflows()
            .unwrap_or_default()
            .into_iter()
            .filter(|w| w.character_id == character_id)
            .collect()
    }

    pub fn save_workflow(&self, mut wf: WorkflowDef) -> Result<WorkflowDef, String> {
        validate_workflow(&wf)?;
        if wf.id.is_empty() {
            wf.id = new_id();
        }
        wf.next_run_at = compute_next_run(&wf);
        let store = self.store.lock().map_err(|_| "store 锁异常".to_string())?;
        store.save_workflow(&wf).map_err(|e| e.to_string())?;
        Ok(wf)
    }

    pub fn delete_workflow(&self, id: &str) -> Result<(), String> {
        let store = self.store.lock().map_err(|_| "store 锁异常".to_string())?;
        store.delete_workflow(id).map_err(|e| e.to_string())
    }

    pub fn get_workflow(&self, id: &str) -> Result<Option<WorkflowDef>, String> {
        let store = self.store.lock().map_err(|_| "store 锁异常".to_string())?;
        store.get_workflow(id).map_err(|e| e.to_string())
    }

    pub fn list_runs(&self, workflow_id: &str) -> Vec<WorkflowRunRow> {
        let Ok(store) = self.store.lock() else { return vec![] };
        store.list_workflow_runs(workflow_id, 20).unwrap_or_default()
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
        let run_id = new_id();
        let store = self.store.lock().map_err(|_| "store 锁异常".to_string())?;
        if apply_guard {
            let state = self.app.state::<AppState>().focus_state.lock().unwrap().clone();
            if !guard_matches(&wf.guard, &state) {
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
        store
            .insert_workflow_run(&run_id, &wf.id, triggered_by)
            .map_err(|e| e.to_string())?;
        drop(store);
        self.start_run(wf, triggered_by, run_id.clone())?;
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
        let Ok(store) = self.store.lock() else { return HashSet::new() };
        store.visible_automation_thread_ids().unwrap_or_default()
    }

    /// Automation thread ids hidden by cleanup (chat list filters them out).
    pub fn hidden_automation_thread_ids(&self) -> HashSet<String> {
        let Ok(store) = self.store.lock() else { return HashSet::new() };
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

    fn emit_run_changed(&self, workflow_id: &str, run_id: &str, status: &str, error: Option<String>) {
        let _ = self.app.state::<AppState>().events_tx.send(CoreEvent::WorkflowRunChanged {
            workflow_id: workflow_id.to_string(),
            run_id: run_id.to_string(),
            status: status.to_string(),
            error,
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

    fn start_run(&self, wf: WorkflowDef, triggered_by: &str, run_id: String) -> Result<(), String> {
        {
            let mut running = self.running.lock().unwrap();
            if running.contains(&wf.id) {
                return Ok(()); // anti-reentry
            }
            running.insert(wf.id.clone());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancels.lock().unwrap().insert(wf.id.clone(), cancel.clone());
        let store = self.store.clone();
        let app = self.app.clone();
        let trigger = triggered_by.to_string();
        std::thread::spawn(move || {
            let manager = app
                .state::<AppState>()
                .workflow
                .lock()
                .unwrap()
                .clone();
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
                m.emit_run_changed(&wf.id, &run_id, outcome.status.as_str(), outcome.error.clone());
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
    fn call_one_shot(
        &self,
        character: &CharacterInfo,
        prompt: &str,
        wait: bool,
        cancel: &AtomicBool,
    ) -> Result<Option<(String, String)>, String> {
        let app_state = self.app.state::<AppState>();
        let workspace = app_state
            .settings
            .lock()
            .unwrap()
            .agent_workspace_dir
            .clone()
            .unwrap_or_else(|| {
                std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string())
            });
        let full = if character.persona.trim().is_empty() {
            prompt.to_string()
        } else {
            format!("{}\n\n{}", character.persona.trim(), prompt)
        };
        let (info, mut rx) = {
            let agent = app_state.agent.lock().unwrap();
            let rx = agent
                .subscribe_turn_done()
                .ok_or_else(|| "Agent 运行时未就绪".to_string())?;
            let info = agent.start_thread(&workspace, &full)?;
            if let Ok(s) = self.store.lock() {
                let _ = s.record_automation_thread(&info.id, &character.id, None);
            }
            (info, rx)
        };
        if !wait {
            return Ok(None);
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(AGENT_TIMEOUT_SEC);
        loop {
            if cancel.load(Ordering::Relaxed) {
                let _ = self.app.state::<AppState>().agent.lock().unwrap().interrupt(&info.id);
                return Err("已取消".into());
            }
            if std::time::Instant::now() >= deadline {
                let _ = self.app.state::<AppState>().agent.lock().unwrap().interrupt(&info.id);
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
    fn bubble(&self, text: &str, priority: &str) {
        let _ = self
            .app
            .state::<AppState>()
            .events_tx
            .send(CoreEvent::BubbleRequested {
                text: text.to_string(),
                priority: priority.to_string(),
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
        let _ = self.app.state::<AppState>().events_tx.send(CoreEvent::WorkflowSystemAction {
            action: "focus".into(),
            seconds,
        });
        Ok(())
    }
    fn idle(&self, seconds: i64) -> Result<(), String> {
        let _ = self.app.state::<AppState>().events_tx.send(CoreEvent::WorkflowSystemAction {
            action: "idle".into(),
            seconds,
        });
        Ok(())
    }
    fn ring(&self, seconds: i64) -> Result<(), String> {
        let _ = self.app.state::<AppState>().events_tx.send(CoreEvent::WorkflowSystemAction {
            action: "ring".into(),
            seconds,
        });
        Ok(())
    }
    fn focus_state(&self) -> String {
        self.app.state::<AppState>().focus_state.lock().unwrap().clone()
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
    manager(&app).map(|m| m.list_characters()).unwrap_or_default()
}

#[tauri::command]
pub fn workflow_list(app: tauri::AppHandle, character_id: String) -> Vec<WorkflowDef> {
    manager(&app)
        .map(|m| m.list_workflows(&character_id))
        .unwrap_or_default()
}

#[tauri::command]
pub fn workflow_save(
    app: tauri::AppHandle,
    workflow: WorkflowDef,
) -> Result<WorkflowDef, String> {
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

// ---------------------------------------------------------------------------
// Focus-cli integration (local control plane; NOT in the agent whitelist)
// ---------------------------------------------------------------------------

/// Handle ocus-cli workflow ... from cli.rs. Returns a JSON value.
pub fn cli_handle(app: &AppHandle, parts: &[&str]) -> Result<serde_json::Value, String> {
    let m = manager(app)?;
    match parts {
        ["workflow", "list"] => {
            let chars = m.list_characters();
            let mut workflows = Vec::new();
            for c in &chars {
                for w in m.list_workflows(&c.id) {
                    workflows.push(serde_json::json!({
                        "id": w.id,
                        "characterId": w.character_id,
                        "character": c.name,
                        "name": w.name,
                        "trigger": w.trigger,
                        "guard": w.guard,
                        "enabled": w.enabled,
                        "nextRunAt": w.next_run_at,
                    }));
                }
            }
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
        _ => Err(format!("未知 workflow 子命令: {}", parts.join(" "))),
    }
}