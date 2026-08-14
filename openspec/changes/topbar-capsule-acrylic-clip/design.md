## Context

See proposal.md for the reported boundary mismatch. The previous region-only
attempt did not become visible: a hidden HWND can report a zero `GetClientRect`.
Topbar now reuses the accepted hidden float-host creation configuration, then
uses Tauri's physical inner size for its exact native pill region. It remains a
transparent, mouse-through, non-activating window outside all float-label,
grid, tray, and drag ownership.

## Goals / Non-Goals

**Goals:**

- Reuse the accepted hidden host configuration and clip only the topbar native
  composition to its real client-pixel pill.
- Make the existing global acrylic toggle update topbar.
- Preserve creation-only native setup and all existing topbar input/show paths.

**Non-Goals:**

- Do not alter grid sizing, desktop lock, tray, topbar CSS geometry, Provider
  behavior, workflows, or any other HWND's clipping strategy.

## Decisions

### Reuse the accepted host setup, then apply the exact topbar pill

The topbar calls the same one-time hidden `configure_float_host` used by the
accepted floating windows. It then reads Tauri's physical `inner_size` (rather
than hidden-HWND `GetClientRect`) and makes a round-rectangle native region whose
radius is half the client height. This clips native acrylic at the ownership
layer that CSS cannot reach.

CSS-only clipping was rejected because it cannot constrain the already-observed
rectangular native acrylic. The shared host call is creation-only; topbar is not
added to `FLOAT_LABELS`, so it does not inherit grid, tray, or drag ownership.

### Keep the region immutable after creation

The current topbar size is fixed at creation, so show/hide/move need no region
mutation. Helpers make the physical width, height, half-height radius, shared
host setup, and region decision independently testable. If a future change
resizes topbar, it must explicitly revisit this lifecycle.

### Include topbar in global acrylic synchronization only

Startup acrylic application already includes topbar, but runtime preference
updates omit it. The existing toggle loop will include topbar; it does not
create, resize, activate, or otherwise configure the window.

## Risks / Trade-offs

- [Windows region ownership is API-sensitive] → transfer ownership only on a
  successful region assignment and free the region on failure.
- [Automated geometry tests cannot prove Windows composition] → require the
  explicit mouse-driven visual acceptance gate and keep the change unarchived
  until it passes.
- [A future topbar resize can stale the region] → fixed-size assumption is
  explicit; any size change requires a new proposal.

## Migration Plan

The change has no stored-data migration. Rebuild Focus after automated gates;
the native region is recreated on the next process start. Reverting removes the
creation-only region and returns to the previous rectangular native acrylic.
