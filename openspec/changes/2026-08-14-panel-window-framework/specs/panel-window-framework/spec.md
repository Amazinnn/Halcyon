## Purpose

Establishes the declarative recipe for custom panel windows and proves it with one minimal overview panel, so future panels are added by declaration and assembly.

## ADDED Requirements

### Requirement: Panel windows are declared like every window
A panel window SHALL be a normal window registry entry (Rust WindowSpec), a ViewRegistry entry (frontend), and a capability list entry, with no changes to window creation logic.

#### Scenario: New panel by declaration
- **WHEN** the overview panel is added
- **THEN** it is created through the existing registry-driven create_windows and rendered through the view registry

### Requirement: Example overview panel exists
The overview panel SHALL show the today focus summary and recent workflow runs, updating live from focus:tick and workflow:changed events, and SHALL be opened/closed via the normal float restore/collapse path.

#### Scenario: Panel shows live data
- **WHEN** the overview panel is open and a focus tick or workflow change occurs
- **THEN** its displayed values update without a reload

#### Scenario: Panel uses normal window lifecycle
- **WHEN** the overview panel is restored or collapsed
- **THEN** it behaves like any other float window (grid placement, no overlap, no activation)

### Requirement: Panel recipe is documented
docs/ui-maintenance.md SHALL contain a panel recipe section listing the exact declarations and steps for adding a panel window.

#### Scenario: Recipe completeness
- **WHEN** the recipe is followed
- **THEN** a new panel can be added without editing existing creation, mapping, or tray logic

### Requirement: Existing behavior is unchanged
The new window SHALL NOT alter any existing window, event, or command behavior.

#### Scenario: Regression guard
- **WHEN** the app runs with the overview panel added
- **THEN** existing gates pass and the manual checklist shows no regression in other windows