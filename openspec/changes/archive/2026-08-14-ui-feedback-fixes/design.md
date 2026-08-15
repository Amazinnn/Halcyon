## Context

See proposal - Why. CSS `font` shorthand resets font-size; moving font-family: inherit before font-size fixes the regression without changing intended inheritance. Select width: a closed native select sizes to its longest option unless given an explicit width. Settings spacing: modest widening plus a 5%-white 1px divider under toggle rows.

## Goals / Non-Goals

**Goals:** font regression fix; skills width cap; settings spacing + toggle-row dividers; docs/tests.

**Non-Goals:** no changes to other controls or rows; no behavior changes.

## Decisions

1. Replace `font: inherit` with `font-family: inherit` placed before `font-size` in the three components.
2. .skills-pick width 88px, flex 0 0 88px; popup list unaffected.
3. .popover gap 12px; .group padding-top 10px; .toggle-row: border-bottom rgba(255,255,255,0.05), padding-bottom 8px, margin-bottom 2px — applied to the five toggle rows only.
4. focus-kit tests assert font-size appears after inheritance declarations; ui-design.md font rule + divider utility documented.

## Risks / Trade-offs

- Any other place relying on inherited font sizes from the three components changes to the token size — that is the intended regression fix; manual checklist confirms.
- Divider is very light (5% white); trivially adjustable.