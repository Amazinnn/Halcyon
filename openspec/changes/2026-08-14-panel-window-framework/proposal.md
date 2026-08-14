## Why

The user-selected future scenario is custom panel windows (e.g. an app-usage dashboard). Requirement #127 wants "new window = declaration + assembly". The window registry (ADR-0037), UI kit, event streams, and CLI registry now exist; this change proves and documents the panel assembly path with one minimal example panel.

## What Changes

- Add one example panel window `overview`: today focus summary + recent workflow runs, assembled from the view registry (declaration), a read-only query, event subscriptions (focus:tick, workflow:changed), and UI kit components.
- Document the panel recipe in docs/ui-maintenance.md (panel section): window registry entry + ViewRegistry entry + capability list + query command + event subscription + kit assembly.
- No changes to existing windows or behaviors; the new window is additive.

## Capabilities

### New Capabilities

- `panel-window-framework`: the pattern and example for adding custom panel windows by declaration and assembly.

### Modified Capabilities

- None.

## Impact

window_spec.rs (one entry), view-registry.ts (one entry), capabilities/default.json (one label), a new OverviewPanel view, docs. Everything else untouched.