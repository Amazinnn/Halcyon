## Purpose

Provide a bounded and explicitly enabled local trace for a real Windows pet
drag so a post-release Focus freeze can be attributed before any repair changes
the native-window lifecycle.

## ADDED Requirements

### Requirement: Opt-in post-release drag trace
The system SHALL collect no pet-drag diagnostic output unless explicitly
enabled. When enabled, it MUST write only local lifecycle metadata for the pet
drag and its first subsequent Focus click, without user message content,
provider data, screenshots, or telemetry upload.

#### Scenario: Diagnostics are disabled
- **WHEN** Focus starts without the diagnostic enablement
- **THEN** no pet-drag diagnostic file is written and normal drag behavior is
  unchanged

#### Scenario: Diagnostics are enabled
- **WHEN** a user drags and releases the pet, then next clicks Focus
- **THEN** the local trace contains ordered browser, release-owner, poller,
  overlay, geometry, placement, and first-click boundary records for that drag

### Requirement: A trace supports failure classification
The enabled trace SHALL identify the drag sequence, label, source, timestamp,
and active-drag presence at every captured boundary so a real reproduction can
distinguish an incomplete release, overlay input interception, native window
block, or another Focus main-thread block.

#### Scenario: Focus freezes after a release
- **WHEN** a user completes the documented diagnostic reproduction and Focus
  becomes unresponsive
- **THEN** the final recorded boundary is preserved for the incident Eval and
  no repair is declared from automated ownership evidence alone

### Requirement: Post-placement work does not re-enter settings ownership
After calculating and persisting a drag placement, the system MUST release the
settings mutex before moving the native window, repositioning the pet bubble,
or raising the topbar.

#### Scenario: Pet placement succeeds
- **WHEN** the pet is released over a free grid cell
- **THEN** the new rect is persisted, the settings mutex is released, and the
  window and bubble are positioned without blocking the Tauri main thread

#### Scenario: Pet placement is occupied
- **WHEN** the pet is released over an occupied grid cell
- **THEN** the previous rect is retained, the settings mutex is released, and
  the window and bubble snap back without blocking the Tauri main thread
