## Why

User feedback (requirement #132): the chat Skills select is far wider than the message input (longest option stretches it); four toggle/segmented controls render at the browser-default font size because `font: inherit` (a shorthand) overrides `font-size` when declared after it; the settings popover spacing is cramped and toggle rows lack a visual group boundary.

## What Changes

- Fix font declaration order in FocusToggle/FocusSegmented/FocusSelect: font-family: inherit before font-size (no shorthand override), restoring 12px toggles/segments and 11px selects.
- Chat Skills select fixed at 88px (list popup unaffected).
- Settings popover: inter-group spacing widened (gap 12px, group padding-top 10px); the five toggle rows gain a very light bottom divider (.toggle-row) so the "label + button" pair reads as one row group.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `ui-kit`: font contract (explicit font-size after inheritance, no shorthand override) and row-divider utility join the kit rules.

## Impact

FocusToggle/FocusSegmented/FocusSelect styles, ChatView skills width, SettingsPopover spacing + toggle rows, kit tests, ui-design.md rules.