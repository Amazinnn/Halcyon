//! Declarative window registry (ADR-0037, OpenSpec `window-registry`).
//!
//! One `WINDOW_SPECS` table is the single source of truth for window
//! creation. Adding a window = one entry here + one frontend `VIEW_REGISTRY`
//! entry + a capability list sync, guarded by tests. The historical
//! `FLOAT_LABELS` constant is abolished; the float set, initial layout
//! defaults, and setup glass all derive from this table.
//!
//! Values are transcribed one-to-one from the previous hard-coded
//! `create_windows` builders: observable behavior must not change.

use crate::settings::GridRect;

/// The behavioral family a window belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowKind {
    /// Fullscreen desktop shell.
    Desktop,
    /// Grid float: collapse/restore/snap/resize lifecycle.
    Float,
    /// pet-bubble companion host: float-host styling, no grid lifecycle,
    /// mouse click-through.
    Bubble,
    /// Fullscreen drag/resize preview, mouse click-through.
    Overlay,
    /// Fixed-size status capsule, mouse click-through.
    Topbar,
}

/// One declarative window entry.
#[derive(Debug, Clone, Copy)]
pub struct WindowSpec {
    pub label: &'static str,
    pub title: &'static str,
    pub kind: WindowKind,
    /// Grid rect used by `apply_initial_layout` when no saved layout exists
    /// (Float only; differs from `birth_rect` for workflow, keep both).
    pub default_rect: Option<GridRect>,
    /// Grid rect used by `create_windows` when no saved layout exists
    /// (Float only).
    pub birth_rect: Option<GridRect>,
    /// Fixed initial size for non-grid windows (Bubble/Topbar).
    pub fixed_size: Option<(f64, f64)>,
    pub transparent: bool,
    /// Explicit transparent-black background color (v1.10.3.1 #42/#48,
    /// avoids the white-flash edge).
    pub transparent_background: bool,
    pub always_on_top: bool,
    pub skip_taskbar: bool,
    pub resizable: bool,
    pub fullscreen: bool,
    /// Applied after creation: mouse click-through + no-activate.
    pub ignore_cursor_events: bool,
    /// Configured as a hidden-created float host (native full-client rect
    /// window procedure).
    pub float_host: bool,
    /// Receives the standard glass layer during setup. `pet` is false: its
    /// glass is the derived-tint path (`apply_current_pet_acrylic`).
    pub setup_acrylic: bool,
    /// Born hidden (`visible(false)` in the builder).
    pub hidden_at_start: bool,
}

const fn float_rect(col: usize, row: usize, cols: usize, rows: usize) -> Option<GridRect> {
    Some(GridRect {
        col,
        row,
        cols,
        rows,
    })
}

/// The nine Focus Desktop windows, in creation order.
pub const WINDOW_SPECS: &[WindowSpec] = &[
    WindowSpec {
        label: "desktop",
        title: "Focus Desktop",
        kind: WindowKind::Desktop,
        default_rect: None,
        birth_rect: None,
        fixed_size: None,
        transparent: false,
        transparent_background: false,
        always_on_top: false,
        skip_taskbar: false,
        resizable: true,
        fullscreen: true,
        ignore_cursor_events: false,
        float_host: false,
        setup_acrylic: false,
        hidden_at_start: false,
    },
    WindowSpec {
        label: "chat",
        title: "对话",
        kind: WindowKind::Float,
        default_rect: float_rect(0, 0, 2, 2),
        birth_rect: float_rect(0, 0, 2, 2),
        fixed_size: None,
        transparent: true,
        transparent_background: true,
        always_on_top: true,
        skip_taskbar: true,
        resizable: false,
        fullscreen: false,
        ignore_cursor_events: false,
        float_host: true,
        setup_acrylic: true,
        hidden_at_start: true,
    },
    WindowSpec {
        label: "stats",
        title: "统计",
        kind: WindowKind::Float,
        default_rect: float_rect(0, 0, 2, 2),
        birth_rect: float_rect(0, 0, 2, 2),
        fixed_size: None,
        transparent: true,
        transparent_background: true,
        always_on_top: true,
        skip_taskbar: true,
        resizable: false,
        fullscreen: false,
        ignore_cursor_events: false,
        float_host: true,
        setup_acrylic: true,
        hidden_at_start: true,
    },
    WindowSpec {
        label: "music",
        title: "音乐",
        kind: WindowKind::Float,
        default_rect: float_rect(0, 0, 2, 2),
        birth_rect: float_rect(0, 0, 2, 2),
        fixed_size: None,
        transparent: true,
        transparent_background: true,
        always_on_top: true,
        skip_taskbar: true,
        resizable: false,
        fullscreen: false,
        ignore_cursor_events: false,
        float_host: true,
        setup_acrylic: true,
        hidden_at_start: true,
    },
    WindowSpec {
        label: "pet",
        title: "桌宠",
        kind: WindowKind::Float,
        default_rect: float_rect(0, 0, 2, 2),
        birth_rect: float_rect(0, 0, 2, 2),
        fixed_size: None,
        transparent: true,
        transparent_background: true,
        always_on_top: true,
        skip_taskbar: true,
        resizable: false,
        fullscreen: false,
        ignore_cursor_events: false,
        float_host: true,
        setup_acrylic: false, // pet glass = derived host tint path
        hidden_at_start: true,
    },
    WindowSpec {
        label: "pet-bubble",
        title: "Focus Pet Bubble",
        kind: WindowKind::Bubble,
        default_rect: None,
        birth_rect: None,
        fixed_size: Some((340.0, 120.0)),
        transparent: true,
        transparent_background: true,
        always_on_top: true,
        skip_taskbar: true,
        resizable: false,
        fullscreen: false,
        ignore_cursor_events: true,
        float_host: true,
        setup_acrylic: false,
        hidden_at_start: true,
    },
    WindowSpec {
        label: "workflow",
        title: "工作流",
        kind: WindowKind::Float,
        default_rect: float_rect(4, 2, 4, 4), // historical reconcile default
        birth_rect: float_rect(0, 2, 6, 5),   // v1.10.4 (#51) birth 6x5
        fixed_size: None,
        transparent: true,
        transparent_background: true,
        always_on_top: true,
        skip_taskbar: true,
        resizable: false,
        fullscreen: false,
        ignore_cursor_events: false,
        float_host: true,
        setup_acrylic: true,
        hidden_at_start: true,
    },

    WindowSpec {
        label: "overview",
        title: "概览",
        kind: WindowKind::Float,
        default_rect: float_rect(0, 2, 2, 2),
        birth_rect: float_rect(0, 2, 2, 2),
        fixed_size: None,
        transparent: true,
        transparent_background: true,
        always_on_top: true,
        skip_taskbar: true,
        resizable: false,
        fullscreen: false,
        ignore_cursor_events: false,
        float_host: true,
        setup_acrylic: true,
        hidden_at_start: true,
    },
    WindowSpec {
        label: "grid-overlay",
        title: "Grid Overlay",
        kind: WindowKind::Overlay,
        default_rect: None,
        birth_rect: None,
        fixed_size: None,
        transparent: true,
        transparent_background: false,
        always_on_top: true,
        skip_taskbar: true,
        resizable: true,
        fullscreen: true,
        ignore_cursor_events: true,
        float_host: false,
        setup_acrylic: false,
        hidden_at_start: true,
    },
    WindowSpec {
        label: "topbar",
        title: "状态",
        kind: WindowKind::Topbar,
        default_rect: None,
        birth_rect: None,
        fixed_size: Some((crate::TOPBAR_WINDOW_WIDTH, crate::TOPBAR_WINDOW_HEIGHT)),
        transparent: true,
        transparent_background: false,
        always_on_top: true,
        skip_taskbar: true,
        resizable: true,
        fullscreen: false,
        ignore_cursor_events: true,
        float_host: false,
        setup_acrylic: false,
        hidden_at_start: true,
    },
];

/// Labels of every grid float window, in registry (creation) order.
pub fn float_labels() -> impl Iterator<Item = &'static str> {
    WINDOW_SPECS
        .iter()
        .filter(|s| s.kind == WindowKind::Float)
        .map(|s| s.label)
}

pub fn is_float_label(label: &str) -> bool {
    WINDOW_SPECS
        .iter()
        .any(|s| s.kind == WindowKind::Float && s.label == label)
}

pub fn spec(label: &str) -> Option<&'static WindowSpec> {
    WINDOW_SPECS.iter().find(|s| s.label == label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_set_matches_the_historical_lifecycle() {
        let labels: Vec<&str> = float_labels().collect();
        assert_eq!(labels, ["chat", "stats", "music", "pet", "workflow"]);
        for label in ["desktop", "pet-bubble", "grid-overlay", "topbar"] {
            assert!(!is_float_label(label), "{label} must not be a float");
        }
    }

    #[test]
    fn spec_lookup_and_unique_labels() {
        assert!(spec("chat").is_some());
        assert!(spec("nope").is_none());
        let mut seen = std::collections::HashSet::new();
        for s in WINDOW_SPECS {
            assert!(seen.insert(s.label), "duplicate label {}", s.label);
        }
    }

    #[test]
    fn registry_labels_match_the_capability_window_list() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("capabilities/default.json");
        let raw = std::fs::read_to_string(&path)
            .expect("capabilities/default.json must exist next to the crate");
        let v: serde_json::Value =
            serde_json::from_str(&raw).expect("capabilities/default.json must be valid JSON");
        let windows = v["windows"]
            .as_array()
            .expect("capabilities windows must be an array");
        let mut labels: Vec<&str> = WINDOW_SPECS.iter().map(|s| s.label).collect();
        let mut caps: Vec<&str> = windows.iter().filter_map(|w| w.as_str()).collect();
        labels.sort_unstable();
        caps.sort_unstable();
        assert_eq!(
            labels, caps,
            "WINDOW_SPECS and capabilities/default.json windows must match exactly"
        );
    }
}