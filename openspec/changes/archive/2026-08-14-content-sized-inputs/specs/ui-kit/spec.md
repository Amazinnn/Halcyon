## ADDED Requirements

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