# OpenSpec Adoption Eval

Date: 2026-08-13
Requirement: #105

## Scope

Adopt the official OpenSpec core workflow for future Focus maintenance without
backfilling existing behavior or modifying the unaccepted Agent-first pets
candidate.

## Evidence

| Check | Status | Evidence |
| --- | --- | --- |
| CLI availability | Pass | `openspec --version` returned `1.8.0`; local Node is `v24.13.1`, above the upstream minimum. |
| Project initialization | Pass | `openspec init --tools 'codex,claude' --profile core --no-copilot-cloud --no-animation` created the root `openspec/` structure. |
| Codex integration | Pass | Generated the six core `openspec-*` skills beneath `.agents/skills/`; use `$openspec-propose` and related skills. |
| Claude Code integration | Pass | Generated six `opsx` commands and six `openspec-*` skills beneath `.claude/`; use `/opsx:propose` and related commands. |
| Workflow configuration | Pass | `openspec/config.yaml` records Focus stack, immutable design document, Eval gates, and the separation from requirements, ADRs, and incident records. |
| Empty change state | Pass | `openspec status` reported no active changes. The current Agent-first candidate was not imported, altered, or archived. |
| Documentation consistency | Pass | `git diff --check` completed successfully after the OpenSpec artifacts and project documentation were written. |

## Result Boundary

This is a process and documentation change. It does not validate product
behavior and therefore does not rerun desktop builds or manual Windows checks.
The next behavior change must create the first OpenSpec change and follow its
existing product-specific Eval gate.
