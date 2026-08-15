# event-streams Specification

## Purpose
Groups core events into domains with a documented subscription matrix and lets light windows (topbar, pet-bubble, overlay) subscribe minimally instead of initializing the full Agent store.
## Requirements
### Requirement: Core events carry a domain
Every Rust CoreEvent SHALL map to a domain (focus, stats, agent, workflow, supervision, pet, music, probe, panel) via a domain() accessor.

#### Scenario: Domain lookup
- **WHEN** any core event is inspected
- **THEN** its domain() returns one of the documented domains

### Requirement: Subscription matrix is documented
A docs/architecture/event-streams-v1.md SHALL list every core event, its namespace, emitter, and current listeners.

#### Scenario: Matrix covers all events
- **WHEN** the event-streams doc is checked against event_bus.rs
- **THEN** every event variant appears in the matrix

### Requirement: Thin agent store mode for light windows
The frontend agent store SHALL support a thin init that only subscribes to agent:status (and queries status once); topbar/pet-bubble/grid-overlay SHALL use it, while other windows keep full initialization.

#### Scenario: Topbar stays thin
- **WHEN** the topbar window loads
- **THEN** it subscribes only to agent:status and does not initialize characters/sessions/workflow state

#### Scenario: Chat keeps full init
- **WHEN** the chat or desktop window loads
- **THEN** the agent store initializes fully as before

### Requirement: Event payloads and behavior are unchanged
Event names, payloads, and all existing window behavior SHALL remain identical.

#### Scenario: No regression
- **WHEN** the app runs after the change
- **THEN** existing automated gates pass and the manual checklist reports no behavior change

