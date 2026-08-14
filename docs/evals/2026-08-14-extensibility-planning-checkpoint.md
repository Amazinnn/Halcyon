# Extensibility Planning Checkpoint

Date: 2026-08-14
Requirements: #126, #127
Documents: docs/architecture/extensibility-plan-v1.md, docs/next-phase.md, docs/STATUS.md
Status: Planning docs delivered; C1–C4 implementation pending approval

## Scope

Documentation round (evals/README docs-only rule): a brainstorming session
(user-directed, per requirement #127) produced the extensibility master plan.

- User-selected future scenarios: custom panel windows, Agent-driven focus-cli
  expansion, multi-window event streams; pet extensibility reserved as P2.
- The plan maps each scenario to extension points (UI kit, CLI command
  registry, event namespaces + thin-window subscriptions, pet boundaries) and
  defines the implementation roadmap C1 (Focus UI Kit + design/maintenance
  docs) → C2 (CLI CommandSpec registry) → C3 (event grouping + thin
  subscriptions) → C4 (panel window framework), each an independent OpenSpec
  change.
- requirements-verbatim.md appended #126 (UI kit + design docs) and #127
  (CLI/pet extensibility + brainstorming); STATUS.md checkpoint and
  next-phase.md 扩展方向 section now link the master plan.

## Gates

| Check | Result |
| --- | --- |
| git diff --check | Clean |
| Requirement numbering/status consistency | #126/#127 recorded, docs link them |
| Release rebuild | N/A (docs-only round per evals/README) |

## Pending

User approval to start C1 (Focus UI Kit OpenSpec change). No code changes in
this round.
