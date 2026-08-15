## MODIFIED Requirements

### Requirement: Kit controls replace duplicated hand-written styles
Kit controls SHALL set font-size after any font inheritance declaration so the declared size always wins.

#### Scenario: Toggle font size is the declared size
- **WHEN** a FocusToggle, FocusSegmented, or FocusSelect renders
- **THEN** its font-size equals the kit token (12px / 12px / 11px), not the inherited default

### Requirement: Overflow-prone flex rows wrap
A compact select in a form row SHALL have a bounded width so the row control (not the longest option) determines the width; the opened list may still size to its content.

#### Scenario: Skills select stays compact
- **WHEN** the chat composer renders the Skills select
- **THEN** the closed select is ~88px wide regardless of the longest skill name

### Requirement: Toggle rows show a group boundary
Settings rows that pair a text label with a toggle SHALL render a very light divider under the row so label and control read as one unit.

#### Scenario: Toggle row divider
- **WHEN** a settings toggle row renders
- **THEN** a light horizontal line separates it from the next row, with breathing room between text and line