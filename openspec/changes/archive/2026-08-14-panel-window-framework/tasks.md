## 1. Declarations

- [x] 1.1 Add `overview` WindowSpec (Float, default/birth rect, inTray true) to window_spec.rs
- [x] 1.2 Add `overview` entry to frontend view-registry.ts (component + tray title/icon)
- [x] 1.3 Add `overview` to capabilities/default.json windows array

## 2. Panel view

- [x] 2.1 Create views/overview/OverviewPanelView.vue (today summary + recent runs, focus:tick + workflow:changed live updates, kit components)
- [x] 2.2 Add frontend test for the panel store logic (summary/runs rendering inputs)
- [x] 2.3 Gate: npm test -- --run, npm run build, cargo test --lib, openspec validate --specs --strict, git diff --check; commit feat(panel)

## 3. Docs and gates

- [x] 3.1 Add the panel recipe section to docs/ui-maintenance.md
- [x] 3.2 Full gate set + rebuild; Eval snapshot; STATUS/next-phase updates; commit docs