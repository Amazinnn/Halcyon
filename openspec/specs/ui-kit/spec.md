# ui-kit Specification

## Purpose
Defines one reusable Focus control kit and one design-token source so new windows and panels get consistent visuals without hand-written duplicated styles.
## Requirements
### Requirement: Design tokens are the single visual source
All colors, spacing, radii, motion, type scale, shadows, and control sizes SHALL live in styles.css :root variables; components SHALL NOT hard-code visual values.

#### Scenario: Component uses tokens
- **WHEN** a kit component renders
- **THEN** its colors, radii, and spacing all reference CSS variables

### Requirement: Kit controls replace duplicated hand-written styles
The desktop view tray, settings popover, workflow view, chat view, and float window headers SHALL use kit components instead of their own copies of button/toggle/segmented/input/slider/select/card/window-frame styles.

#### Scenario: Repeated control styles are gone
- **WHEN** the five target files are inspected
- **THEN** none of them defines its own switch/segmented/ghost/btn/text-input styles

### Requirement: Window header control behavior is preserved
FocusWindowFrame SHALL keep the pin and collapse behavior and props of the previous WindowHeader.

#### Scenario: Header actions unchanged
- **WHEN** a float window header pin/collapse button is clicked
- **THEN** the same native behavior runs as before the migration

### Requirement: Observable visuals stay unchanged
The refactor SHALL NOT alter any visible style: merged component styles must equal the current rendered styles.

#### Scenario: Visual regression guard
- **WHEN** the app is rebuilt after the migration
- **THEN** existing automated gates pass and the manual checklist reports no visual difference

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

### Requirement: Autosize inputs follow content within hard bounds
FocusInput SHALL support an autosize mode whose width follows the content between a minimum floor (--ctrl-min-input-auto) and 100% of the container; the input SHALL NEVER exceed its container.

#### Scenario: Short content shrinks the input
- **WHEN** an autosize input contains one or two characters
- **THEN** the input shrinks toward its floor instead of staying full width

#### Scenario: Long content never breaks the frame
- **WHEN** an autosize input reaches its container width
- **THEN** the input stops growing and further text is hidden inside the input, with no overflow beyond the container

### Requirement: Overflowing display text fades instead of hard-cutting
Long display text in constrained rows SHALL fade at the overflowing edge (mask gradient) rather than hard-wrap or overflow the frame.

#### Scenario: Agent name fades at the right edge
- **WHEN** an Agent name exceeds its row width
- **THEN** the tail of the name fades to transparent instead of breaking the row

### Requirement: Multi-line inputs have a bounded height
Chat composer and workflow textareas SHALL grow with content up to a bounded max height (~4 lines) and then scroll internally; the window frame is never broken.

#### Scenario: Composer scrolls beyond four lines
- **WHEN** the composer content exceeds the max height
- **THEN** the input area scrolls internally and the window stays the same size

### Requirement: Composer controls share one height
The Skills select and the send/stop buttons SHALL align to the input box height in the chat composer.

#### Scenario: Aligned composer row
- **WHEN** the chat composer renders
- **THEN** Skills select, input box, and buttons share the same height baseline

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
