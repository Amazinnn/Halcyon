## Purpose

Establishes one declarative window registry as the single source of truth for every Focus Desktop WebView window, so adding a window means declaring it in the registry and its frontend view instead of editing scattered creation and mapping logic.

## ADDED Requirements

### Requirement: All windows are declared in one registry
Focus Desktop SHALL define every WebView window (desktop, chat, stats, music, pet, pet-bubble, workflow, grid-overlay, topbar) in a single static window registry, and window creation SHALL be driven by that registry rather than by per-window creation code.

#### Scenario: Registry contains every window
- **WHEN** the application starts
- **THEN** a window is created for every registry entry, and no WebView window is created that is not declared in the registry

#### Scenario: Registry entries are unique
- **WHEN** the registry is validated
- **THEN** every window label appears exactly once

### Requirement: Float window lifecycle derives from the registry
The set of grid float windows (chat, stats, music, pet, workflow) SHALL derive from the registry, and every place that enumerates floats (initial layout, placement, collapse/restore handling) SHALL use that derived set.

#### Scenario: Float set matches the registry
- **WHEN** the float set is derived from the registry
- **THEN** it contains exactly the registry entries whose kind is float, in registry order

#### Scenario: Non-float windows are excluded
- **WHEN** a non-float label (desktop, pet-bubble, grid-overlay, topbar) is checked against the float set
- **THEN** it is not treated as a float window

### Requirement: Default layout comes from the registry
Each float window's default grid placement SHALL come from its registry declaration; no per-window default special case SHALL exist in layout code.

#### Scenario: Default placement without saved layout
- **WHEN** a float window has no saved grid position
- **THEN** it is placed using its registry-declared default rect

### Requirement: Frontend views resolve from a view registry
The frontend SHALL resolve each window label to its view component, its transparent-window styling, and (for float windows) its tray entry through a single view registry; the window shell and the desktop view tray SHALL consume that registry instead of hard-coded mappings.

#### Scenario: Unknown label falls back safely
- **WHEN** the frontend starts in a window whose label is not in the view registry
- **THEN** it renders the desktop view

#### Scenario: Tray shows every float view
- **WHEN** the desktop view tray is expanded
- **THEN** it shows one entry per float window in the registry, with the declared title and icon, and opening an entry restores that window

### Requirement: Capability list stays consistent with the registry
The Tauri capability window list SHALL exactly match the set of registry labels, and automated tests SHALL enforce that consistency.

#### Scenario: Registry and capability list diverge
- **WHEN** a registry label is missing from (or extra in) the capability window list
- **THEN** the automated test suite fails, preventing a window from silently losing its permissions

### Requirement: Refactor preserves observable window behavior
This registry change SHALL NOT alter any observable window behavior: creation order, window flags, initial visibility, acrylic glass application, bubble/topbar/overlay behavior, and float placement semantics remain as before the change.

#### Scenario: Behavior regression guard
- **WHEN** the application is rebuilt after this change
- **THEN** existing automated gates pass and a manual Windows acceptance checklist reports no regression in window behavior
