## Why

The top status capsule has a pill-shaped WebView border but rectangular native
acrylic behind it. Requirement #117 reports the resulting mismatched glass
shadow, which cannot be accepted as a visual-only CSS issue because the native
composition surface owns the rectangle.

## What Changes

- Remove topbar from every native acrylic, native region, and system-shadow
  path. Its transparent host keeps only no-activate, mouse-through, topmost,
  and show/move responsibilities.
- Render background, edge, and shadow inside one WebView pill layer, so the
  glass boundary cannot diverge from its capsule border.
- Keep the existing no-activate, mouse-through, topmost, and show/move paths;
  topbar still does not enter float-label, grid, tray, or drag lifecycle.
- Make the existing global acrylic setting control only this WebView glass
  treatment for topbar; it no longer requests native composition.
- Supersede the region approach in an ADR and
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
float hosts; this proposal scopes shared creation configuration plus one native
pill region to topbar only.
