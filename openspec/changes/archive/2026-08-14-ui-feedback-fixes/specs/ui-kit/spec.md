## ADDED Requirements

### Requirement: Declared font sizes always win
Kit controls SHALL declare font-family inheritance BEFORE font-size and SHALL NOT use the font shorthand after a font-size declaration.

#### Scenario: Toggle font size is the declared size
- **WHEN** a FocusToggle, FocusSegmented, or FocusSelect renders
- **THEN** its font-size equals the kit token (12px / 12px / 11px), not the inherited default

### Requirement: Compact selects in form rows have bounded width
A closed native select in a form row SHALL have an explicit bounded width so the row control, not the longest option, determines the width; the opened list may still size to its content.

#### Scenario: Skills select stays compact
- **WHEN** the chat composer renders the Skills select
- **THEN** the closed select is ~88px wide regardless of the longest skill name

### Requirement: Toggle rows show a group boundary
Settings rows that pair a text label with a toggle SHALL render a very light divider under the row so label and control read as one unit.

#### Scenario: Toggle row divider
- **WHEN** a settings toggle row renders
- **THEN** a light horizontal line separates it from the next row, with breathing room between text and line