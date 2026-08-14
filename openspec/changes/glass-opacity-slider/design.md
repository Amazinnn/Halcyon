## Context

SWCA acrylic accepts a tint alpha in `gradient_color`; the WebView surfaces
hard-code alpha in CSS. Both must follow one user value without changing the
default look.

## Decisions

### One normalized factor keeps the default identical

`opacity` is stored 5-100 (5 prevents the SWCA path degrading to plain
transparency). The current visual corresponds to 22 (float alpha 56/255).
`factor = opacity / 22`; each layer's alpha becomes
`round(base_alpha x factor)` clamped to [8, 255]. Base alphas: floats 56,
pet 64, topbar pill 0.84, pet surface 0.50, bubble 0.77.

### One event fans the change out to all windows

`settings:acrylic-changed` already exists for the toggle; it now carries
`opacity` too. Topbar already listens; pet and pet-bubble listen and write
`--glass-opacity` on `document.documentElement`, and their CSS switches to
`color-mix`-style alpha derived from that variable.

## Risks / Trade-offs

- [SWCA alpha floor] -> clamp 5-100; alpha below ~8 can make the glass
  invisible or fall back unexpectedly.
- [Per-window drift] -> base alphas differ per surface by design (pet tint is
  stronger); the factor keeps their relative look while all follow the slider.
- [Automated tests cannot prove glass] -> require the mouse-driven visual
  acceptance gate; keep the change unarchived until it passes.
