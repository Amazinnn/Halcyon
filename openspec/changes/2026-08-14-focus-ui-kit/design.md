## Context

See proposal - Why. styles.css already carries base tokens (colors/spacing/radius/motion); only control-level tokens are missing. The kit is consumed first by the five most repetitive files; domain-specific controls stay untouched.

## Goals / Non-Goals

**Goals:** one token source; eight kit components; five-file migration with zero visual change; two docs (ui-design.md, ui-maintenance.md).

**Non-Goals:** no new window, no Rust/schema changes, no restyle (only deduplication), music/pet/topbar-specific controls stay out.

## Decisions

1. **Tokens**: add --fs-*, --shadow-*, --z-*, --ctrl-h-* to :root; values copied from current usages so visuals are identical.
2. **Components under components/focus/**: one file per component, script-setup + scoped styles using tokens; props modeled on current call sites (no forward-looking props).
3. **FocusWindowFrame**: renamed WindowHeader with identical props (title, collapsible) and the same emit/invoke logic; four consumers updated by import/label only.
4. **Migration order**: SettingsPopover (largest) → DesktopView → WorkflowView → ChatView → headers; each file keeps its own layout styles, only control styles move into components.
5. **Docs**: ui-design.md is the contract (written first, revised after implementation); ui-maintenance.md captures workflows (new control, new window via ADR-0037, token impact check).

## Risks / Trade-offs

- Visual drift during dedup → component styles are transcriptions of the merged existing rules; per-file build + manual checklist gate; revert per file if needed.
- Scope creep (restyling while migrating) → migration only moves existing styles, no redesign; any visual change is out of scope.