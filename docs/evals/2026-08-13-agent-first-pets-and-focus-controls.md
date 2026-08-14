# Agent-First Pets, Calibration, And Bubble Eval

Date: 2026-08-13
Requirements: #103, #104, #106, #107, #109, #110, #111, #112, #113
ADR: ADR-0031, ADR-0032
OpenSpec change: `pet-state-pack-and-settings`
Status: Automated checkpoint implemented; Windows visual acceptance Pending

## Scope

This immutable checkpoint covers the accumulated Agent-first pet work and the
current calibration/tint/bubble implementation:

- one optional, non-transferable pet package in each Agent workspace;
- no native pet or companion host when the current Agent has no readable pack;
- official Hatch Pet and explicit `focus-hatch-pet` package adapters;
- stable visible-subject analysis without rewriting source assets;
- proportional DPI-safe rendering plus package-scoped horizontal correction;
- Focus-owned continuous state mapping;
- pet-only derived host tint;
- exactly-once successful direct-reply bubbles with pet/chat avoidance; and
- hidden restricted controls during standard/scholar work focus.

This snapshot does not claim that the new Windows rendering or bubble behavior
has passed the deferred human gate. No release tag is created from it.

## Confirmed Drag Repair

The diagnostic change proved that post-release freezing was not an uncleared
pointer, stale `active_drag`, or visible overlay. The trace stopped at
`finalize:snap:start`: placement held `state.settings` and called bubble
positioning, which tried to acquire the same non-reentrant mutex. Placement now
finishes calculation and persistence inside the lock, then performs native
movement, bubble placement, and topbar work after releasing it.

The user subsequently reported that drag, release, later Focus clicks, and the
next drag were normal. That closes INC-020 and allows the independent diagnostic
change to be archived.

## Package And Rendering Evidence

Focus reads JSON-declared relative image paths and validates atlas geometry for
both accepted formats. Per-animation analysis applies an alpha mask, 3x3
morphological opening, small connected-component filtering, a union across all
declared frames, and a safety inset. Low-confidence analysis falls back to the
full cell and emits a non-blocking warning. Focus stores rectangles, warnings,
colors, and correction in Agent-local `.focus-display.json`; it does not modify
the imported manifest, atlas, or external package.

The current package
`C:\Users\yanwei\Desktop\blue-whale-maid-focus-hatch-pet-final` was inspected
read-only through a temporary environment-driven test that was removed after
use. Its manifest declares a 12x8 atlas with 256x256 cells and a 3072x2048 WebP.
Derived evidence included:

- accent `#819dd0` and host tint `#23324d`;
- `idle` source rectangle `(34, 10, 187, 234)`;
- `focused` source rectangle `(49, 13, 158, 232)`;
- `sleeping` source rectangle `(10, 29, 236, 198)`; and
- non-full-cell rectangles for all eight animations, with sparse-artifact
  warnings for affected frames.

Canvas CSS size, DPR backing size, and draw geometry share one calculation.
The calibrated source is contained proportionally in the existing safe inset.
The 0.75-1.33 correction is stored per Agent and package, defaults to 1.00, and
resets on replacement. The pet HWND alone receives the derived dark tint.

The final review added a 60% retained-alpha confidence threshold, sampled the
palette only from cleaned pixels inside calibrated subject rectangles, and
reconciled stale state mappings against replacement animation IDs. Agent imports
now validate a sibling staging directory before exchanging the active package;
reimporting the current package is a regression case and no longer deletes its
own source before copying.

A pre-push review found that the first implementation returned and decoded the
complete base64 atlas on every Provider state transition. It also allowed an
older async response to replace a newer animation and did not close replaced
`ImageBitmap` objects. The repaired path loads the atlas only when the package
or current Agent refreshes, requests only lightweight animation metadata on
state changes, discards stale generations, and closes every replaced or stale
bitmap. Red-first frontend tests cover latest-only sequencing and bitmap
lifetime.

The follow-up concurrency review found that package refresh and animation
refresh still shared one generation counter. A valid state update could cancel
an in-flight package refresh, while a stale package failure could clear the new
Agent's already loaded pet. The final request coordinator keeps package and
animation generations independent, invalidates animation work when a package
refresh starts, and binds every commit to the captured Agent and package. A
state change during atlas decoding triggers one fresh lightweight animation
request after the package commits. Three red-first tests cover independence,
package invalidation, and stale Agent refusal.

Before delivery, independent review also identified a companion visibility
race and several persistence-boundary gaps. The companion now cancels an
in-flight placement/show sequence whenever drag, expiry, Agent selection, or a
replacement message changes visibility; old fade callbacks are cleared before
they can advance a replacement message. New Agent imports accept only the two
explicit package formats, while existing fixed-atlas packages remain readable
and have a dedicated one-time migration path. Package replacement now retains
the previous package until one SQLite transaction updates both the Agent pack
ID and state mapping, then rolls the directory back if persistence fails.
Agent deletion stages its package removal before the now-transactional workflow,
session, and character cascade. Supervision reminders no longer synthesize a
frontend bubble and instead emit the same targeted `bubble:requested` event
used by other producers.

## State And Bubble Evidence

Settings maps arbitrary discovered animations to six Focus-owned continuous
states: `resting`, `focusing`, `working`, `waiting`, `happy`, and `troubled`.
Provider-native and transient names are not exposed. Happy lasts five seconds
before returning to waiting; cancellation returns to the timer base state.

Visible non-empty successful Codex and Claude direct turns each emit exactly one
targeted `bubble:requested`. Errors, cancellation, and hidden workflow turns do
not emit a Provider bubble. Workflow sourced history remains separate and does
not synthesize a second frontend bubble.

The companion remains eligible with chat open. Native placement tries above,
above-left, above-right, right, left, and below, stays inside the work area,
never intersects the pet, and minimizes overlap with visible chat. Drag start
hides the companion; drag completion restores it after persistence locks are
released. New replies replace old pages, which use measured two-line layout,
three-second rotation, and fade transitions.

The same review found two state-isolation gaps. Selecting another Agent now
clears the prior bubble, reaction, Provider state, pet state, and happy timer.
The normal Provider `success` then `idle` tail no longer cancels the five-second
happy duration; another substantive work/wait/error state may still take over.
The frontend also no longer synthesizes a workflow bubble from
`workflow:agent_result`; only the authoritative backend `bubble:requested`
event controls that presentation. Each repair was captured by a failing test
before the minimal implementation.

The final review also removed direct-chat frontend bubble synthesis, made
identical replies carry distinct playback IDs, based expiry on measured pages,
and suppressed every show attempt between `pet:drag-started` and
`pet:drag-ended`. Native placement now returns no result and hides the host when
no clamped candidate is pet-safe. Scholar pause/skip are rejected at the store
action boundary, while Rust exit rejection uses the mode frozen at round start.

## Automated Evidence

Final command counts are filled only from the fresh delivery run below.

| Check | Status | Evidence |
| --- | --- | --- |
| Frontend tests | Pass | `npm test -- --run`: 17 files, 85 tests passed. Coverage includes rendering geometry, independent package/animation request generations, state lifecycle, settings correction, authoritative bubble playback, drag suppression, focus restrictions, and drag release. |
| Frontend production build | Pass | `npm run build` completed; only the existing Vite chunk-size advisory remains. |
| Rust library tests | Pass | `cargo test --lib`: 210 passed, 1 ignored, 0 failed. Coverage includes retained-alpha calibration, calibrated palette derivation, staged replacement rollback, explicit import-format boundaries, state-map reconciliation, Provider bubble count, placement, frozen exit mode, and drag lock boundaries. |
| Event schema | Pass | `npm test`: 11 valid and 4 invalid fixtures checked; TypeScript passed. |
| OpenSpec strict validation | Pass | `openspec validate pet-state-pack-and-settings --strict` and `openspec validate --specs --strict` passed after syncing and archiving the diagnostic change. |
| Diff hygiene | Pass | `git diff --check` exited 0; only line-ending normalization notices were printed. |
| Release rebuild | Pass | First attempt reached the Rust linker but failed with Windows `os error 5` because this repository's release `desktop.exe` and watchdog were still running. After stopping only those two exact-path processes, `launch-focus.cmd rebuild` exited 0. |

## Deferred Windows Acceptance

## 2026-08-14 Pet Resize Regression

User report #114 found that a loaded spritesheet did not resize with the pet
host. The cause was lifecycle ordering: the observer was attached only after
the asynchronous package load, and it looked for a conditionally rendered
canvas parent that could be absent at that instant. The corrective path binds
the observer to the stable pet stage, then awaits Vue's DOM commit and refits
the CSS canvas and DPR backing dimensions after package loading and mounting.

| Check | Status | Evidence |
| --- | --- | --- |
| Targeted regression test (red) | Pass | Before implementation, `npx vitest run src/views/pet/PetView.test.ts` failed because `nextTick`, the stable stage ref, and post-commit refit were absent. |
| Targeted regression test (green) | Pass | `npx vitest run src/views/pet/PetView.test.ts`: 1 file, 1 test passed after binding the stable stage and post-commit refit. |
| Windows visual acceptance | Pending | Resize the loaded pet through all four supported host sizes after the rebuilt application is available. |

1. Import the current blue-whale package and inspect all four pet sizes.
2. Confirm automatic proportions; move the correction slider and reset to 1.00.
3. Confirm the pet host uses a restrained blue-derived tint.
4. Send real Codex or Claude replies with chat closed and open; each successful
   reply produces one bubble in both states.
5. Confirm a long reply rotates through complete two-line pages.
6. Drag the pet while a bubble is visible; it hides, then reappears without
   touching the pet or visible chat.
7. Confirm a no-package current Agent shows no pet host or companion.
8. Confirm drag, release, later Focus clicks, and another drag remain responsive.
9. Confirm no blue title strip, overlap, brightness-center, or tray regression.

Until these checks are reported, the new rendering, tint, and companion visuals
remain Pending even after this checkpoint is pushed to `main`.
## 2026-08-14 Requirement #115: pet bubble reliable delivery

Status: **automated verification passed; Windows manual acceptance Pending**.

Observed production symptom: a successful direct Agent reply can emit the
targeted `bubble:requested` event before the independent `pet-bubble` WebView
has restored the current Agent identity, causing that window to discard it.
The repair keeps one latest direct-reply envelope for the current Agent only in
memory for 30 seconds. A bubble window claims it once after Agent initialization;
the immediate and claimed paths share a delivery id and the frontend de-duplicates
it. The record clears on Agent switch/deletion and is not persisted across
restart. Workflow and supervision bubbles are explicitly excluded from this
handoff.

Red-first evidence:

- `agent.test.ts`: duplicate immediate/claimed `deliveryId` initially produced
  two playback identities, then passed after de-duplication; concurrent `init()`
  shares its listener registration; a post-init current-Agent claim produces
  one bubble.
- `pending_bubble_is_claimed_once_for_matching_agent_and_expires` covers target
  matching, one-time consumption, and TTL expiry.

Completed gates on 2026-08-14: `npm test -- --run` (90 tests), `npm run build`,
`cargo test --lib` (211 passed, 1 ignored), `packages/event-schema` `npm test`,
`openspec validate pet-state-pack-and-settings --strict`, `git diff --check`, and
`launch-focus.cmd rebuild`. The rebuild result is ready for the manual list below;
no release tag or OpenSpec archive has been created.

Native scope: `pet-bubble` now receives the same once-only hidden creation float
host setup as accepted internal float hosts. Its later show/hide/move path stays
no-activate and mouse-through; it remains outside grid and tray lifecycle.

Manual acceptance required after rebuild:

1. Start Focus and send the first real direct Agent message; verify one bubble.
2. Repeat with chat closed and open; the bubble must remain independent.
3. Confirm a long reply paginates, drag suppression/return still works, and the
   bubble avoids the pet and visible chat window.
4. Switch to another Agent and test again; no prior Agent message may appear.
5. Check an Agent without a pet package; no visible pet/bubble host, no float
   caption, blue strip, overlap, freeze, or brightness-center regression.
