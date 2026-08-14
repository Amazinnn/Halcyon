## 1. Rust window registry

- [x] 1.1 Create `apps/desktop/src-tauri/src/window_spec.rs` with `WindowKind` (Desktop/Float/Bubble/Overlay/Topbar), `WindowSpec` (label, title, kind, default_rect, transparent, always_on_top, skip_taskbar, resizable, fullscreen, ignore_cursor_events, float_host, setup_acrylic, hidden_at_start, fixed_size), and `pub const WINDOW_SPECS` listing the nine windows with values transcribed one-to-one from the current builders in `create_windows`
- [x] 1.2 Add derived helpers `float_labels()`, `is_float_label()`, `spec(label)` to the new module
- [x] 1.3 Refactor `create_windows` in lib.rs to a spec-driven loop preserving creation order, per-kind branches (Float keeps `initial_float_rect` + `configure_float_host`; Bubble/Topbar/Overlay keep ignore-cursor/noactivate handling), and the trailing initial-show loop unchanged
- [x] 1.4 Remove the `FLOAT_LABELS` constant; route `is_float_label` through the registry; update `apply_initial_layout` to `float_labels()` and read each float's `default_rect` from the spec (drop the workflow special case)
- [x] 1.5 Drive the setup glass loop from specs with `setup_acrylic`; pet keeps its existing derived-tint acrylic path
- [x] 1.6 Remove the dead `"logos"` entry from `apps/desktop/src-tauri/capabilities/default.json`
- [x] 1.7 Add window_spec tests: float set content/order, float membership positives and negatives (desktop/topbar/pet-bubble/grid-overlay excluded), spec lookup, unique labels, and exact equality of `WINDOW_SPECS` labels with the capabilities `windows` array; update the existing is_float_label tests and test use list
- [x] 1.8 Gate the Rust stage: `cargo test --lib`, `openspec validate --specs --strict`, `git diff --check`; commit as feat(window-registry)

## 2. Frontend view registry

- [x] 2.1 Create `apps/desktop/src/lib/view-registry.ts` with `ViewSpec` (label, kind, title, icon, component, transparent), `VIEW_REGISTRY` (nine entries; float views carry tray title/icon), `viewForLabel()` (unknown label falls back to DesktopView), `floatViews()`, `isTransparentLabel()`
- [x] 2.2 Replace the `App.vue` switch with `viewForLabel()` and the hard-coded transparent-label list with `isTransparentLabel()`
- [x] 2.3 Replace the four hard-coded tray buttons in `DesktopView.vue` with a v-for over `floatViews()` calling `openView(v.label)`
- [x] 2.4 Add `apps/desktop/src/lib/view-registry.test.ts` covering label→component mapping, unknown-label fallback, transparent set, floatViews content/order, and unique labels
- [x] 2.5 Gate the frontend stage: `npm test -- --run`, `npm run build`, `git diff --check`; commit as feat(window-registry)

## 3. Full gates, Eval, and manual acceptance

- [ ] 3.1 Run the full gate set: `npm test -- --run`, `npm run build`, `cargo test --lib`, `packages/event-schema npm test`, `openspec validate --specs --strict`, `git diff --check`; stop desktop.exe/watchdog, then `npm run tauri build -- --no-bundle`
- [ ] 3.2 Write `docs/evals/2026-08-14-window-registry-refactor-checkpoint.md` with scope (requirement #125, ADR-0037, this change), gate commands/results, test counts, and all manual items Pending
- [ ] 3.3 Update `docs/STATUS.md` (checkpoint section linking the Eval, manual acceptance Pending) and `docs/next-phase.md` (tier-1 progress; next candidate Focus UI Kit)
- [ ] 3.4 Commit docs and push all stages to origin (Clash proxy or `git -c http.proxy= -c https.proxy= push`)
- [ ] 3.5 Deliver the numbered manual Windows checklist to the user; after acceptance, `openspec sync-specs` and archive the change; no release tag until acceptance