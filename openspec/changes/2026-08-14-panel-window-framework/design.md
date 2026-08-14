## Context

See proposal - Why. Everything the panel needs already exists: window registry (ADR-0037), view registry, get_today_focus_summary + workflow_runs_recent queries, focus:tick + workflow:changed events, and (after C1) kit components.

## Goals / Non-Goals

**Goals:** one example overview panel proving the declaration+assembly path; panel recipe in ui-maintenance.md; zero change to existing behavior.

**Non-Goals:** no app-usage data collection (that is a future data-domain change), no panel configuration UI, no dynamic panel loading.

## Decisions

1. **Overview panel = grid float**: declared as WindowKind::Float (grid lifecycle, tray entry), so it inherits collapse/restore/placement/no-overlap with zero new lifecycle code. It does NOT appear in the tray (inTray: false keeps the tray at four entries); it opens via the existing window label (e.g. shortcut/panel usage later). Actually the tray is the only UI entry today — see 4.
2. **Data**: two existing read-only invokes (get_today_focus_summary, workflow_runs_recent) on mount + focus:tick/workflow:changed listeners for live updates; no new Rust commands.
3. **UI**: assembled from kit components (FocusCard rows) once C1 lands; falls back to plain classes if C1 migration of the needed pieces lags.
4. **Tray entry**: add it to the tray (inTray: true) so the user can actually open it — the tray is the established entry point; this changes the tray from 4 to 5 entries, which is additive and covered by the manual checklist.
5. **Consistency test**: the existing capabilities-vs-registry Rust test automatically covers the new label once declared.

## Risks / Trade-offs

- New window adds a WebView (bundle/CPU) → one small panel is acceptable; the manual checklist covers startup stacking regression.
- Tray grows to 5 entries → intentional, documented in the checklist.