## Why

Focus can currently leave a visible but empty pet host when the current Agent
has no readable package. The latest observed repro is more serious: after
selecting a package, dragging the pet can make the whole application stop
responding. Package compatibility, rendering, state mapping, and bubble work
cannot be accepted while that native drag path is unsafe.

## What Changes

This change is deliberately phased and gated by real Windows acceptance.

1. Stabilize pet dragging and hide the native pet host when the current Agent
   has no valid package.
2. Add explicit support for the official Hatch Pet package and the Focus
   `focus-hatch-pet` package, then derive a stable visible-content crop and
   render it proportionally. A package-scoped manual aspect correction remains
   available when the generated artwork itself has the wrong proportions.
3. Add Focus-owned continuous state mapping, a pet-colored host, and the
   independent companion bubble for every successful final Agent reply.
4. Hide restricted focus controls and leave the top status capsule to a later
   change.

## Capabilities

### New Capabilities

- `pet-package-and-state-mapping`: validates the two explicit package forms,
  discovers their animations, and maps Focus persistent states.
- `pet-companion-bubble`: provides the independent pet-attached, paged message
  window.
- `settings-experience`: keeps settings descriptions readable and exposes
  package/state information when the corresponding phase is reached.

## Impact

The later pet phases affect only package analysis, pet rendering, pet-specific
acrylic color, state mapping, and the companion window. They do not alter the
accepted float chrome, desktop locking, grid geometry, or tray behavior.

## Acceptance Boundary

The user has accepted the post-release drag deadlock repair. Remaining native
and visual checks stay explicit in the Eval. This checkpoint may be committed
and pushed to `main` after automated verification, but it MUST remain marked
pending and MUST NOT be tagged as a release until the deferred manual gate is
completed.
