## Purpose

Keep the top status capsule's Windows acrylic composition inside the same
pill-shaped boundary that users see and allow the global glass preference to
control that composition consistently.

## ADDED Requirements

### Requirement: Topbar acrylic follows the capsule boundary
The system SHALL first reuse the accepted hidden float-host creation
configuration, then clip native acrylic for the top status capsule to a pill
whose corner radius equals half of that window's actual client-pixel height.
The native clip SHALL be established once while topbar is hidden before it is
first shown.

#### Scenario: Topbar is first shown
- **WHEN** Focus creates and then displays the top status capsule
- **THEN** its native acrylic boundary is pill-shaped and matches the visible
  capsule instead of appearing as a rectangle

#### Scenario: Topbar is subsequently moved or shown again
- **WHEN** Focus repositions, hides, or shows the existing top status capsule
- **THEN** it keeps the established pill clip without reconfiguring the native
  window or changing activation behavior

### Requirement: Topbar honors the global acrylic preference
The system SHALL update the top status capsule when the existing global acrylic
preference changes, using the same enabled/disabled state as other acrylic-backed
Focus hosts.

#### Scenario: Global acrylic is disabled
- **WHEN** the user disables the global acrylic preference
- **THEN** the top status capsule disables its native acrylic composition

#### Scenario: Global acrylic is re-enabled
- **WHEN** the user re-enables the global acrylic preference
- **THEN** the top status capsule restores native acrylic while preserving its
  pill-shaped boundary

### Requirement: Topbar keeps its established interaction contract
The system SHALL keep the top status capsule non-activating, mouse-through, and
topmost. It MAY reuse only the accepted creation-time float-host setup; it MUST
NOT join the float-label/grid, tray, desktop-lock, Provider, or workflow
lifecycles.

#### Scenario: Pointer passes through a visible topbar
- **WHEN** a user clicks or drags at the visible top status capsule
- **THEN** it does not activate Focus or intercept the underlying desktop input
