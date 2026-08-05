//! settings.json: wallpaper path, per-window grid layout, topmost flags,
//! collapsed set and logos docking edge. Written atomically (tmp + rename).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub const SETTINGS_FILE: &str = "settings.json";

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