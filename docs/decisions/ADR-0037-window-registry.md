# ADR-0037: Declarative Window Registry (WindowSpec + ViewRegistry)

Status: Accepted, Windows visual acceptance pending
Date: 2026-08-14
Requirements: #125
OpenSpec: `2026-08-14-window-registry-declarative`
Amends: none

## Context

The restructure round (requirement #125) starts with a declarative window
registry because adding a window today requires touching five scattered
places: `create_windows`, `FLOAT_LABELS`, the collapse/grid placement paths,
the `App.vue` label switch, and `capabilities/default.json`. The user's
stated goal is "新增窗口 = 声明 + 拼积木" (new window = declaration plus
assembly) instead of editing existing creation logic. All nine windows are
created dynamically from `create_windows` in lib.rs (4829 lines); `tauri.conf.json`
declares no static windows, so a Rust-side registry fully drives creation.

## Decision

1. **One static `WINDOW_SPECS` table** in a new `window_spec.rs` module is
   the single source of truth for window creation. Each entry declares
   label / title / kind / default grid rect / builder flags
   (transparent, always_on_top, skip_taskbar, resizable, fullscreen,
   ignore_cursor_events, float_host, setup_acrylic, hidden_at_start,
   fixed_size). Values are transcribed one-to-one from the current builders;
   observable behavior does not change.
2. **WindowKind (Desktop / Float / Bubble / Overlay / Topbar)** captures the
   existing behavioral families: Float = grid lifecycle
   (collapse/restore/snap/resize); Bubble = pet-bubble companion host (float
   host styling, no grid lifecycle, click-through); Overlay/Topbar =
   fullscreen preview and fixed-size capsule, click-through.
3. **`FLOAT_LABELS` is abolished.** The float set, `is_float_label`,
   `apply_initial_layout` defaults (dropping the workflow special case), and
   the setup glass loop all derive from the registry.
4. **Frontend `ViewRegistry`** (`src/lib/view-registry.ts`) is the single
   label → component / transparent styling / tray-entry mapping; `App.vue`
   drops its switch (unknown labels fall back to DesktopView) and
   `DesktopView.vue` derives its tray buttons from float views.
5. **Capabilities remain a static Tauri security boundary** (never
   generated), and a Rust test asserts the `windows` array equals the
   registry labels exactly. The never-created `"logos"` entry is removed.
6. **Tray visibility is registry-derived**: every float window appears in the
   desktop view tray; a future window that must not appear in the tray adds
   an explicit flag later.

## Consequences

A new window now means one `WINDOW_SPECS` entry + one `VIEW_REGISTRY` entry +
a capabilities sync enforced by the consistency test (three declarations,
two of them test-guarded). The TS and Rust registries are two compile-time
tables that must be updated together; both sides have tests. No settings key,
DB schema, or saved layout changes (window labels are unchanged), so no
migration is needed. Windows visual acceptance remains the only gate before
the OpenSpec change is archived.
