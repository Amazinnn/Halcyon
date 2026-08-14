## Context

See proposal - Why. The overflow is container-level (no wrap) plus item-level (min-width: 0). Fix both: tokens give items a floor, wrap gives rows an escape.

## Goals / Non-Goals

**Goals:** minimum-width tokens; FocusInput/FocusSelect floors; Agent create row two-line layout; pack-row wrap + name floor; ui-design layout rules; zero change elsewhere.

**Non-Goals:** no redesign of other windows; no changes to controls already fitting; no behavior changes.

## Decisions

1. Tokens live in styles.css :root; components consume them (kit contract).
2. Agent create row: name input on its own line (full popover width), provider select + add button on the second line — the name input can no longer be crushed.
3. pack-row: flex-wrap: wrap + row-gap 4px; pack-name flex: 1 1 var(--text-min-row) with min-width var(--text-min-row).
4. Audit pass: check .row/app-row/run-row in SettingsPopover and similar rows elsewhere; wrap only rows with real overflow risk.
5. ui-design.md adds the layout section; focus-kit tests assert the tokens; SettingsPopover source assertions updated if they touch layout.

## Risks / Trade-offs

- Minimum widths can push rows taller in narrow windows → acceptable trade for readable text; the 300px popover fits the planned layouts (verified by hand-test item).
- wrap can change row heights → row-gap keeps rhythm; no visual change where rows already fit.