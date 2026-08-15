## 1. Font regression

- [x] 1.1 Fix font declaration order in FocusToggle/FocusSegmented/FocusSelect

## 2. Width and layout

- [x] 2.1 Chat skills select fixed 88px
- [x] 2.2 Settings spacing (gap 12px, group padding-top 10px) and .toggle-row divider on five rows

## 3. Docs and tests

- [x] 3.1 ui-design.md font rule and divider utility; focus-kit test assertions
- [x] 3.2 Gate: npm test -- --run, npm run build, scripts/rust-gate.ps1, openspec validate --specs --strict, git diff --check; commit feat(ui-kit)