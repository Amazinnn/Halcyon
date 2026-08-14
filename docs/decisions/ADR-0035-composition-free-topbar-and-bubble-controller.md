# ADR-0035: Composition-Free Topbar And Confirmed Bubble Controller

Status: Accepted, Windows visual verification pending
Date: 2026-08-14
Requirements: #120
OpenSpec: `topbar-capsule-acrylic-clip`, `pet-state-pack-and-settings`
Supersedes: ADR-0033 and ADR-0034 topbar mechanism

## Context

Windows visual repro showed that native acrylic remained rectangular despite
native regions. It also showed that an event/claim based bubble handoff could
lose the only visible delivery when the independent WebView was not ready.

## Decision

1. Topbar has a transparent host with no native acrylic, region, or system
   shadow. One WebView pill owns its background, edge, and shadow. The global
   glass preference updates this WebView-only treatment.
2. A native Bubble Controller owns the one 30-second direct-reply envelope. It
   sends only to a ready bubble-host generation and consumes only after render
   acknowledgement, successful placement, and no-activate show.
3. The Controller records read-only enqueue, ready, acknowledgement, placement,
   and show diagnostics. It excludes errors, cancellation, workflows, and tool
   process events.
4. Provider streaming consumes only explicitly public text deltas. It never
   infers or exposes hidden reasoning; no-delta Claude turns use an activity
   indicator until their final result.

## Consequences

Topbar no longer uses the native acrylic appearance, in exchange for one exact
visible pill boundary. Bubble delivery becomes retryable across WebView reloads
without duplicating confirmed displays. Automated tests can prove the protocol
but not Windows visuals, so both OpenSpec changes remain open and untagged until
the user completes mouse-driven acceptance.
