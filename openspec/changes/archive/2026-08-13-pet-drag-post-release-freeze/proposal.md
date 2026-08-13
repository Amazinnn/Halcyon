## Why

After a desktop pet drag visibly ends, the next click can freeze Focus. The
bounded trace from a real reproduction stops at `finalize:snap:start`: the
placement path holds `state.settings` and then asks the pet bubble path to lock
the same non-reentrant mutex. The Tauri main thread therefore self-deadlocks
before snap completion.

## What Changes

- Add a local-only, explicitly enabled stage trace for one pet drag and its
  first subsequent Focus click.
- Move pet bubble repositioning and all native post-placement work outside the
  settings lock while preserving successful placement and occupied snap-back.
- Reopen INC-020 and record the actual Windows result in a dedicated Eval.
- Preserve current movement, snapping, native host, desktop-lock, and tray
  behavior while diagnosis is underway.

## Capabilities

### New Capabilities
- `pet-drag-diagnostics`: Collects a bounded, opt-in local trace that
  distinguishes post-release lifecycle failure classes.

### Modified Capabilities

- `pet-drag-diagnostics`: Uses the captured trace to require post-placement
  work to run only after settings persistence has released its mutex.

## Impact

Touches the shared Rust drag lifecycle, the pet browser drag boundary, local
application diagnostic output, the production incident ledger, and this
change's Eval. It adds no dependency, recovery process, provider behavior, or
window styling change.
