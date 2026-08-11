# ADR-0029: Native Ownership of Float Hosts

Status: Accepted
Date: 2026-08-11
Requirements: #86, #88, #89, #90, #91, #92, #93, #94, #95
Incident: INC-001

## Context

The internal Focus floats use transparent WebView content. Windows keeps a
hidden non-client band on borderless popup hosts, and a default background
erase exposes white pixels when the host is moved. Earlier fixes repeatedly
rewrote styles, subclass chains, and `SWP_FRAMECHANGED` during the window
lifecycle. That made the visual result timing-dependent: the static style
probe could be clean while a moved window still showed a blue or white strip.

## Decision

1. `chat`, `stats`, `music`, `pet`, and `workflow` are configured once while
   hidden. The host keeps `WS_POPUP` and `WS_EX_NOACTIVATE`.
2. Focus replaces `GWLP_WNDPROC` once per float HWND during hidden creation.
   The procedure returns `0` for `WM_NCCALCSIZE` and `1` for
   `WM_ERASEBKGND` and `WM_NCACTIVATE`; all other messages are forwarded to
   the original Tauri procedure. The activation result prevents the delegated
   default non-client renderer from painting a caption during a native drag.
   Focus does not use `SetWindowSubclass`.
3. The initial hidden configuration is the only path allowed to change the
   host style or send `SWP_FRAMECHANGED`. Showing, hiding, moving, resizing,
   restoring, and topmost changes use `SetWindowPos` with `SWP_NOACTIVATE`;
   drag movement also uses `SWP_ASYNCWINDOWPOS`.
4. Grid positioning converts the live client/outer delta into an outer rect.
   `topbar` and `grid-overlay` only use the no-activation show path and do not
   receive float-host frame configuration.
5. Each float host also receives `DWMWA_WINDOW_CORNER_PREFERENCE = ROUND`
   once during the same hidden configuration. This clips native acrylic at the
   host-composition layer; it does not use `SetWindowRgn` and it does not alter
   the verified non-client-message or drag paths.

## Evidence and Consequences

The current release baseline disproved the managed-subclass approach: all five
floats were clean at first open but displayed the strip during real mouse
movement and after mouse-up. Completely removing the procedure exposed native
top/left outlines and the host title. The historical direct-procedure path is
therefore restored with a strict creation-only boundary; it is never reapplied
during show, drag, mouse-up, positioning, restore or topmost changes.

The user's confirmation that the remaining top strip contains the host title
identifies it as a native caption. The `WM_NCACTIVATE` ownership is a focused
third candidate, not visual proof. INC-001 remains `Open / S2` until the user
confirms real dragging, resizing, collapsing, restoring and topmost operations
for all five floats.
