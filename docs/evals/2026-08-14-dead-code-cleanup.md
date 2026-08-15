# Dead-Code Cleanup + Lint Gates Checkpoint

Date: 2026-08-14
Requirement: #133
OpenSpec change: `2026-08-14-dead-code-cleanup-and-lint-gates` (skip_specs)
Status: Automated gates green; lightweight manual regression Pending

## Scope

- Removed all 18 build warnings: retired v1.7 multi-window pet API
  (load_draft_package, import, import_for_agent, sheet_base64, info_for,
  remove, animation, contains, resolve, initial_provider_for_pet, row_order),
  topbar_uses_native_composition, AGENT_ID, claude.rs first_delta_sent,
  mem::forget (let _ =), unused imports (lib.rs untouched, drag.rs Mutex is
  cfg(test), pets.rs base64).
- Test-only helpers restored with #[cfg(test)] (ScreenRect::contains,
  BubblePosition::rect, PetPackage::animation); a tests-module helper
  re-implements import_for_agent via prepare + commit (10 call sites unchanged).
- Deleted dead tests with the dead APIs; rewrote live tests onto active APIs
  (transparent_background_validation now calls check_transparent_background;
  list test writes packs directly).
- Frontend: agent store bubble state removed (state, _bubbleSequence,
  _seenBubbleDeliveryIds, showBubble, clearBubble, bubble:requested listener);
  tests updated (dedup tests deleted, counts adjusted).
- Gate script: rust-gate.ps1 now fails on any rustc warning (dead-code
  regression guard) and runs clippy correctness/suspicious -D warnings.
  rustfmt --check was measured (304 diffs across the pre-existing code base)
  and deliberately NOT enforced this round to avoid a massive unrelated diff.

## Automated Gates

| Gate | Result |
| --- | --- |
| scripts/rust-gate.ps1 (test + warnings + clippy correctness) | Pass, 215 passed / 0 failed / 1 ignored, 0 warnings |
| npm test -- --run | Pass, 23 files / 128 tests |
| npm run build | Pass |
| openspec validate | Pass |
| git diff --check | Clean |
| Release rebuild | Pending |

## Manual Acceptance (lightweight; no visual change)

1. App starts; settings/pet/workflow paths work (pet import + agent rows).
2. Chat direct reply still works; pet bubble still appears via the independent
   endpoint (bubble state removal regression).
3. Startup/focus flow/glass unchanged.

## Notes

- clippy style lints and rustfmt remain out of scope (pre-existing 30+
  categories / 304 diffs); correctness-level lints are now gate-enforced.
