# ADR-0034: Topbar Shared Host Setup And Exact Pill Region

Status: Accepted, Windows visual verification pending
Date: 2026-08-14
Requirements: #117, #118
OpenSpec: `topbar-capsule-acrylic-clip`
Supersedes: ADR-0033 operational mechanism

## Context

The region-only topbar repair did not alter the observed rectangular acrylic.
Its hidden-HWND `GetClientRect` source can be zero before first show. The other
accepted Focus floating windows already have a stable creation-only native host
setup that strips non-client drawing, sets no-activate behavior, and installs
the known-safe native procedure while hidden.

## Decision

1. Topbar reuses `configure_float_host` once during hidden creation.
2. Topbar then uses Tauri's physical `inner_size` to set one exact native pill
   region with a radius of half its client height.
3. It remains absent from `FLOAT_LABELS`; therefore it never joins grid, tray,
   float drag, desktop-lock, Provider, or workflow ownership.
4. Its existing mouse-through, no-activate, topmost, show/hide/move, and global
   acrylic-toggle behavior remain unchanged.

## Consequences

The topbar shares only proven creation-time host mechanics, not float lifecycle
membership. The exact region is based on a non-zero physical size before first
show. Automated checks cannot prove final Windows composition, so this remains
open until the required user mouse-driven acceptance succeeds.
