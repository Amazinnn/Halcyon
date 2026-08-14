## Purpose

Keep settings concise while documenting package ownership and Focus-only state
mapping when those later phases become available.

## ADDED Requirements

### Requirement: Package guidance follows explicit formats
The settings surface SHALL explain that an Agent owns at most one optional pet
package and SHALL link users to the README for the official Hatch Pet and
`focus-hatch-pet` package contracts. It MUST not claim that the retired
`hatch-pet-draft-0.2` format is supported.

#### Scenario: A user reads pet import help
- **WHEN** the user opens the relevant settings guidance
- **THEN** it identifies the official Hatch Pet and planned `focus-hatch-pet`
  contracts without presenting the retired draft as importable

### Requirement: State mapping waits for package discovery
The settings surface SHALL only expose mapping choices after a valid package
has been imported and its animations have been discovered. The choices SHALL
be Focus-owned continuous states, never Provider-native or transient states.

#### Scenario: An Agent has no valid package
- **WHEN** the selected Agent has no readable package
- **THEN** no animation mapping selector is shown

### Requirement: Phase 1 does not expand settings behavior
During the pet-drag stability phase, settings behavior SHALL remain unchanged
except that removing or switching away from a current Agent package hides the
native pet host.

#### Scenario: A current package is removed in Phase 1
- **WHEN** the user removes the current Agent's package
- **THEN** settings completes the removal and the native pet host disappears
  without adding new package or state controls

### Requirement: Package display controls stay with the selected Agent
When the current Agent has a readable package, settings SHALL show its import
quality warnings and a compact horizontal-correction control with current value
and reset. It MUST hide these controls when no package is available.

#### Scenario: A package with sparse artifacts is imported
- **WHEN** analysis produces warnings
- **THEN** settings explains that the source package was not modified and lists
  the affected animation frames

### Requirement: Restricted focus actions use the active round mode
The system SHALL freeze the selected focus mode when a work round starts.
Standard and scholar rounds MUST reject application exit; scholar rounds MUST
also reject pause and skip at the action boundary, including CLI-triggered
actions. Changing the saved mode only affects the next round.

#### Scenario: Settings change during a scholar round
- **WHEN** a scholar work round is active and the saved next-round mode changes
- **THEN** exit, pause, and skip remain unavailable for the active round

### Requirement: Global direct-chat streaming display preference
The settings surface SHALL provide one global persisted "显示流式输出" preference
that defaults to disabled. It SHALL apply to the next direct Codex or Claude
conversation, regardless of the selected Agent, and survive Focus restart.

#### Scenario: Streaming preference is disabled
- **WHEN** a direct Provider response produces public text deltas
- **THEN** chat hides the deltas and displays the final reply only

#### Scenario: Streaming preference is enabled
- **WHEN** the next direct Codex or Claude response produces public text deltas
- **THEN** chat displays those public deltas before the final reply

#### Scenario: Provider reasoning or tool activity occurs
- **WHEN** a direct conversation produces hidden reasoning, a workflow event,
  or tool activity
- **THEN** the preference MUST NOT expose it as streaming text
