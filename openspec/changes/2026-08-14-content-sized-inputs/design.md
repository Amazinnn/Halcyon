## Context

See proposal - Why. Chromium field-sizing: content gives native content-width sizing for inputs (progressive enhancement: ignored by older engines, falling back to current widths). mask-image gradients give edge fades for display text. The window grid already prevents sibling squeezing; the new hard rule is that no control may exceed its container (max-width: 100%) so the outer frame is never broken.

## Goals / Non-Goals

**Goals:** autosize inputs with floors and caps; fade-overflow utility; bounded multi-line growth; composer alignment; ui-design rules.

**Non-Goals:** no window-level resizing (pet-bubble-style hosts stay special); no changes to other controls; no ellipsis replacement outside the named spots.

## Decisions

1. FocusInput autosize: field-sizing: content + min-width 40px + max-width 100%. Non-autosize behavior unchanged.
2. Fade utilities .fade-x/.fade-y in styles.css (mask-image gradients); applied to pack-name and run-name only for now.
3. Composer/textarea: max-height var(--ctrl-max-input-h) (~4 lines) + overflow-y auto.
4. Composer alignment: Skills select align-self: stretch with min-height 36px; buttons height 36px.
5. autosize applied to Agent name and URL name inputs (short content).
6. ui-design.md documents the three rules: content sizing, hard bounds, overflow handling (input = native hide, display text = fade, multi-line = internal scroll).

## Risks / Trade-offs

- field-sizing/mask unsupported on old WebView2 → progressive fallback to current behavior; verified in the manual checklist on the real release.
- Fade on pack-name/run-name is a visual change → single-file revert possible; compared in manual checklist.