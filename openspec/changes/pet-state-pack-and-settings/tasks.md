## 1. Pet drag stability gate

- [x] 1.1 Trace the browser and poller release paths and add a red-first Rust
  regression proving that only one release can claim an active drag.
- [x] 1.2 Make drag finalization single-owner and make pointer cancellation and
  lost capture share the frontend release path.
- [x] 1.3 Hide the native pet host and companion when the current Agent has no
  readable package; synchronize only after persistence locks are released.
- [x] 1.4 Add the no-package visibility predicate test and document the
  focused automated evidence in the Eval.
- [x] 1.5 Rebuild and pause for user acceptance: the user verified drag,
  release, later Focus clicks, and a second drag remain responsive. The
  no-package-host visual check remains part of the final deferred gate.

## 2. Package compatibility and proportional rendering

- [x] 2.1 Replace the retired `hatch-pet-draft-0.2` plan with explicit official
  Hatch Pet and `focus-hatch-pet` adapters.
- [x] 2.2 Validate JSON-declared image paths and package geometry without
  hard-coded filenames; add package fixtures and red-first coverage.
- [x] 2.3 Render packages proportionally within the existing safety margin;
  verify no horizontal or vertical stretching.
- [x] 2.4 Create the desktop-only `focus-hatch-pet` review Skill and document
  its contract without overwriting the global official Skill.
- [x] 2.5 Add stable-content analysis, non-blocking artifact warnings, DPI-safe
  canvas geometry, package-scoped horizontal correction, and pet-only derived
  host tint, with automated coverage and real-package read-only evidence.
- [ ] 2.6 Run the deferred package/rendering manual gate.

## 3. Focus state mapping and companion bubble

- [x] 3.1 Define only Focus-owned continuous states and Agent-local mapping to
  discovered package animations.
- [x] 3.2 Implement success-duration return to waiting and state playback
  tests.
- [x] 3.3 Implement and test exactly-once successful direct-reply bubbles,
  chat-independent visibility, six-candidate placement, pet/chat avoidance,
  drag suppression, replacement, two-line pagination, rotation, and fades.
- [ ] 3.4 Run the state/bubble manual gate.
- [x] 3.5 Add red-first reliable-delivery tests for late initialization,
  delivery-id de-duplication, expiry, and Agent switching.
- [x] 3.6 Add the 30-second current-Agent in-memory delivery handoff and
  one-time bubble-window claim; keep workflow semantics unchanged.
- [x] 3.7 Configure `pet-bubble` once while hidden using the accepted float
  host path, run automated gates and rebuild; push the Pending checkpoint.
- [x] 3.8 Add a red-first same-current-Agent initialization regression and
  preserve pending delivery unless the persisted Agent actually changes or is
  deleted; update the Eval and INC-022 evidence.
- [x] 3.9 Add the global default-off persisted "显示流式输出" preference, limit
  it to Provider-public direct-chat text deltas, and cover bootstrap,
  restart, Codex, and Claude behavior.
- [x] 3.10 Make the package-derived pet WebView tint 50% transparent while
  retaining the existing global acrylic authority; add focused coverage and
  the Windows visual gate.
- [x] 3.11 After Requirement #119, add a red-first bubble-local delivery test;
  detach the bubble host from full Agent-store initialization while retaining
  immediate/claim `deliveryId` de-duplication and accepted float-host setup.

## 4. Restricted controls and closeout

- [x] 4.1 Hide restricted controls in standard/scholar work focus without
  changing the backend desktop-lock or forced-exit recovery paths.
- [x] 4.2 Record the top status capsule as a separate future change.
- [x] 4.3 Update the relevant ADR, README, STATUS, handoff, incident ledger,
  and Eval for each phase actually completed.
- [x] 4.4 Run the full automated gates and rebuild. Commit and push the Pending
  checkpoint to `main` as explicitly authorized; do not tag or archive before
  all affected manual gates pass.


## 5. Verified production fixes for bubble delivery and resident streaming

- [x] 5.1 CDP-verified root cause: `pet-bubble` was missing from the Tauri
  capability window list; add it so event listeners register and the host
  reports ready generations.
- [x] 5.2 Red-first coverage for a real captured resident-mode Claude stream:
  multi-line `assistant` JSON with `thinking` raw newlines must parse and
  emit `message.thinking`/`message.delta` cumulative increments plus
  `tool.started` from `tool_use` blocks.
- [x] 5.3 Add the additive `message.thinking` AgentEvent schema kind,
  fixture, and TypeScript union; keep schemaVersion 1.
- [x] 5.4 Frontend thinking accumulation, muted rendering above the answer,
  `publicTextDeltaSeen` on thinking, finalize retention, and history
  round-trip.
- [x] 5.5 Keep the streaming switch as the single gate for thinking and text
  increments; Codex public-text path unchanged.
- [x] 5.6 Refresh diagnostics expiry so stale pending envelopes expire on
  read.
- [ ] 5.7 User mouse-driven Windows acceptance: bubble beside the pet on every
  successful reply, visible streaming with thinking, and no pet/chat overlap
  regressions.
