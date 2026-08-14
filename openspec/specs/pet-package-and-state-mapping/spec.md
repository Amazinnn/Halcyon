# Pet Package And State Mapping Specification

## Purpose

Allow each Focus Agent to use a validated package without exposing Provider or
transient runtime states to the pet mapping interface.

## Requirements

### Requirement: A current Agent without a readable package has no pet host
The system SHALL hide the native pet host and companion when the current Agent
has no database-declared, readable package. It MUST NOT leave an empty box or
transparent drag surface on the desktop.

#### Scenario: Current Agent has no package
- **WHEN** the current Agent has no package, its package is removed, or its
  package cannot be read
- **THEN** the pet host and companion are hidden

### Requirement: Pet drag release is finalized once
The system SHALL let the browser release and native cursor poller compete for
one active-drag ownership token. Only its first claimant may stop and finalize
the drag.

#### Scenario: Browser and poller report the same release
- **WHEN** `pointerup` and native button-up both report a release for a pet
  drag
- **THEN** exactly one path finalizes the pet and the next drag starts from an
  empty active-drag state

### Requirement: Package adapters are explicit in Phase 2
The system SHALL later accept the official Hatch Pet package unchanged and a
Focus package whose manifest has `format: "focus-hatch-pet"`. It MUST discover
declared animation assets rather than relying on fixed file names.

#### Scenario: Unsupported package JSON
- **WHEN** a package does not match either explicit format
- **THEN** import rejects it with a format-specific error

### Requirement: Package rendering preserves aspect ratio
The system SHALL render a validated pet proportionally inside the icon safety
margin, maximizing its occupied area without crop or non-uniform scaling.

#### Scenario: A non-square source is rendered into the pet icon
- **WHEN** a validated animation's source aspect ratio differs from the host
  icon aspect ratio
- **THEN** the rendered image preserves the source ratio and leaves only the
  configured safety margin rather than stretching or cropping the sprite

#### Scenario: The pet host changes grid size
- **WHEN** the pet host changes between supported grid dimensions after its
  package has loaded
- **THEN** the canvas recomputes both its CSS dimensions and device-pixel-ratio
  backing dimensions from the stable stage, while retaining the calibrated
  aspect ratio and safety margin

### Requirement: Display calibration isolates stable visible content
The system SHALL derive a stable per-animation source rectangle from all used
frames without modifying the source package. It SHALL ignore sparse disconnected
artifacts when confidence is sufficient and SHALL fall back to the full cell
when calibration is ambiguous.

#### Scenario: A generated frame contains a thin horizontal streak
- **WHEN** the streak is disconnected or removed by the calibrated alpha mask
- **THEN** display sampling excludes it and import reports a non-blocking quality
  warning identifying the affected animation and frame

#### Scenario: Calibration cannot preserve enough visible content
- **WHEN** the retained alpha mass falls below the safe threshold
- **THEN** the animation uses its complete declared cell and no destructive crop
  is stored

#### Scenario: Excluded artifacts have a different dominant color
- **WHEN** pixels rejected by calibration form a larger color cluster than the
  retained subject
- **THEN** host and companion colors use only retained calibrated subject pixels

### Requirement: Package display correction is explicit and scoped
The system SHALL store horizontal correction per Agent and current package with
a default of `1.00`, a range of `0.75` through `1.33`, and a reset action.
Replacing the package SHALL reset the correction.

#### Scenario: A user corrects generated artwork that appears too wide
- **WHEN** the user selects a narrower horizontal correction
- **THEN** the corrected display is maximized with contain geometry inside the
  safe inset without changing the source image or exceeding the pet host

#### Scenario: A replacement cannot be staged and validated
- **WHEN** copy, parsing, analysis, or display-metadata generation fails
- **THEN** the previously imported package remains intact and runnable

### Requirement: Pet host color follows package content
The system SHALL derive a representative color from calibrated visible pixels
and use a low-saturation dark variant for only the pet host background. The
companion SHALL use a lighter accent from the same source.

#### Scenario: A blue pet package becomes current
- **WHEN** its package analysis succeeds
- **THEN** the pet host uses a restrained blue-derived tint instead of the global
  dark-green float tint, while other float hosts remain unchanged

### Requirement: Pet WebView preserves the native acrylic through its derived tint
The pet WebView SHALL apply the package-derived host tint at 50% opacity. The
existing global acrylic setting SHALL remain its native-acrylic authority.

#### Scenario: Global acrylic is enabled for a package-backed pet
- **WHEN** the current package supplies a derived host tint
- **THEN** the tinted pet WebView remains semi-transparent and the native
  acrylic is visible through it
