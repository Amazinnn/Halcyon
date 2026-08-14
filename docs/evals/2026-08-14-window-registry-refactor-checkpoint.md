# Window Registry Refactor Checkpoint

Date: 2026-08-14
Requirement: #125
ADR: ADR-0037
OpenSpec change: `2026-08-14-window-registry-declarative`
Commits: e0a2c77 (proposal + #125 + ADR-0037), b60b570 (Rust registry), aa82a17 (frontend ViewRegistry)
Status: Automated gates pass; Windows manual acceptance Pending

## Scope

Pure structural refactor (tier 1 of the approved restructure round): the
declarative window registry.

- Rust: new `window_spec.rs` with `WindowKind` (Desktop/Float/Bubble/Overlay/
  Topbar), `WindowSpec`, and `WINDOW_SPECS` (nine windows, values transcribed
  one-to-one from the previous builders). `create_windows` is now a
  spec-driven loop with the same creation order and flags; `FLOAT_LABELS` is
  abolished (float set, initial-layout defaults, and setup glass derive from
  the registry; the workflow special case is gone). Dead `"logos"` removed
  from `capabilities/default.json`; a Rust test enforces exact registry ↔
  capability-list equality.
- Frontend: `src/lib/view-registry.ts` (`VIEW_REGISTRY` label → component /
  transparent / tray entry with an explicit `inTray` flag), `App.vue` switch
  and transparent-label list replaced, `DesktopView.vue` tray rendered from
  `floatViews()`.
- Documents: requirement #125 appended, ADR-0037 added, OpenSpec change with
  proposal / window-registry spec / design / tasks (all planning artifacts
  complete; tasks 1.1-2.5 implemented).

## Automated Gates

| Gate | Command | Result |
| --- | --- | --- |
| Frontend tests | `cd apps/desktop && npm test -- --run` | Pass, 21 files / 115 tests (incl. 5 new view-registry tests) |
| Frontend build | `cd apps/desktop && npm run build` | Pass (vue-tsc + vite, 111 modules) |
| Rust tests | `cd apps/desktop/src-tauri && cargo test --lib` | Pass, 221 passed / 0 failed / 1 ignored (incl. 3 new window_spec tests) |
| Schema | `cd packages/event-schema && npm test` | Pass (12 valid, 4 invalid fixtures) |
| OpenSpec | `openspec validate --specs --strict` | Pass, 7 specs |
| OpenSpec | `openspec validate --changes` | Pass, change valid |
| Diff hygiene | `git diff --check` | Clean |
| Release rebuild | `npm run tauri build -- --no-bundle` | Ran after stopping Focus processes (see notes) |

## Manual Acceptance (all Pending)

None of the following has been verified by the user yet; Pending is not Pass.

1. Launch Focus: desktop fullscreen normal, no startup stacking regression.
2. View tray: four entries (对话/统计/音乐/工作流) with same names, icons, order; opening each shows the saved layout.
3. Float drag: grid glow center follows the window, near-bright/far-dim gradient unchanged; snap lands on the grid.
4. Collapse/restore: collapse hides immediately; restore with no free slot stays collapsed with a hint; no overlap.
5. Glass: chat/stats/music/workflow glass and the settings opacity slider stay globally synchronized; pet tint unchanged.
6. Pet bubble: reply bubbles appear beside the pet, avoid the chat window, sentence-complete paging and dynamic sizing unchanged.
7. Topbar capsule: position, glass shadow fit, mouse click-through unchanged.
8. Overlay preview on drag/resize; layout persists across restart.
9. Focus flow (start/pause/skip/end) and desktop-lock linkage unchanged.

## Notes

- The change intentionally has zero observable behavior change; the 
  consistency test between `WINDOW_SPECS` and `capabilities/default.json`
  windows guards future window additions (three declarations: Rust spec,
  frontend registry, capability list).
- No release tag; OpenSpec change stays open until manual acceptance; then
  `openspec sync-specs` + archive.
