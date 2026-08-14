## 1. Settings and native acrylic

- [x] 1.1 Add `acrylic_opacity` (default 22) with the 5-100 clamp and the
  normalized alpha mapping; red-first tests for 22 -> 56, 100 -> 255, and the
  lower clamp.
- [x] 1.2 Add `set_acrylic_opacity` persisting the value, re-applying native
  acrylic to floats and pet, and emitting `settings:acrylic-changed` with
  enabled + opacity.
- [x] 1.3 Expose `acrylicOpacity` in bootstrap.

## 2. Settings UI and WebView layers

- [x] 2.1 Add the opacity range slider next to the 毛玻璃 switch.
- [x] 2.2 Map opacity to `--glass-opacity` in topbar pill, pet surface, and
  bubble backdrop; content cards stay opaque.
- [x] 2.3 Store round-trip tests and event payload coverage.

## 3. Evidence and acceptance

- [x] 3.1 Run all automated gates and rebuild.
- [x] 3.2 User mouse-driven Windows acceptance: slider changes every window's
  glass in real time, default equals the previous look, restart keeps the
  value, and readability of content cards is unchanged.
