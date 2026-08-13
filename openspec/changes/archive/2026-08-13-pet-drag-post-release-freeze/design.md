## Context

See proposal.md. `drag_start` and `drag_end` share the generic grid-drag
implementation with other floats. A Rust poller can also emit a natural-release
event, so the post-release sequence crosses browser, background and main-window
threads. The current ownership test only covers the in-memory token.

## Goals / Non-Goals

**Goals:**

- Capture a bounded, ordered trace for exactly the lifecycle boundaries needed
  to classify a real post-release freeze.
- Keep diagnostic collection off unless explicitly enabled and write no user
  content, window pixels, provider data, or persistent runtime state.
- Remove the confirmed settings-mutex self-deadlock without changing window
  movement, snapping, overlay, bubble geometry, or native-host behavior.

**Non-Goals:**

- Do not change native window ownership, movement, snapping, overlay behavior,
  tray behavior, desktop lock, package import, or rendering.
- Do not add a watchdog, telemetry service, retry loop, or automatic repair.

## Decisions

### Environment-gated local JSONL

`FOCUS_DRAG_DIAGNOSTICS=1` enables a local JSONL sink under the existing
application-data directory. Environment gating makes the diagnostic inert for
normal releases and permits a reproducible handoff without a new settings UI.
The launch script exposes a diagnostic launch mode rather than requiring the
user to edit environment variables.

### Bounded lifecycle vocabulary

Rust records ownership, poller, overlay, geometry, placement and completion
boundaries. The pet view reports pointer release/cancel and the first next
pointer down after a release. Each record carries one monotonic sequence,
source, timestamp and active-drag presence. This is sufficient to distinguish
the selected failure classes without high-volume movement logging.

### Best-effort, non-blocking observation

The sink opens/appends per event and ignores I/O errors. It never takes the
window, settings, store, or active-drag mutex while writing, and it never joins
or waits for a thread. This prevents diagnosis from adding a second lifecycle
dependency.

### Settings mutation ends before native post-placement work

The real trace completed pointer release, poller shutdown, overlay hiding and
geometry capture, then stopped at `finalize:snap:start`; it never recorded
`finalize:snap:complete`. Inspection of that boundary showed
`place_window_inner()` retaining the `state.settings` guard while calling
`position_pet_bubble_for_current_pet()`, which immediately locks
`state.settings` again. Because `std::sync::Mutex` is not reentrant, this is a
deterministic self-deadlock on the Tauri main thread.

Placement is therefore split at the ownership boundary. While holding
`settings`, Focus calculates the final rect, persists a successful placement,
and returns that rect. Only after the guard has been dropped does it move the
window, reposition the pet bubble, and raise the topbar. Occupied placement
still returns and positions the previous rect; only the lock lifetime changes.

## Risks / Trade-offs

- [A frozen UI cannot report the next browser click] -> the last Rust stage
  still identifies the furthest completed native boundary.
- [JSONL append can fail] -> diagnosis remains functional; the user is told to
  report missing output rather than treating absence as success.
- [Environment variables are easy to omit] -> `launch-focus.cmd diagnostics`
  sets the variable for the spawned release process.
- [Moving work across the mutex boundary could change snap behavior] -> the
  helper returns the same final rect for both successful and occupied paths,
  and focused tests cover lock release plus both outcomes.
