# Global Glass Opacity Specification

## Purpose

Give the user one global slider that tunes how much desktop shows through every
Focus glass layer (native acrylic tint and the WebView translucent surfaces)
without changing the default look or content-card readability.

## Requirements

### Requirement: Global glass opacity setting
The system SHALL persist one global opacity value in the range 5..100, where 22
reproduces the current visual exactly. Every glass layer's alpha SHALL be
`round(base_alpha x opacity / 22)` clamped to 8..255, with the same factor
applied to native SWCA tint alphas and WebView surface alphas.

#### Scenario: Default value preserves the current look
- **WHEN** Focus starts with the default opacity (22)
- **THEN** every glass layer renders with its historical alpha (floats 56,
  pet 64, topbar pill 0.84, pet surface 0.50, bubble backdrop 0.77)

#### Scenario: Slider moves to a more transparent value
- **WHEN** the user lowers the opacity below 22
- **THEN** all glass layers become proportionally more transparent and the
  desktop shows through more

#### Scenario: Slider moves to a more solid value
- **WHEN** the user raises the opacity above 22
- **THEN** all glass layers become proportionally more solid (darker/more
  frosted) and the desktop shows through less

### Requirement: Opacity applies to every window at once
The system SHALL re-apply the changed opacity to the native acrylic of the
chat, stats, music, workflow, and pet hosts and SHALL notify every WebView
through `settings:acrylic-changed` carrying `enabled` and `opacity` so the
topbar pill, pet surface, and bubble backdrop update immediately.

#### Scenario: Opacity changes while Focus is running
- **WHEN** the user drags the opacity slider
- **THEN** every visible glass surface updates without restarting Focus

#### Scenario: Focus restarts
- **WHEN** Focus starts again
- **THEN** the persisted opacity value is restored and applied to all windows

### Requirement: Content cards stay readable
The system SHALL keep opaque content surfaces (chat messages, settings panels)
independent of the opacity value; only glass layers follow the slider.

#### Scenario: Opacity is set to its most solid value
- **WHEN** the user sets opacity to 100
- **THEN** glass surfaces become near-opaque while message and panel content
  remains unchanged and readable
