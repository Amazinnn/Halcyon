## ADDED Requirements

### Requirement: Inputs and selects keep a usable width
FocusInput and FocusSelect SHALL NOT shrink below their token minimum widths (--ctrl-min-input / --ctrl-min-select) in narrow flex rows.

#### Scenario: Input keeps a usable width
- **WHEN** a FocusInput or FocusSelect renders in a narrow flex row
- **THEN** it never shrinks below its token minimum width

### Requirement: Text-bearing flex items keep readable width
Multi-line-capable text items in flex rows SHALL have a minimum width or ellipsis/fade protection; they SHALL NOT use bare min-width: 0.

#### Scenario: Agent names stay horizontal
- **WHEN** an Agent name renders in the settings Agent row
- **THEN** it gets at least the text-min-row width and wraps only when the window is genuinely too narrow

### Requirement: Overflow-prone flex rows wrap
Flex rows that can overflow their container SHALL use flex-wrap with row-gap.

#### Scenario: Agent management row wraps
- **WHEN** the settings Agent management row cannot fit its buttons
- **THEN** the buttons wrap to the next line instead of overflowing the frame