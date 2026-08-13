## 1. Diagnostic boundaries

- [x] 1.1 Add red-first Rust coverage for opt-in diagnostic enablement and
  ordered lifecycle events without changing drag behavior.
- [x] 1.2 Add the bounded Rust JSONL recorder and instrument ownership,
  poller, overlay, geometry, placement, and completion boundaries.
- [x] 1.3 Add red-first frontend coverage for browser release/cancel and the
  first post-release click boundary, then wire the pet view to the recorder.
- [x] 1.4 Add `launch-focus.cmd diagnostics` to start only the release child
  with diagnostics enabled.

## 2. Evidence gate

- [x] 2.1 Run focused and full build/test/schema/diff/rebuild checks; update
  the Eval with actual command evidence.
- [x] 2.2 Pause for the user to reproduce once and preserve the diagnostic
  JSONL. Classify the result in the Eval and update INC-020 without claiming a
  repair.

## 3. Confirmed repair

- [x] 3.1 Add red-first Rust coverage proving the placement calculation
  releases `settings` before pet post-placement work, including successful and
  occupied outcomes.
- [x] 3.2 Split `place_window_inner()` at that lock boundary and keep native
  movement, overlay, styles, brightness geometry, tray and desktop lock
  unchanged.
- [x] 3.3 Run focused and full frontend/Rust/schema/build/diff/OpenSpec checks,
  update the Eval and INC-020, then rebuild the release.
- [x] 3.4 Pause for the real mouse gate: drag, release, click chat/main/another
  Focus window, then drag the pet again. Do not close the incident before the
  user reports success.
