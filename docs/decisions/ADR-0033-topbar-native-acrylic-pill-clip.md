# ADR-0033: Topbar Native Acrylic Pill Clip

Status: Accepted
Date: 2026-08-14
Requirements: #117
OpenSpec: `topbar-capsule-acrylic-clip`

## Context

The topbar WebView draws a pill, but its HWND acrylic composition is rectangular.
CSS cannot clip that native composition. ADR-0029 intentionally excludes topbar
from float-host frame ownership because topbar is mouse-through and does not
participate in the grid, tray, or float drag lifecycle.

## Decision

1. While hidden at initial creation only, Focus reads topbar's actual client
   pixels and assigns one round-rectangle native region.
2. The region radius is half the client height; its width and height are the
   measured client dimensions.
3. This region is not reapplied during visibility, movement, topmost, focus,
   or acrylic-toggle operations. A successful `SetWindowRgn` takes ownership;
   failure deletes the created region.
4. Runtime global acrylic synchronization includes topbar, while preserving its
   no-activate, mouse-through, topmost contract.

## Consequences

The topbar receives a narrowly scoped native composition clip, not the
ADR-0029 float-host style/procedure configuration. Any future topbar resizing
must explicitly revisit the creation-only region assumption. Automated tests
prove geometry only; Windows visual acceptance remains mandatory.
