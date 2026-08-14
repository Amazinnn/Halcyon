# Focus UI Kit / CLI Registry / Event Streams / Panel Framework Checkpoint

Date: 2026-08-14
Requirements: #126, #127, #128
ADR: ADR-0037 (window registry), ADR-0038 (UI kit), extensibility plan v1
OpenSpec changes:
- `2026-08-14-focus-ui-kit` (C1)
- `2026-08-14-cli-command-registry` (C2)
- `2026-08-14-event-grouping-thin-windows` (C3)
- `2026-08-14-panel-window-framework` (C4)
Status: Code implemented; frontend gates green; **cargo test deferred to a
Windows restart** (system update pending — see Environment Note); manual
Windows acceptance Pending

## Scope

One batch implementation of the extensibility roadmap C1-C4 (user: "一把做完吧"):

- **C1 Focus UI Kit**: control tokens added to styles.css (:root); eight kit
  components (FocusButton/FocusToggle/FocusSegmented/FocusInput/FocusSlider/
  FocusSelect/FocusCard/FocusWindowFrame); SettingsPopover/DesktopView/
  WorkflowView/ChatView migrated off hand-written control styles (dead styles
  removed, ~40 duplicated rule groups deleted); docs/ui-design.md and
  docs/ui-maintenance.md delivered. WindowHeader.vue replaced by
  FocusWindowFrame (identical props/behavior).
- **C2 CLI command registry**: COMMAND_SPECS declarative table (match pattern,
  agent whitelist flag, help, handler); dispatch and agent_whitelisted derive
  from it; `debug windows` now iterates the window-registry float set;
  focus-cli help text derives from the registry (client/server share one
  command source). Wire semantics (token, audit, JSON framing) unchanged.
- **C3 Event streams**: CoreEvent gains a Domain enum + domain() (Focus/Stats/
  Agent/Workflow/Supervision/Pet/Music/Probe/Panel) with an exhaustiveness
  test; agent store gains thin init (topbar/pet-bubble/grid-overlay subscribe
  only to pet:state_changed); App.vue routes thin vs full init per label;
  docs/architecture/event-streams-v1.md documents the subscription matrix.
- **C4 Panel framework**: `overview` panel window declared in all three
  places (WindowSpec / ViewRegistry with inTray / capabilities) and assembled
  from read-only queries + event subscriptions + kit components; the panel
  recipe is documented in ui-maintenance.md §3. Tray grows from 4 to 5
  entries (additive, covered by manual checklist).

## Automated Gates

| Gate | Command | Result |
| --- | --- | --- |
| Frontend tests | `cd apps/desktop && npm test -- --run` | Pass, 23 files / 129 tests (incl. 6 kit tests, 2 migration tests, 3 thin-mode tests, 5 registry tests) |
| Frontend build | `cd apps/desktop && npm run build` | Pass (vue-tsc + vite; CSS 53.05 kB, down from 53.51 kB after dead-style removal) |
| Rust compile | `cargo test --lib` (build phase) | Compiles clean; **runtime blocked** (see Environment Note) |
| Schema | `cd packages/event-schema && npm test` | Pass (12 valid, 4 invalid fixtures) |
| OpenSpec | `openspec validate --specs --strict` / `--changes` | Pass (7 specs; 5 changes incl. window-registry) |
| Diff hygiene | `git diff --check` | Clean |
| Release rebuild | `npm run tauri build -- --no-bundle` | **Deferred with cargo test** (same environment block) |

## Environment Note (blocking, not a code defect)

Every debug test binary (cargo test, focus-cli.exe) now fails at process
start with STATUS_ENTRYPOINT_NOT_FOUND (0xc0000139), while release binaries
and a minimal throwaway crate run fine. Diagnosis: the machine has pending
Windows updates — `Component Based Servicing\RebootPending` and
`WindowsUpdate\RebootRequired` are set, and PendingFileRenameOperations
contains queued replacements. This is an environment-level loader failure,
not a code regression (the same crate compiled and ran 221 tests earlier
today). **Action: restart Windows, then rerun `cargo test --lib`, the
event-schema tests if needed, and `npm run tauri build -- --no-bundle`**,
then the manual checklist below can be executed.

## Manual Acceptance (all Pending until restart + rebuild)

1. Settings popover: every control (toggles, slider, segmented, inputs,
   selects, buttons incl. hover/disabled states) renders and behaves as before
   (visual zero-change promise of C1).
2. Desktop view: tray now shows 5 entries (对话/统计/音乐/工作流/概览); URL
   inputs and focus-mode pill segmented unchanged.
3. Workflow view: run/stop, list-row ghost buttons, trigger selects, parameter
   segments unchanged.
4. Chat view: agent select, skills select, streaming ghost button unchanged;
   window header pin/collapse in all four floats unchanged.
5. Overview panel: opens from tray, shows today focus + recent runs, updates
   live on workflow changes, collapses/restores like any float.
6. Topbar: pet state dot still shows agent pet state (thin mode regression).
7. focus-cli: run every command group (timer/stats/desktop/apps/workflow/
   agent); --help lists all commands; agent whitelist + audit still enforced.
8. Float drag/snap/collapse/restore, glass opacity sync, pet bubble, desktop
   lock linkage — no regression.

## Notes

- All four OpenSpec changes stay open until the restart-unblocked gates and
  the manual acceptance pass; then sync-specs + archive each.
- 6 pre-existing Rust warnings remain (dead code, mem::forget) — untouched,
  listed for the tier-2 clippy gate.
