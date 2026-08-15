## Why

Topbar initializes the full Agent store just to show a status dot, and event consumers have no documented subscription matrix. Requirement #127 (multi-window event streams) needs event domains and thin-window subscriptions so new windows know what to listen to and light windows stay light.

## What Changes

- Add a thin mode to the frontend agent store: only the `agent:status` listener plus one status query, skipping character/session/workflow initialization.
- App.vue initializes the agent store by window: topbar/pet-bubble/grid-overlay use thin mode; other windows keep the full init.
- Add a domain() grouping to Rust CoreEvent and document the event subscription matrix (docs/architecture/event-streams-v1.md): who emits, who listens, per-domain namespaces.
- No event payload or naming changes; no behavior change for chat/pet/workflow windows.

## Capabilities

### New Capabilities

- `event-streams`: domain grouping of core events plus a documented subscription matrix and thin-window subscription mode.

### Modified Capabilities

- None.

## Impact

event_bus.rs (domain grouping), App.vue and stores/agent.ts (thin mode), docs. AgentEvent schema untouched.