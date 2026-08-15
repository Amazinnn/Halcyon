## Why

The crate builds with 18 warnings, nearly all dead code from retired paths (the v1.7 multi-window pet API was replaced by Agent-workspace pets in v1.11.2; the topbar composition flag and agent-store bubble state were superseded by later reworks). Requirement #133 asks for the cleanup; ponytail: delete, do not rewrite.

## What Changes

- Delete Rust functions/fields with zero product references (verified per function via grep) together with the dead tests that referenced them.
- Delete the frontend agent-store bubble state (written but never read; pet-bubble and the Rust BubbleController own bubble delivery now).
- Fix the ineffective std::mem::forget call and the unused Mutex import.
- Extend scripts/rust-gate.ps1 with cargo clippy --lib --tests -D warnings and cargo fmt --check so the warnings stay gone.
- No behavior change; active tests stay green.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- None (pure cleanup; no spec-level behavior change).

## Impact

pets.rs, lib.rs, agents/mod.rs, claude.rs, grid.rs (dead helpers), stores/agent.ts, scripts/rust-gate.ps1, tests that referenced deleted code.