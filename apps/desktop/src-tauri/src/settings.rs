//! settings.json: wallpaper path, per-window grid layout, topmost flags,
//! collapsed set, logos docking edge, file-shortcut zone and acrylic toggle.
//! Written atomically (tmp + rename).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub estimated_minutes: Option<u32>,
    #[serde(default)]
    pub bound_app: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ShortcutType {
    File,
    Folder,
    Application,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ShortcutType,
    pub target: String,
    pub order: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GridRect {
    pub col: usize,
    pub row: usize,
    pub cols: usize,
    pub rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub wallpaper_path: Option<String>,
    pub grid: HashMap<String, GridRect>,
    pub topmost: HashMap<String, bool>,
    pub collapsed: Vec<String>,
    pub logos_edge: String,
    #[serde(default)]
    pub shortcuts: Vec<Shortcut>,
    #[serde(default = "default_true")]
    pub acrylic_enabled: bool,
    #[serde(default = "default_subtitle")]
    pub focus_subtitle: String,
    #[serde(default)]
    pub tasks: Vec<Task>,
    #[serde(default)]
    pub current_task_id: Option<String>,
    #[serde(default = "default_25")]
    pub focus_minutes: u32,
    #[serde(default = "default_5")]
    pub rest_minutes: u32,
    #[serde(default)]
    pub distraction_apps: Vec<String>,
    #[serde(default)]
    pub allowed_apps: Vec<String>,
    #[serde(default = "default_true")]
    pub supervision_enabled: bool,
    #[serde(default)]
    pub supervision_pause_until: Option<i64>,
    #[serde(default = "default_true")]
    pub sound_enabled: bool,
    #[serde(default = "default_show_topbar")]
    pub show_topbar: String,
}

fn default_25() -> u32 {
    25
}

fn default_5() -> u32 {
    5
}

fn default_show_topbar() -> String {
    "auto".into()
}

fn default_true() -> bool {
    true
}

fn default_subtitle() -> String {
    "保持节奏，阳光会照到每一片叶子".into()
}

impl Default for Settings {
    fn default() -> Self {
        let mut grid = HashMap::new();
        grid.insert("chat".into(), GridRect { col: 8, row: 0, cols: 4, rows: 4 });
        grid.insert("stats".into(), GridRect { col: 8, row: 4, cols: 4, rows: 3 });
        grid.insert("music".into(), GridRect { col: 8, row: 7, cols: 3, rows: 1 });
        grid.insert("pet".into(), GridRect { col: 11, row: 7, cols: 1, rows: 1 });
        let mut topmost = HashMap::new();
        for k in ["chat", "stats", "music", "pet"] {
            topmost.insert(k.to_string(), true);
        }
        Self {
            wallpaper_path: None,
            grid,
            topmost,
            collapsed: Vec::new(),
            logos_edge: "top".into(),
            shortcuts: Vec::new(),
            acrylic_enabled: true,
            focus_subtitle: "保持节奏，阳光会照到每一片叶子".into(),
            tasks: Vec::new(),
            current_task_id: None,
            focus_minutes: 25,
            rest_minutes: 5,
            distraction_apps: Vec::new(),
            allowed_apps: Vec::new(),
            supervision_enabled: true,
            supervision_pause_until: None,
            sound_enabled: true,
            show_topbar: "auto".into(),
        }
    }
}

impl Settings {
    pub fn load(dir: &Path) -> Self {
        let path = dir.join(SETTINGS_FILE);
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Settings::default(),
        }
    }

    pub fn save(&self, dir: &Path) -> Result<(), String> {
        let path = dir.join(SETTINGS_FILE);
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let tmp = dir.join("settings.json.tmp");
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        Ok(())
    }
}