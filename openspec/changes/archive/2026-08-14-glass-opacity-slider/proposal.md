## Why

The global acrylic toggle is binary; every glass layer (native SWCA tint,
topbar pill, pet surface, bubble backdrop) hard-codes its own alpha, so the
user cannot tune how much desktop shows through. Requirement #123 asks for one
slider in Settings that affects all windows at once.

## What Changes

- Add a persisted `acrylic_opacity` (0-100) setting; the current look maps to
  22. Every glass layer derives alpha = round(base_alpha x opacity/22),
  clamped to [8, 255], so the default equals today's visuals exactly.
- `set_acrylic_opacity` persists, re-applies native acrylic to
  chat/stats/music/workflow/pet, and emits `settings:acrylic-changed`
  carrying `{enabled, opacity}`; each window maps opacity onto its CSS glass
  layer through one `--glass-opacity` variable.
- Settings > 外观 shows a range slider next to the existing 毛玻璃 switch.
- Content cards (messages, panels) stay opaque; only glass layers follow.

## Capabilities

### New Capabilities

- `global-glass-opacity`: one global opacity value shared by every window's
  native and WebView glass layers.

### Modified Capabilities

- None.

## Impact

Affects settings persistence/bootstrap, native acrylic application, the
topbar/pet/bubble WebView surfaces, and the Settings UI. Desktop lock, grid,
tray, providers, workflows, and bubble delivery are unchanged. Windows visual
acceptance is required before archiving.
