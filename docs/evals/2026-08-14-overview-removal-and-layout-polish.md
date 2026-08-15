# Overview Removal + UI Layout Polish Checkpoint

Date: 2026-08-14
Requirements: #129, #130
OpenSpec changes: `2026-08-14-remove-overview-panel`, `2026-08-14-ui-layout-polish`
Status: Automated gates green; manual Windows acceptance Pending

## Scope

- **Overview removal (#129)**: the C4 example panel duplicated the stats window;
  the user decided to remove it. Deleted the WindowSpec entry, ViewRegistry
  entry, capability entry, and OverviewPanelView.vue; float-set test back to
  five floats, tray test back to four entries; the panel recipe in
  ui-maintenance.md §3 stays as pure text steps.
- **UI layout polish (#130)**: layout floor tokens
  (--ctrl-min-input 96px / --ctrl-min-select 88px / --text-min-row 120px);
  FocusInput/FocusSelect apply their minimum widths; SettingsPopover Agent
  create row is now two lines (name input full width) and pack-row wraps with
  a 120px name floor so Agent names stay horizontal; audited remaining flex
  rows (no other real overflow risk); ui-design.md gained the layout &
  text-width rules section; kit tests assert the new tokens.

## Automated Gates

| Gate | Result |
| --- | --- |
| npm test -- --run | Pass, 23 files / 129 tests |
| npm run build | Pass |
| scripts/rust-gate.ps1 (cargo test --lib) | Pass, 222 / 0 failed |
| openspec validate --specs --strict / --changes | Pass (7 specs / 7 changes) |
| git diff --check | Clean |
| Release rebuild | Pending (next step) |

## Manual Acceptance (Pending)

1. Tray back to four entries; no overview residue.
2. Settings > Agent: name input wide enough, long Chinese names stay on one
   line; provider select not squeezed; add button works.
3. Settings > Agent management rows: names horizontal (no vertical stacking),
   buttons wrap instead of overflowing.
4. Other settings text (help/run list/app list/state mapping) reads normally.
5. No regression in desktop/workflow/chat/music windows.
6. Startup, focus flow, bubble, glass sync unchanged.

## Notes

- Both OpenSpec changes stay open until the rebuild + manual acceptance pass;
  then sync-specs + archive.
