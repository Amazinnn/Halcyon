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

## 2026-08-14 Requirements #116/#117 follow-up planning

Status: **implementation pending; no Windows acceptance claim**.

The user reported that the first direct reply still produced no bubble. Source
tracing reproduced the missing handoff: the independent bubble WebView restores
the same Agent through `agent_set_current`, and that command currently clears
the pending delivery even when the persisted Agent ID did not change. The
active pet OpenSpec now requires a red-first same-current-Agent regression,
preserving the 30-second, single-entry, one-claim delivery only until a real
Agent switch or deletion.

Requirement #116 also records the approved default-off global
"显示流式输出" preference. Its contract is intentionally limited to
Provider-public direct-chat text deltas for the next Codex or Claude turn; it
does not expose hidden reasoning, workflow progress, or tool detail. The pet
surface will retain its package-derived tint at 50% opacity so existing native
acrylic can show through.

Requirement #117 has a separate active OpenSpec
`topbar-capsule-acrylic-clip`: native acrylic must be clipped once while the
topbar is hidden to the actual client-pixel pill, and the global acrylic toggle
must update topbar. Automated tests and rebuild have not been run for either
follow-up; the mouse-driven Windows checks remain pending.

## 2026-08-14 Automated delivery checkpoint

Status: **automated verification passed; Windows visual acceptance Pending**.

The same-current-Agent initialization path now preserves a pending bubble; a
real Agent change still clears it. `chatStreamingEnabled` is globally persisted,
defaults to false, is supplied by bootstrap, and snapshots direct Codex/Claude
display behavior for the next turn. The pet WebView blends its derived tint at
50%. Topbar receives a creation-only native client-pixel round region (radius
is half height); its runtime acrylic toggle is synchronized without joining the
float-host lifecycle. ADR-0033 records that boundary.

Fresh gates passed: frontend tests (90), `npm run build`, event-schema tests,
`cargo test --lib` (212 passed, 1 ignored), strict validation for both active
changes and global specs, `git diff --check`, and `launch-focus.cmd rebuild`.
The first rebuild retry found two exact-path old release processes holding
`desktop.exe`; after stopping only those processes, rebuild exited 0.

The remaining work is exclusively the required Windows mouse-driven acceptance:
bubble delivery with chat closed/open, public-text streaming switch behavior,
pet translucency, and topbar pill glass during show/hide/focus without input
interception or float/grid/tray regressions.

## 2026-08-14 Follow-up after failed visual report

The user reported that all four requested visual behaviors were still absent.
The previous automated checkpoint is therefore not visual evidence. The retry
now falls back to the pet HWND outer rect when client geometry is zero, moves
topbar native-region creation until after HWND creation but before first show,
and exposes the persisted streaming preference directly in the chat header as
well as Settings. Fresh frontend, Rust, schema, strict-validation, diff, and
release-rebuild gates passed. Windows acceptance remains required.

## 2026-08-14 Rework after the user-reported visual failure

Status: **implementation changed; Windows visual acceptance remains Pending**.

The user reported that the prior rebuilt candidate still showed none of the four
requested behaviors. That report supersedes any implication that prior automated
gates proved visible behavior.

- Bubble delivery no longer boots the independent window's full Pinia Agent
  store. It registers its dedicated event listener first, gets current identity
  from bootstrap, and claims the one pending direct-reply delivery. The local
  endpoint de-duplicates immediate and claimed delivery ids.
- The pet surface now has a visible package-derived translucent gradient,
  border/highlight, and blur rather than a single subtle color-mix layer.
- Topbar now reuses the accepted hidden float-host creation setup, then creates
  its exact half-height pill region from physical `inner_size`; ADR-0034
  supersedes the earlier region-only mechanism.

Fresh red-first evidence in this rework:

- `pet-bubble.test.ts` failed because a bubble-local endpoint did not exist,
  then passed for immediate delivery, duplicate suppression, and wrong-Agent
  rejection.
- `PetView.test.ts` failed because there was no explicit translucent glass
  surface, then passed with the gradient/blur contract.
- Rust topbar tests failed until shared host setup and exact-pill configuration
  were both declared, then passed.

Fresh completion gates on 2026-08-14:

| Check | Status | Evidence |
| --- | --- | --- |
| Frontend tests | Pass | `npm test -- --run`: 18 files, 93 tests passed. |
| Frontend build | Pass | `npm run build` exited 0; the existing Vite chunk-size advisory remains. |
| Rust library tests | Pass | `cargo test --lib`: 215 passed, 1 ignored, 0 failed. |
| Event schema | Pass | `packages/event-schema` `npm test`: 11 valid and 4 invalid fixtures checked. |
| OpenSpec | Pass | Both active changes and global strict validation exited 0. |
| Diff hygiene | Pass | `git diff --check` exited 0. |
| Release rebuild | Pass | `launch-focus.cmd rebuild` exited 0. |

None of these gates proves native Windows composition or the live direct-reply
flow. Visual behavior still requires the user's mouse.

## 2026-08-14 Requirement #120 rework (in progress)

The prior checkpoint is invalidated by the user's Windows report. The topbar no
longer requests native acrylic, native region, or system shadow; its WebView
owns one pill background/border/shadow surface. Direct-reply bubbles now use a
native Controller: queue, ready generation, render acknowledgement, placement,
and no-activate show are separately recorded, and a delivery is retained until
the acknowledged native show succeeds. Codex and Claude now use one public-text
delta gate instead of first-versus-later output heuristics; a Claude turn with
no public delta shows an activity state. Focused Rust red tests passed; full
gates and Windows mouse acceptance remain pending.

## 2026-08-14 Production-root-cause rework (Requirements #121/#122)

Status: **automated verification passed; Windows visual acceptance Pending**.

Live CDP instrumentation on the rebuilt release found the true production
failures behind the recurring "no bubble" and "no streaming" reports:

- The pet-bubble WebView was missing from the Tauri capability window list, so
  every `plugin:event|listen` was ACL-rejected and the host never reported a
  ready generation. The native Bubble Controller therefore never dispatched
  (diagnostics showed `readyAgentId: null`; console showed
  `Command plugin:event|listen not allowed by ACL`). The capability list now
  includes `pet-bubble`.
- Resident Claude turns (stdin held open) never emit `content_block_delta`;
  their only increments are partial `assistant` messages carrying
  `thinking`/`text`/`tool_use` content blocks (verified with the real CLI
  in five configurations, including open stdin and `--resume`). The adapter
  parsed only `content_block_delta`, so resident-mode deltas were always
  zero. The adapter now consumes `assistant` messages: `thinking` streams
  as the new additive `message.thinking` event, `text` as `message.delta`
  (cumulative-length diffed), and `tool_use` restores `tool.started`. The
  user explicitly approved showing the Provider-visible thinking process while
  the streaming switch is on (requirement #122); the switch remains the single
  gate and Codex keeps its public-text path.
- Topbar pixel sampling showed the WebView pill shadow (`0 6px 18px`) clipped
  by the transparent host's rectangular bounds (dark pixels up to 28 device px
  outside the pill curve at the corners). The host now reserves shadow margins
  (L/R 20, top 14, bottom 26) around the 500x44 pill; the shadow renders fully
  and follows the pill curve exactly.

Fresh gates on 2026-08-14: frontend tests (95), `npm run build`, event-schema
tests (12 valid), `cargo test --lib` (217 passed, 1 ignored), strict
validation for both active changes and global specs, `git diff --check`, and
`launch-focus.cmd rebuild`.

Manual Windows acceptance required after rebuild:

1. With the streaming switch on, send a real direct message; verify the chat
   shows the thinking block (muted) and the answer appearing incrementally, and
   the final reply is complete.
2. With the switch off, verify no increments appear and the final reply still
   arrives.
3. Send successful replies with chat open and closed; each must produce one
   pet bubble beside the pet that avoids the pet and the visible chat window.
4. Verify the topbar capsule shadow fully follows the pill curve with no
   clipped corners; position and click-through are unchanged.
5. Drag the pet with a bubble visible; the bubble hides and returns after
   release without overlapping.
6. Regressions: no caption/blue strip, no overlap/freeze, pet resize through
   all four host sizes, and no prior-Agent bubble residue after switching.
