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

### Bubble delivery revision (2026-08-14)

Successful direct Agent replies must reach the independent pet-bubble window
when that WebView finishes initialization after the provider event. The core
keeps only the newest current-Agent envelope in memory for 30 seconds; a bubble
window claims it once after initialization. This does not persist through a
Focus restart, and chat visibility does not affect delivery.

### Follow-up revision (2026-08-14)

The reported first-reply failure is a same-Agent initialization path, not an
Agent switch: persisting an already-current Agent MUST NOT discard the pending
delivery before the bubble can claim it. The global settings surface also gains
a persisted, default-off control for displaying Provider-public text deltas on
the next direct conversation. It never exposes hidden reasoning, workflow
steps, or tool summaries. The pet WebView surface uses the existing
package-derived host tint at 50% opacity so its native acrylic remains visible.

### Controller and public-stream revision (2026-08-14, Requirement #120)

The prior bubble claim and cross-window event path is replaced. A native Bubble
Controller owns one current-Agent, 30-second pending envelope and dispatches it
only to a ready `pet-bubble` host. The host must confirm that it rendered the
payload before native placement and no-activate display consume the envelope.
Reloads, delayed readiness, failed acknowledgement, and failed placement retain
the envelope until deterministic expiry or a real Agent switch/deletion.

The streaming setting is a display policy, not a request-mode flag: Codex and
Claude share one adapter boundary that accepts only Provider-explicit public
text deltas. A turn without such a Claude delta displays an activity indicator
until its final reply. Hidden reasoning, tools, and workflow events remain out
of chat.

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
acrylic color, direct-chat display preference, state mapping, and the companion
window. They do not alter the accepted float chrome, desktop locking, grid
geometry, tray behavior, Provider hidden reasoning, workflow behavior, or the
separate top status capsule.

## Acceptance Boundary

The user has accepted the post-release drag deadlock repair. Remaining native
and visual checks stay explicit in the Eval. This checkpoint may be committed
and pushed to `main` after automated verification, but it MUST remain marked
pending and MUST NOT be tagged as a release until the deferred manual gate is
completed.

### Resident streaming and delivery-ACL revision (2026-08-14, #121/#122)

Live CDP instrumentation verified the real production failures behind the
recurring "no bubble" and "no streaming" reports. The bubble WebView was never
listed in the Tauri capability window set, so its event listeners were rejected
and the host never reported ready; the Controller therefore never dispatched.
The capability list now includes `pet-bubble` for every delivery path.

Resident Claude turns (stdin held open) never emit `content_block_delta`;
their only increments are partial `assistant` messages whose `thinking`
blocks contain raw newlines (multi-line JSON) and whose `text` blocks are
cumulative. The adapter now parses an accumulating JSON document and consumes
`assistant` messages: `thinking` blocks stream as a new additive
`message.thinking` event, `text` blocks as `message.delta` (cumulative
length diffed), and `tool_use` blocks restore `tool.started`. The user
approved showing the Provider-visible thinking process while the streaming
switch is on; the switch still gates everything, and Codex keeps its existing
public-text path. The topbar host gains a shadow margin so its single-WebView
pill shadow renders fully inside the window and follows the pill curve exactly.
