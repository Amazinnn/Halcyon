## Context

See proposal - Why. The overview window was declared in the three standard places (WindowSpec, ViewRegistry with inTray, capabilities) plus its view; all are removed together, and the consistency test (WINDOW_SPECS vs capabilities) guards the removal.

## Goals / Non-Goals

**Goals:** remove the example panel cleanly; keep the recipe documentation; zero impact on other windows.

**Non-Goals:** no changes to the panel recipe mechanism, window registry, or UI kit.

## Decisions

1. Remove in dependency-safe order: window_spec entry → view-registry entry + view file → capability entry; the capabilities consistency test fails red if any is missed.
2. Float-set and tray tests revert to the pre-C4 expectations.
3. ui-maintenance.md §3 keeps the recipe steps and drops the OverviewPanelView reference.

## Risks / Trade-offs

- Any leftover declaration is caught by the consistency test or the frontend tests; trivial to revert.