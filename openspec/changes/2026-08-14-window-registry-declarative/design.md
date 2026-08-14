## Context

See proposal.md - Why. The current implementation hard-codes nine `tauri::WebviewWindowBuilder` calls in `create_windows` (lib.rs 2805-3029) plus a `FLOAT_LABELS: [&str; 5]` constant used by placement (1647), `apply_initial_layout` (3119/3167, including a workflow default-rect special case at 3122), and tests; the setup glass loop hard-codes `["chat","stats","music","workflow"]` (3376). The frontend maps labels in an `App.vue` switch plus a hard-coded transparent-label list and four tray buttons in `DesktopView.vue`. `capabilities/default.json` lists the nine windows plus a never-created `"logos"` entry. `tauri.conf.json` declares no static windows; all windows are created dynamically, so a Rust-side registry fully drives creation.

## Goals / Non-Goals

**Goals:**
- One static `WINDOW_SPECS` table as the only window declaration source; `create_windows`, float enumeration, initial layout defaults, and setup glass all derive from it.
- Frontend `ViewRegistry` as the only label → component/transparent/tray mapping; `App.vue` switch and hard-coded tray buttons disappear.
- A Rust test guards capabilities ↔ registry consistency so a new window cannot silently miss permissions.
- Bit-for-bit identical observable behavior (creation order, flags, visibility, glass, bubble/topbar/overlay).

**Non-Goals:**
- No behavior change, no window kind/feature additions, no Focus UI Kit, no plugin API, no build-time generation of capabilities, no restructuring of other lib.rs domains (follow-up changes), no changes to desktop lock, providers, workflows, or grid semantics.

## Decisions

1. **WindowKind enum (Desktop / Float / Bubble / Overlay / Topbar).** Float = grid lifecycle (collapse/restore/snap/resize); Bubble = pet-bubble companion host (float host styling, no grid lifecycle, click-through); Overlay/Topbar = fullscreen preview and fixed-size capsule, click-through. Rationale: captures the existing behavioral families so the creation loop can branch on kind; alternatives considered — a boolean flag soup (harder to read) or a fully open builder closure per entry (reintroduces scattered logic).

2. **Registry entry carries every builder-relevant flag** (transparent, always_on_top, skip_taskbar, resizable, fullscreen, ignore_cursor_events, float_host, setup_acrylic, hidden_at_start, fixed_size) plus `default_rect` for floats. Values are transcribed one-to-one from the current builders; e.g. pet-bubble fixed_size (340,120), topbar fixed TOPBAR_WINDOW_* constants, desktop fullscreen + visible at start. `setup_acrylic` is false for pet (its glass is the derived-tint `apply_current_pet_acrylic` path, called separately) and pet-bubble (no acrylic). Rationale: explicit per-window flags beat inferring flags from kind, because today's family members are not uniform (pet-bubble is a float host but not a grid float; pet is a float but has its own glass path).

3. **`create_windows` becomes a spec-driven loop preserving today's creation order** (desktop → chat → stats → music → pet → pet-bubble → workflow → grid-overlay → topbar, i.e. registry order). Float branch keeps `initial_float_rect` + `configure_float_host`; Bubble branch adds ignore-cursor/noactivate; Topbar/Overlay branches keep their current special handling; the trailing initial-show loop (non-collapsed floats, pet only when the current Agent has a valid pack) stays as-is. Rationale: order and side effects are observable on Windows (z-order, startup stacking); any reordering is out of scope.

4. **Derived helpers:** `float_labels()`, `is_float_label()`, `spec(label)`. `apply_initial_layout` switches from `FLOAT_LABELS` to `float_labels()` and reads each float's `default_rect` from the spec, removing the workflow special case. The setup glass loop iterates specs with `setup_acrylic`. Tests keep asserting the same float/non-float membership as today.

5. **Capabilities consistency test:** a lib test reads `capabilities/default.json` (cargo test runs with cwd = `src-tauri`; if that assumption ever breaks, switch to `env!("CARGO_MANIFEST_DIR")` + relative join) and asserts its `windows` array equals the registry labels exactly, after removing the dead `"logos"` entry. Rationale: Tauri capabilities are a static security boundary and must stay explicit; a test is lighter than a codegen step, which is deferred to the tier-2 gate tooling.

6. **Frontend ViewRegistry** (`src/lib/view-registry.ts`): `ViewSpec { label, kind, title, icon, component, transparent }`; `viewForLabel()` falls back to DesktopView for unknown labels (matching today's switch default); `floatViews()` returns float-kind entries for the tray; `isTransparentLabel()` replaces the hard-coded list in `App.vue` (result identical: all labels except desktop). DesktopView tray renders `floatViews()` via v-for and calls `openView(v.label)`. Rationale: single place to extend for a new window; the fallback keeps unknown-window startup safe.

7. **Registry name/kind consistency across stacks** is enforced by the frontend tests (unique labels, expected float set) and the Rust capability test; there is no runtime cross-check between TS and Rust registries (they live in different processes), which is acceptable because both are compile-time tables reviewed in the same change.

## Risks / Trade-offs

- **Behavior drift during transcription** (flags, order, initial-show conditions) → transcribe one-to-one from the current builders, rely on the existing 211 Rust / 90 frontend tests plus the numbered manual Windows checklist; the freeze baseline tag/backup allows instant rollback of a stage commit.
- **cwd assumption for the capabilities test** → cargo test runs from `src-tauri`; fallback to `env!("CARGO_MANIFEST_DIR")` if the assumption breaks; the test failure message names the file.
- **TS/Rust registry duplication** (two files must both be updated for a new window) → documented in ADR-0037 and covered by tests on both sides; a future tier-2 gate could diff them automatically.
- **Tray now derived from registry** — if a future float window should NOT appear in the tray, add an explicit tray flag later; today's four floats all appear, so behavior is unchanged.

## Migration Plan

No data or config migration: `settings` grid/collapsed keys keep the same window labels, so saved layouts remain valid. Rollback = revert the stage commit (freeze tag v1.12.10-restructure-freeze + backup archive exist). The change is delivered in three commits (Rust, frontend, docs/Eval), each gated; archiving the OpenSpec change waits for the user's manual Windows acceptance.

## Open Questions

None — all decisions affecting specs/approach/tasks are resolved above.
