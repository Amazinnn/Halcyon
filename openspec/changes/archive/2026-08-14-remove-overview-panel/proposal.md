## Why

The C4 overview panel duplicates the stats window (today summary + recent runs are subsets of stats). The user asked why it exists at all (requirement #129) and decided to remove it; the panel recipe documentation stays.

## What Changes

- Remove the `overview` window declaration (window_spec.rs), its ViewRegistry entry and view component, and its capability list entry.
- Revert the float-set test to five floats and the tray test to four entries.
- ui-maintenance.md §3 keeps the panel recipe as pure text steps (declaration + read-only query + event subscription + kit assembly).

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `panel-window-framework`: the example panel requirement is removed; the recipe remains documented.

## Impact

window_spec.rs, view-registry.ts (+ tests), capabilities/default.json, OverviewPanelView.vue deletion, ui-maintenance.md. No behavior change to other windows.