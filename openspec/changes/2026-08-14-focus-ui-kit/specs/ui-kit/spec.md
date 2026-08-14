## Purpose

Defines one reusable Focus control kit and one design-token source so new windows and panels get consistent visuals without hand-written duplicated styles.

## ADDED Requirements

### Requirement: Design tokens are the single visual source
All colors, spacing, radii, motion, type scale, shadows, and control sizes SHALL live in styles.css :root variables; components SHALL NOT hard-code visual values.

#### Scenario: Component uses tokens
- **WHEN** a kit component renders
- **THEN** its colors, radii, and spacing all reference CSS variables

### Requirement: Kit controls replace duplicated hand-written styles
The desktop view tray, settings popover, workflow view, chat view, and float window headers SHALL use kit components instead of their own copies of button/toggle/segmented/input/slider/select/card/window-frame styles.

#### Scenario: Repeated control styles are gone
- **WHEN** the five target files are inspected
- **THEN** none of them defines its own switch/segmented/ghost/btn/text-input styles

### Requirement: Window header control behavior is preserved
FocusWindowFrame SHALL keep the pin and collapse behavior and props of the previous WindowHeader.

#### Scenario: Header actions unchanged
- **WHEN** a float window header pin/collapse button is clicked
- **THEN** the same native behavior runs as before the migration

### Requirement: Observable visuals stay unchanged
The refactor SHALL NOT alter any visible style: merged component styles must equal the current rendered styles.

#### Scenario: Visual regression guard
- **WHEN** the app is rebuilt after the migration
- **THEN** existing automated gates pass and the manual checklist reports no visual difference