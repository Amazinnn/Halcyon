## 1. Native capsule clipping

- [x] 1.1 Add a red-first Rust geometry regression for a topbar client-pixel
  pill: radius equals half height and the creation-only configuration occurs
  once.
- [x] 1.2 Apply the topbar-only native region while hidden at creation, with
  correct Windows region ownership and no float-host frame configuration.
- [x] 1.3 Add topbar to runtime global acrylic synchronization without changing
  activation, mouse-through, topmost, grid, tray, desktop-lock, Provider, or
  workflow paths.

## 2. Evidence and acceptance

- [x] 2.1 Add ADR-0033 for the topbar-only native acrylic region and update
  STATUS, the Eval, and any affected incident evidence.
- [x] 2.2 Run frontend tests, frontend build, `cargo test --lib`, event-schema
  tests, strict validation for this and the pet change plus global specs,
  `git diff --check`, and `launch-focus.cmd rebuild`.
- [ ] 2.3 Pause for user mouse-driven Windows acceptance: visible/hidden and
  focus-state capsule boundaries remain pill-shaped, global acrylic toggles
  apply, clicks pass through, and no float/grid/tray regressions occur.
