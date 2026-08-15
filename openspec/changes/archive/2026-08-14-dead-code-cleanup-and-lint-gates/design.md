## Context

See proposal - Why. Each warning candidate was verified: grep shows zero non-test references, or references only from tests that exercise the retired API. Tests that asserted deleted code are deleted or rewritten onto active APIs; pure test-tool methods (ScreenRect::contains, BubblePosition::rect, PetPackage::animation) are restored behind #[cfg(test)].

## Goals / Non-Goals

**Goals:** zero dead-code warnings; gate that keeps them gone (warning check + clippy correctness); no behavior change.

**Non-Goals:** no rustfmt enforcement (304 pre-existing diffs would be a massive unrelated diff); no clippy style-lint cleanup (30+ pre-existing categories); no restructuring; no new dependencies.

## Decisions

1. Per-function grep verification before deletion; delete function + its tests together; rewrite live tests onto active APIs.
2. Keep functions with product references (info_for_agent, prepare_import_for_agent, list).
3. agent.ts bubble state removed (state/_bubbleSequence/_seenBubbleDeliveryIds/showBubble/clearBubble/bubble:requested listener); pet-bubble independent endpoint and Rust BubbleController own delivery.
4. rust-gate.ps1: cargo test → cargo check warning scan (any warning fails) → cargo clippy --lib --tests -- -A clippy::all -W clippy::correctness -W clippy::suspicious -D warnings.
5. drag.rs Mutex import is cfg(test) (used only in test diagnostics); restored as such.

## Risks / Trade-offs

- Blind deletion risk mitigated by grep + the test suite going green after deletion.
- A test that looks dead may indirectly guard a live path: checked asserted objects still have product paths before deleting; rewritten where value remained.