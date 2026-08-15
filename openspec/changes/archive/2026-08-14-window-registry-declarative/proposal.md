## Why

Adding a window today touches five scattered places (`create_windows`, `FLOAT_LABELS`, collapse/grid placement, `App.vue` switch, capabilities) and the Rust entry file is a 4800-line `lib.rs`. The user's restructure round (requirement #125, docs/next-phase.md "扩展方向 1") asks for a declarative window registry so a new window becomes "declaration + component" instead of editing existing creation logic. This is the natural first step for splitting `lib.rs` by domain.

## What Changes

- Introduce a static `WINDOW_SPECS` table (label / kind / title / default grid rect / builder flags) as the single source of truth for window creation; `create_windows` becomes a spec-driven loop with the same creation order and the exact same per-window flags as today.
- Remove the `FLOAT_LABELS` constant; the float set and `is_float_label` derive from the registry. `apply_initial_layout` reads the default rect from the spec (dropping the workflow special case).
- The setup glass loop derives from the spec (`setup_acrylic` flag); pet keeps its existing derived-tint acrylic path.
- Frontend: a `ViewRegistry` (label → component / transparent / tray entry) replaces the `App.vue` switch and the hard-coded view-tray buttons in `DesktopView.vue`; tray items derive from float views.
- Remove the never-created `"logos"` entry from `capabilities/default.json` (dead config) and add a Rust test asserting the capabilities windows array exactly matches the registry labels.
- **No visible behavior change**: window order, flags, initial visibility, glass, bubble, topbar, and overlay behavior stay bit-for-bit identical.

## Capabilities

### New Capabilities

- `window-registry`: all WebView windows are declared in one static table; float lifecycle, glass setup, initial layout, and the frontend view map derive from that declaration, and the Tauri capability window list stays consistent with it.

### Modified Capabilities

- None.

## Impact

Affects `apps/desktop/src-tauri/src/lib.rs` (create_windows, apply_initial_layout, setup glass loop, tests), a new `window_spec.rs` module, `capabilities/default.json`, `apps/desktop/src/App.vue`, `apps/desktop/src/views/desktop/DesktopView.vue`, and a new frontend `view-registry.ts` plus tests. Desktop lock, providers, workflows, grid semantics, and bubble delivery are unchanged.

Non-goals (per config rules): no visible window behavior change; no change to focus locking, providers, or workflows; no Focus UI Kit (separate future change); no plugin API; no build-time generation of capabilities (a Rust consistency test guards it instead).
