## Purpose

Present complete final Agent messages beside a valid current pet without
clipping, input capture, or entry into the grid/tray lifecycle.

## ADDED Requirements

### Requirement: Companion visibility follows a valid pet
The system SHALL hide the companion whenever there is no current readable pet
package. It MUST not become an independent empty window.

#### Scenario: The current Agent package is absent
- **WHEN** the current Agent has no readable package or its package is removed
- **THEN** the companion is hidden with the pet host

### Requirement: Successful final replies use one dedicated companion window
The system SHALL display every successful final direct Agent reply in one
independent transparent, non-activating, mouse-through companion host whether
chat is open or closed. Failed and cancelled turns MUST NOT create a bubble.

#### Scenario: Chat is open when a direct reply completes
- **WHEN** Codex or Claude produces a non-empty successful final reply
- **THEN** exactly one targeted bubble replaces any current bubble while the
  chat message remains visible

#### Scenario: Chat completion and the authoritative bubble event both arrive
- **WHEN** one Provider reply produces its normal chat completion followed by
  `bubble:requested`
- **THEN** only the authoritative event starts one companion playback lifecycle

#### Scenario: A Provider turn fails
- **WHEN** the turn ends in error or cancellation
- **THEN** the error remains in chat and no companion message is emitted

### Requirement: Companion placement avoids owned windows
The companion SHALL prefer positions above the pet, MUST never overlap the pet,
and SHOULD avoid the visible chat window. It SHALL stay inside the work area and
hide if no pet-safe candidate exists.

#### Scenario: The centered-above position intersects chat
- **WHEN** another pet-safe candidate avoids chat
- **THEN** the first such candidate in the defined placement order is selected

#### Scenario: Every pet-safe candidate intersects chat
- **WHEN** no candidate can avoid chat completely
- **THEN** the candidate with the smallest chat overlap is used

### Requirement: Companion playback remains complete and stable
The companion SHALL paginate complete text into two measured lines, rotate each
page every three seconds, fade between pages, and immediately restart from page
one when a new message replaces the current message.

#### Scenario: A long final message is available in Phase 3
- **WHEN** the current pet receives a final message that needs more than two
  measured lines
- **THEN** the companion displays every complete page in rotation without
  capturing desktop mouse input

### Requirement: Pet dragging temporarily suppresses the companion
The system SHALL hide the companion when pet dragging begins and SHALL only
reposition and fade it in after snapping finishes outside persistence locks.

#### Scenario: The pet moves while a message is active
- **WHEN** the drag finalizes successfully
- **THEN** the active companion reappears at a newly calculated pet-safe position

#### Scenario: A new reply arrives during dragging
- **WHEN** `bubble:requested` arrives before pet snap finalization
- **THEN** its message replaces the old one but the companion remains hidden
  until `pet:drag-ended`
### Requirement: Reliable direct-reply delivery
The system MUST display a successful direct Agent reply in the pet bubble even
when the bubble WebView initializes after the provider event. The core MUST keep
at most one latest in-memory delivery for the current Agent, expire it after 30
seconds, and remove it after one claim. Switching or deleting the current Agent
MUST clear it; a Focus restart MUST NOT restore it.

#### Scenario: late bubble window initialization
- **WHEN** a successful direct reply is emitted before the pet-bubble WebView
  has restored its Agent identity
- **THEN** its local delivery endpoint listens, restores identity from bootstrap,
  claims and displays the reply exactly once with the original delivery id

#### Scenario: unrelated or expired delivery
- **WHEN** a delivery targets another Agent or is older than 30 seconds
- **THEN** the bubble window MUST NOT display it

#### Scenario: bubble WebView restores the same current Agent
- **WHEN** the bubble WebView persists the Agent ID already current in Focus
  before claiming a matching pending delivery
- **THEN** the delivery remains claimable and displays exactly once

#### Scenario: current Agent actually changes
- **WHEN** the persisted current Agent ID changes to another Agent or the
  current Agent is deleted
- **THEN** any pending delivery for the previous Agent is cleared
