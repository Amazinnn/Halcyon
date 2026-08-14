# ADR-0033: Topbar Native Acrylic Pill Clip

Status: Superseded by ADR-0034
Date: 2026-08-14
Requirements: #117, #118
OpenSpec: `topbar-capsule-acrylic-clip`

## Context

The topbar WebView draws a pill, but its HWND acrylic composition is rectangular.
CSS cannot clip that native composition. The region-only method documented here
used hidden-HWND `GetClientRect`, which can be zero before first display and did
not correct the user-visible boundary. ADR-0034 replaces its operational path.

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

This historical region-only decision was superseded after the Windows visual
report. Automated tests prove geometry only; Windows visual acceptance remains
mandatory.
