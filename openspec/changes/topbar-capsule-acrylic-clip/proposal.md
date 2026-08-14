## Why

The top status capsule has a pill-shaped WebView border but rectangular native
acrylic behind it. Requirement #117 reports the resulting mismatched glass
shadow, which cannot be accepted as a visual-only CSS issue because the native
composition surface owns the rectangle.

## What Changes

- Create a topbar-only native pill region once during hidden window creation,
  using its actual client-pixel dimensions and a radius equal to half its client
  height, so native acrylic is clipped to the capsule boundary.
- Keep the existing no-activate, mouse-through, topmost, and show/move paths;
  do not apply float-host frame configuration or grid behavior to topbar.
- Make the existing global acrylic setting update topbar as well as the other
  acrylic-backed hosts.
- Record the intentional topbar-only exception to ADR-0029 in a new ADR and
  require user mouse-driven Windows acceptance before this change is archived.

## Capabilities

### New Capabilities

- `topbar-capsule-acrylic`: defines the native acrylic clipping and global
  acrylic-toggle behavior for the top status capsule.

### Modified Capabilities

- None.

## Impact

This affects topbar creation and global acrylic updates in
`apps/desktop/src-tauri/src/lib.rs`, plus focused Rust coverage and the
topbar-related Eval/incident/ADR records. It deliberately leaves the accepted
float hosts, pet-bubble host, grid overlay, tray behavior, desktop locking,
providers, and workflows unchanged. ADR-0029 remains the governing policy for
float hosts; this proposal scopes one native region to topbar only.
