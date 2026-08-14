# ADR-0036: Provider Thinking Streaming And Resident Assistant Delta Source

Status: Accepted, Windows visual verification pending
Date: 2026-08-14
Requirements: #116, #120, #121, #122
OpenSpec: `pet-state-pack-and-settings`, `topbar-capsule-acrylic-clip`
Amends: ADR-0035 point 4 (streaming display policy)

## Context

Live CDP instrumentation proved two production failures that unit tests and
one-shot CLI probes could not reveal.

1. The independent pet-bubble WebView was never in the Tauri capability window
   list, so every `plugin:event|listen` call was rejected and the host never
   reported readiness; the Bubble Controller therefore never dispatched. This
   is the verified root cause of the bubble never appearing since #115.
2. In resident mode (stdin kept open), Claude Code does not emit
   `content_block_delta` stream events. Its only per-turn increments are
   partial `assistant` messages whose `thinking` blocks contain raw
   newlines (multi-line JSON) and whose `text` blocks carry cumulative
   content. The adapter parsed only `content_block_delta` and only line by
   line, so resident-mode deltas were always zero.

The user explicitly approved showing the Provider-visible thinking process in
chat while the streaming switch is on (requirement #122), superseding the
no-hidden-reasoning boundary recorded in ADR-0035 point 4.

## Decision

1. The capability ACL is the bubble host's delivery gate: `pet-bubble` is
   added to the default capability window list, matching every other Focus
   window.
2. Claude resident output is parsed as an accumulating JSON document (one
   logical message may span physical lines) and consumes `assistant`
   messages as the incremental source:
   - `thinking` content blocks stream as a new `message.thinking` event,
     cumulative-length diffed;
   - `text` content blocks stream as `message.delta`, cumulative-length
     diffed;
   - `tool_use` content blocks restore `tool.started` visibility in
     resident mode.
3. `message.thinking` is added to the AgentEvent v1 schema as an additive
   event kind (schemaVersion stays 1). Codex keeps its existing
   `item/agentMessage/delta` public-text path; thinking display is Claude-
   only and gated by the same streaming switch as text deltas.
4. Chat renders the thinking stream as a distinct muted block above the answer
   text and keeps it with the completed message. It never enters the pet
   bubble, workflow results, or persisted Provider sessions.
5. Topbar keeps its single-WebView pill surface; the transparent host gains a
   shadow margin so the WebView-owned shadow renders fully inside the window
   and follows the pill curve exactly (requirement #121).

## Consequences

Streaming becomes observable for resident Claude turns (thinking plus public
text), and the bubble host becomes a live delivery endpoint. The schema grows
one additive event kind; older frontends ignore unknown kinds. Windows visual
acceptance remains the only gate for both OpenSpec changes.
