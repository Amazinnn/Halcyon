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
    Url,
    Internal,
}

impl ShortcutType {
    /// Stable lowercase string used as the DB `type` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            ShortcutType::File => "file",
            ShortcutType::Folder => "folder",
            ShortcutType::Application => "application",
            ShortcutType::Url => "url",
            ShortcutType::Internal => "internal",
        }
    }

    pub fn parse(s: &str) -> Option<ShortcutType> {
        match s {
            "file" => Some(ShortcutType::File),
            "folder" => Some(ShortcutType::Folder),
            "application" => Some(ShortcutType::Application),
            "url" => Some(ShortcutType::Url),
            "internal" => Some(ShortcutType::Internal),
            _ => None,
        }
    }
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
    #[serde(default = "default_focus_mode")]
    pub focus_mode: String,
    #[serde(default = "default_agent_provider")]
    pub agent_provider: String,
    #[serde(default)]
    pub agent_workspace_dir: Option<String>,
    #[serde(default)]
    pub pet_pack_id: Option<String>,
    #[serde(default = "default_true")]
    pub pet_bg_fade: bool,
    #[serde(default)]
    pub music_folder: Option<String>,
    /// v1.10.2 (#36/#38/#41): bumped after the one-time layout migration.
    #[serde(default)]
    pub layout_version: Option<u32>,
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

fn default_focus_mode() -> String {
    "standard".into()
}

fn default_true() -> bool {
    true
}

fn default_agent_provider() -> String {
    "codex".into()
}

fn default_subtitle() -> String {
    "保持节奏，阳光会照到每一片叶子".into()
}

impl Default for Settings {
    fn default() -> Self {
        let mut grid = HashMap::new();
        grid.insert("chat".into(), GridRect { col: 8, row: 0, cols: 4, rows: 4 });
        grid.insert("stats".into(), GridRect { col: 8, row: 4, cols: 5, rows: 4 });
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
            focus_mode: "standard".into(),
            agent_provider: "codex".into(),
            agent_workspace_dir: None,
        pet_pack_id: None,
        pet_bg_fade: true,
        music_folder: None,
        layout_version: None,
        }
    }
}

impl Settings {
    pub fn load(dir: &Path) -> Self {
        let path = dir.join(SETTINGS_FILE);
        let mut s = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
            Err(_) => Settings::default(),
        };
        if s.migrate_layout() {
            // persist immediately so the migrated layout survives restarts
            let _ = s.save(dir);
        }
        s
    }

    /// v1.10.2 (#36/#38/#41) + v1.10.4 (#51): one-time layout migrations for
    /// windows whose default size changed. Runs once per version and is
    /// idempotent; user customizations that do not match the old defaults are
    /// left untouched.
    /// Returns true when a migration was applied (caller persists it).
    pub fn migrate_layout(&mut self) -> bool {
        let v = self.layout_version.unwrap_or(0);
        let mut changed = false;
        if v < 1 {
            if let Some(r) = self.grid.get_mut("workflow") {
                if r.cols == 2 && r.rows == 2 {
                    r.cols = 4;
                    r.rows = 3;
                }
            }
            if let Some(r) = self.grid.get_mut("music") {
                if r.cols == 3 && r.rows == 2 {
                    r.cols = 3;
                    r.rows = 3;
                }
            }
            if let Some(r) = self.grid.get_mut("stats") {
                if r.cols == 4 && r.rows == 3 {
                    r.cols = 5;
                    r.rows = 4;
                }
            }
            self.layout_version = Some(1);
            changed = true;
        }
        if v < 2 {
            if let Some(r) = self.grid.get_mut("workflow") {
                if r.cols == 4 && r.rows == 3 {
                    r.cols = 6;
                    r.rows = 5;
                }
            }
            self.layout_version = Some(2);
            changed = true;
        }
        changed
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

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_with(entries: &[(&str, GridRect)]) -> Settings {
        let mut s = Settings::default();
        s.grid.clear();
        for (k, v) in entries {
            s.grid.insert(k.to_string(), *v);
        }
        s
    }

    #[test]
    fn layout_migration_resizes_workflow_music_stats() {
        let mut s = grid_with(&[
            ("workflow", GridRect { col: 1, row: 5, cols: 2, rows: 2 }),
            ("music", GridRect { col: 8, row: 7, cols: 3, rows: 2 }),
            ("stats", GridRect { col: 8, row: 4, cols: 4, rows: 3 }),
        ]);
        s.migrate_layout();
        assert_eq!(s.grid["workflow"].cols, 6);
        assert_eq!(s.grid["workflow"].rows, 5);
        assert_eq!(s.grid["music"].cols, 3);
        assert_eq!(s.grid["music"].rows, 3);
        assert_eq!(s.grid["stats"].cols, 5);
        assert_eq!(s.grid["stats"].rows, 4);
        assert_eq!(s.layout_version, Some(2));
    }

    #[test]
    fn layout_migration_idempotent_and_keeps_later_customization() {
        let mut s = grid_with(&[("workflow", GridRect { col: 1, row: 5, cols: 2, rows: 2 })]);
        s.migrate_layout();
        // user later customizes back to 2x2; migration must not re-run
        s.grid.insert("workflow".into(), GridRect { col: 1, row: 5, cols: 2, rows: 2 });
        s.migrate_layout();
        assert_eq!(s.grid["workflow"].cols, 2);
        assert_eq!(s.layout_version, Some(2));
    }

    #[test]
    fn layout_migration_keeps_custom_sizes() {
        let mut s = grid_with(&[
            ("workflow", GridRect { col: 3, row: 3, cols: 3, rows: 3 }),
            ("music", GridRect { col: 0, row: 0, cols: 3, rows: 4 }),
            ("stats", GridRect { col: 0, row: 0, cols: 5, rows: 3 }),
        ]);
        s.migrate_layout();
        assert_eq!(s.grid["workflow"].cols, 3);
        assert_eq!(s.grid["music"].rows, 4);
        assert_eq!(s.grid["stats"].cols, 5);
        assert_eq!(s.layout_version, Some(2));
    }

    #[test]
    fn fresh_defaults_are_new_sizes() {
        let s = Settings::default();
        assert_eq!(s.grid["stats"].cols, 5);
        assert_eq!(s.grid["stats"].rows, 4);
    }

}
