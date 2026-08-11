# Eval: 2026-08-11 Float Window Repair Session

## Scope

Requirement #91 and INC-001 only. This session investigates the blue/white
border that appears after a real user drag of a Focus internal float.

## Evidence Rules

- Native style probes, synthetic movement, screenshots, unit tests and builds
  may support a causal hypothesis but cannot pass the visual gate.
- The only visual acceptance is the user's real release test after an explicit
  request from Focus maintenance.
- No unrelated Agent, Skill, workflow, desktop-lock or UI work belongs here.

## Current State

`Pending user verification`. The release is from `main`; it now contains the
one-time host configuration but no float-host subclass or non-client message
handler. The user has not accepted the result of a real mouse drag, so INC-001
remains `Open`.

The current release was rebuilt from a dirty `main` worktree. This is not a
branch mismatch, but manual evidence must identify that rebuilt release rather
than an earlier commit as its baseline.

Manual baseline on the current release: all five float types initially open
without the strip; the strip appears both while dragging and after mouse-up.
This rules out a restore-only or topmost-only trigger. No fix has been accepted.

## Automated Evidence

| Check | Result | Evidence |
| --- | --- | --- |
| Root-cause regression (red) | Pass | `float_hosts_delegate_nonclient_messages_to_native_windowing` failed under the old `WM_NCCALCSIZE -> 0` handler, as expected. |
| Root-cause regression (green) | Pass | The same test passes after removing the float subclass and all non-client message overrides. |
| Desktop build | Pass | `cd apps/desktop && npm run build` completed. |
| Rust library tests | Pass | `cd apps/desktop/src-tauri && cargo test --lib`: 173 passed. |
| Event schema | Pass | `cd packages/event-schema && npm test`: 11 valid and 4 invalid fixtures checked. |
| Release rebuild | Pass | `launch-focus.cmd rebuild` exited 0. |

## Required User Gate

After a proposed fix is rebuilt, the maintainer must stop and ask the user to
open and drag each of `chat`, `stats`, `music`, `pet` and `workflow`, then
report whether any blue/white border, disabled hide control or overlap occurs.

## Candidate Result

`Failed by manual verification`. Without the subclass, the candidate exposed
top and left native outlines and the host window title in the upper-left of the
WebView. The repair must return to root-cause investigation; no visual success
is claimed.

## Candidate 2

`Pending user verification`. The repair restores the historical direct window
procedure only during hidden creation, forwarding every message except the
full-client and background-erase results to Tauri's original procedure. It
does not use the managed subclass or reapply native configuration later.
The focused regression was red without the procedure and green after this
creation-only path was restored. `npm run build`, `cargo test --lib` (173
passed), event-schema tests and `launch-focus.cmd rebuild` have passed.

Manual result: partial. The left native outline is gone, but the upper strip
remains. The next step is source identification, not another style change.

The source-identification gate has now passed: the user confirmed that the
remaining upper strip includes the host window title. It is therefore a
Windows native caption, not a Focus web component. The next hypothesis is
limited to `WM_NCACTIVATE` reaching the delegated Tauri/default procedure
during drag and repainting that caption. No visual acceptance is implied.

## Candidate 3: Activation Paint Ownership

The focused test was deliberately red before implementation:
`float_host_keeps_a_full_client_rect_without_default_background_erase` expected
`WM_NCACTIVATE (0x0086) -> 1`, but received `None` from the delegated path.

The same creation-time direct window procedure now returns `1` for that one
activation message, alongside the existing full-client and background-erase
handling. No style bit, `SWP_FRAMECHANGED`, show/hide, raw movement, snap, or
resize code changed.

| Check | Result | Evidence |
| --- | --- | --- |
| Focused Rust test, red | Pass | Failed with `left: None`, `right: Some(1)` before production change. |
| Focused Rust test, green | Pass | Passed after adding only the activation-message result. |
| Desktop build | Pass | `cd apps/desktop && npm run build`. |
| Rust library tests | Pass | `cd apps/desktop/src-tauri && cargo test --lib`: 173 passed. |
| Event schema | Pass | `cd packages/event-schema && npm test`: 11 valid and 4 invalid fixtures checked. |
| Release rebuild | Pass | `launch-focus.cmd rebuild` exited 0. |

Visual state remains `Pending`. The next action after rebuilding is the user's
real mouse drag of the five float types; no script or screenshot substitutes
for that gate.

## Candidate 3 Manual Result

`Verified` for the original incident symptom: the user confirmed the top
native caption and its title text are no longer visible after real dragging.

The same manual test revealed a separate visual defect: the native acrylic
composition layer extends as a rectangle beyond the WebView's rounded corners.
This is not a return of INC-001's non-client caption. Investigation is limited
to native composition clipping; the verified caption path must remain intact.

## Native Acrylic Corner Candidate

The DWM corner-preference test was deliberately red because the helper API did
not exist. It is green after enabling `Win32_Graphics_Dwm` and applying
`DWMWA_WINDOW_CORNER_PREFERENCE = ROUND` once from `configure_float_host()`.
This is a host-composition preference, not an HWND region or CSS approximation.

| Check | Result | Evidence |
| --- | --- | --- |
| Focused Rust test, red | Pass | The helpers were absent, producing the expected unresolved-name compile failure. |
| Focused Rust test, green | Pass | `float_hosts_prefer_dwm_rounded_corners_for_native_acrylic` passes. |
| Desktop build | Pass | `cd apps/desktop && npm run build`. |
| Rust library tests | Pass | `cd apps/desktop/src-tauri && cargo test --lib`: 174 passed. |
| Event schema | Pass | `cd packages/event-schema && npm test`: 11 valid and 4 invalid fixtures checked. |
| Release rebuild | Pass | `launch-focus.cmd rebuild` exited 0. |

The visual gate remains user-only: inspect the four corners of every float,
then drag one float to confirm the verified caption issue has not returned.

## CSS Radius Alignment Candidate

The user approved the low-risk alignment path after reporting a small remaining
curve mismatch. The WebView outer clip and the five floating view roots now use
`--window-host-radius: var(--r-md)` (12px), matching the DWM rounded-corner
preference. Inner cards retain their existing component radii.

| Check | Result | Evidence |
| --- | --- | --- |
| Focused CSS regression, red | Pass | `float-host-radius.test.ts` failed before the token existed. |
| Focused CSS regression, green | Pass | The token and all five host roots are covered. |
| Frontend tests | Pass | `cd apps/desktop && npm test`: 12 files, 47 tests. |
| Desktop build | Pass | `cd apps/desktop && npm run build`. |
| Rust library tests | Pass | `cd apps/desktop/src-tauri && cargo test --lib`: 174 passed. |
| Event schema | Pass | `cd packages/event-schema && npm test`: 11 valid and 4 invalid fixtures. |
| Release rebuild | Pass | `launch-focus.cmd rebuild` exited 0. |

Manual gate: inspect all five float corners and drag one of them. The acrylic
corners must follow the WebView curve with no rectangular protrusion, and the
previously verified caption/title must stay absent.

## Pet-Baseline Parameter Tuning

The user reported only a very small remaining mismatch and identified the pet
window as the visual baseline. The shared WebView host radius changed from
12px to 10px; this is limited to the outer clip and leaves inner cards and all
native window behavior unchanged. At the user's direction, this tuning round
requires only a release rebuild and no separate verification run.

| Check | Result | Evidence |
| --- | --- | --- |
| Release rebuild | Pass | `launch-focus.cmd rebuild` exited 0 after the 10px tuning. |
