## Why

UI layout feedback (requirement #130): some text/buttons overflow their frames, small components (text inputs) get squeezed far too narrow, and many text blocks collapse into narrow columns because flex items have no minimum width. Root causes: the Agent row in SettingsPopover packs six items into a 300px popover without wrapping; FocusInput/FocusSelect use min-width: 0 and can be crushed; text-bearing flex items (e.g. pack-name) have no minimum width so names render vertically.

## What Changes

- Add layout tokens (--ctrl-min-input 96px, --ctrl-min-select 88px, --text-min-row 120px) to styles.css :root.
- FocusInput and FocusSelect apply their minimum-width tokens instead of min-width: 0.
- SettingsPopover: Agent create row becomes two lines (name input gets full width; provider select + add button below); pack-row wraps (flex-wrap + row-gap) and pack-name gets a 120px minimum so names stay horizontal.
- Audit remaining non-wrapping flex rows for real overflow; wrap only where needed.
- ui-design.md gains a layout & text-width section: text-bearing flex items must have a minimum width or ellipsis protection; flex rows that can overflow must wrap; input/select minimum widths are mandatory. ui-maintenance.md notes the tokens for new controls.
- No visual change outside the repaired spots.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `ui-kit`: minimum-width tokens and layout rules are added to the kit contract.

## Impact

styles.css tokens, FocusInput/FocusSelect, SettingsPopover layout, ui-design/ui-maintenance docs, kit tests.