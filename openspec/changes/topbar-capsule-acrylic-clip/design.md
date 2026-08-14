## Context

See proposal.md for the reported boundary mismatch. `topbar` is deliberately
excluded from ADR-0029 float-host frame configuration: it is a transparent,
mouse-through, non-activating native window with a pill only in its WebView CSS.
Native acrylic therefore remains rectangular unless Windows composition receives
an explicit region.

## Goals / Non-Goals

**Goals:**

- Clip only the topbar native composition to its real client-pixel pill.
- Make the existing global acrylic toggle update topbar.
- Preserve creation-only native setup and all existing topbar input/show paths.

**Non-Goals:**

- Do not call the float-host configuration on topbar.
- Do not alter grid sizing, desktop lock, tray, topbar CSS geometry, Provider
  behavior, workflows, or any other HWND's clipping strategy.

## Decisions

### Apply a once-only native pill region at hidden topbar creation

The topbar creation path reads the actual client rectangle in pixels and makes a
round-rectangle native region whose radius is half the client height. It applies
the region once before visibility. This clips native acrylic at the ownership
layer that CSS cannot reach.

CSS-only clipping was rejected because it cannot constrain the already-observed
rectangular native acrylic. Reusing ADR-0029 float-host configuration was
rejected because that established policy explicitly excludes topbar and carries
unrelated non-client and drag ownership.

### Keep the region immutable after creation

The current topbar size is fixed at creation, so show/hide/move need no region
mutation. A helper will make the client width, height, and half-height radius
independently testable; Windows calls remain confined to the existing native
window module. If a future change resizes topbar, it must explicitly revisit
this design and the region lifecycle.

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
