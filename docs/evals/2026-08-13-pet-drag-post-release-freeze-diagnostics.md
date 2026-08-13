# Pet Drag Post-Release Freeze Diagnostic Eval

Date: 2026-08-13
Requirement: #108
Incident: INC-020 (Verified / S2)
OpenSpec change: `pet-drag-post-release-freeze`
Status: Root cause repaired and accepted by the user

## Scope

This snapshot is limited to evidence collection for the production freeze after
a pet drag has visibly ended. It does not claim that the earlier single-release
ownership change fixed the native lifecycle, and it does not alter accepted
float chrome, brightness geometry, tray behavior, desktop locking, or package
compatibility.

## Planned Evidence

When `FOCUS_DRAG_DIAGNOSTICS=1`, the local diagnostic file records only the
drag sequence number, `pet` label, source, timestamp, active-drag presence, and
these stages: browser pointer down/release/cancel, active-drag claim and stop,
poller exit, overlay hide, geometry read, snap completion, and the first
subsequent Focus click. It is local-only and disabled by default.

## Automated Evidence

| Check | Status | Evidence |
| --- | --- | --- |
| Rust diagnostic ordering | Passed | Included in `cargo test --lib`; ordered diagnostic lifecycle regression passed. |
| Frontend release/click boundaries | Passed | `npm test -- --run`: 17 files, 69 tests passed. |
| Focused lock-boundary regression | Passed | Red first on missing helper; then 2 placement lock tests passed. |
| Frontend production build | Passed | `npm run build`; Vue typecheck and Vite production build completed. |
| Rust library suite | Passed | 192 discovered: 191 passed, 1 existing ignored, 0 failed. |
| Event schema | Passed | 11 valid and 4 invalid fixtures checked; TypeScript check passed. |
| OpenSpec and diff | Passed | Strict change validation and `git diff --check` passed. |
| Release rebuild | Passed | `launch-focus.cmd rebuild` exited 0 after 59.7 seconds. |

## Repair Evidence

The regression was developed red-first. The focused Rust command initially
failed with `no resolve_window_placement in the root`, proving the test required
the new lock boundary. After the minimal split, both free placement and
occupied snap-back tests pass and immediately reacquire the settings mutex
after the placement calculation returns.

The production path now performs only grid calculation and successful
persistence while owning `settings`. Native positioning, current-pet bubble
lookup and topbar raising execute after that function returns. No native drag,
overlay, window-style, brightness, tray or desktop-lock code changed.

`cargo fmt -- --check` was also inspected but is not used as a gate: it reports
pre-existing formatting drift across the broader uncommitted Agent-first Rust
work. Applying it would create unrelated edits outside this repair. The changed
lock-boundary code was formatted locally, and the repository's required
`git diff --check` gate passed.

## Repair Acceptance

The rebuilt release was checked with the required mouse sequence:

1. Drag the desktop pet and release the left mouse button.
2. Click chat or the Focus main window.
3. Click one other Focus window.
4. Drag the desktop pet a second time.

The user reported “正常了，谢天谢地。” after this sequence. Every later Focus
click responded and a subsequent pet drag started normally. INC-020 is therefore
Verified and the diagnostic OpenSpec change may be archived. This acceptance
does not automatically verify the later calibration, tint, or companion UI.

## First Reproduction Attempt

The user successfully reproduced the freeze after the first diagnostic build,
but no `pet-drag.jsonl` was created and no Focus process remained. Inspection
showed that `launch-focus.cmd rebuild` had launched a normal release first, and
the later diagnostics branch used backslash quote escaping that Windows `cmd`
does not support. The tested instance therefore had no diagnostic environment.
This is a diagnostic-launch defect, not evidence for any product root cause.

The launch branch now sets `FOCUS_DRAG_DIAGNOSTICS=1` in its `setlocal`
environment before directly starting the release executable. The recorder also
writes a `diagnostics:enabled` marker at startup so enablement is mechanically
verified before asking for another mouse reproduction.

## Manual Reproduction Gate

1. Start the diagnostic release with `FOCUS_DRAG_DIAGNOSTICS=1`.
2. Drag the packaged desktop pet, release the left button, then click chat or
   the desktop and one other Focus window.
3. If Focus freezes, end only the Focus process if necessary. Do not relaunch
   before the diagnostic file has been preserved.
4. Report the result. The log determines whether the next change is an
   incomplete release, an overlay/input interception, a native window block,
   or another Focus main-thread block.

The corrected diagnostic launch produced the real evidence below.

## Successful Diagnostic Reproduction

The user reproduced the freeze again with the corrected diagnostic release.
The preserved trace ends as follows:

```text
drag:start
browser:pointerdown
browser:pointerup
poller:stopped
release:claimed
finalize:overlay-hide:start
finalize:overlay-hide:complete
finalize:geometry:start
finalize:snap:start
```

There is no `finalize:snap:complete`. This excludes a stale browser release,
an unclaimed `active_drag`, and an overlay that failed to hide. At the exact
stalled boundary, `place_window_inner()` holds `state.settings` and invokes
`position_pet_bubble_for_current_pet()`, which immediately attempts to acquire
`state.settings` again. Rust's `std::sync::Mutex` is not reentrant, so the Tauri
main thread self-deadlocks deterministically.

The repair is limited to ending settings ownership before native positioning,
pet bubble repositioning and topbar raising. The release requires a second
mouse-driven gate after automated verification; the successful diagnostic
reproduction is evidence of the cause, not evidence of the repair.
