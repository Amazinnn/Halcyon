## Context

The pet drag has two legitimate release reporters: the browser receives
`pointerup`, while the Rust cursor poller independently observes that the left
button is no longer down. Previously either path could finalize while the
other left `AppState.active_drag` populated. A later drag could then terminate
or finalize stale state, re-entering main-thread window operations and making
Focus unresponsive.

An Agent with no package also created a transparent native pet host. Hiding
only the Vue content is insufficient because the host remains a drag target.

## Phase 1 Decision

`active_drag` is the single ownership token. Both release paths must first
atomically take it. The winner stops the poller and finalizes once; the loser
does nothing. The poller marks itself finished before emitting natural release
so the winning main-thread path never waits on an event that it caused itself.

The pet host is visible only when all three conditions hold:

- a current Agent exists;
- its database row declares a pet package; and
- that Agent workspace contains a readable package.

Visibility synchronization runs only after database and settings locks are
released. This avoids recursively reacquiring the store while reconciling the
native host.

## Later Phases

### Phase 2: Package and proportional rendering

Support two explicit, validated forms:

- the official Hatch Pet package unchanged; and
- a Focus package identified by `format: "focus-hatch-pet"`.

Both declare their image paths through JSON rather than hard-coded filenames.
The Focus package may customize unit size, animation count, frame count, and
FPS; defaults remain 192x208 cells, 8 columns, and 9 rows.

Import analysis derives one stable source rectangle per animation. For every
declared frame it builds an alpha mask, applies a 3x3 morphological opening,
removes components too small to represent the character, then unions the
surviving bounds across that animation and adds a safety inset. If the retained
alpha mass is too low or the result is otherwise ambiguous, the animation falls
back to its full cell. Excluded edge streaks are reported as non-blocking
quality warnings. The source package is never rewritten; derived display data
lives in the Agent workspace.

Cleaned subject alpha must retain at least 60% of the declared frames' raw
alpha mass; otherwise calibration falls back to complete cells. Replacement is
copied, parsed, analyzed, and given display metadata in a sibling staging
directory before the current package is exchanged. Existing state mappings
retain only animation IDs that the replacement package still declares.

Canvas layout has one geometry source: CSS display dimensions, device-pixel
backing dimensions, and draw coordinates are calculated together. The derived
source rectangle is contained proportionally inside the existing safe inset.
A package-scoped horizontal correction from 0.75 through 1.33 defaults to 1.00;
non-default values are an explicit user correction, not a claim that the source
is naturally proportionate. Replacing the package resets this value.

Cleaned pixels inside the calibrated subject rectangles also produce a
quantized representative color; excluded atlas artifacts do not participate.
Focus
derives a low-saturation dark acrylic tint for the pet HWND and a lighter accent
for its Web fallback and bubble. Other float hosts keep their existing tint.

### Phase 3: State and companion bubble

Expose only Focus-wrapped continuous states. Each Agent maps those states to
any discovered animation. A successful state persists for its configured
duration before returning to waiting.

The bubble remains an independent, non-activating and mouse-through host. A
successful final Codex or Claude reply emits exactly one targeted bubble event;
errors and cancellation remain chat-only. Workflow final results use that same
event once while their sourced chat message remains separate. Opening chat does
not hide the companion.

`bubble:requested` is the only presentation authority. Frontend chat-completion
and workflow-history events never synthesize a second bubble.

### Reliable delivery revision (2026-08-14)

The event relay assigns a stable delivery id to each targeted bubble event. It
retains only the latest current-Agent delivery in AppState memory for 30 seconds.
The pet-bubble window claims a matching delivery after its Agent store finishes
initialization; claiming consumes it. Agent switching and deletion clear the
record. Immediate delivery and a later claim use the same id, so the frontend
can deduplicate without extending playback. The record is not stored on disk.

The Agent store uses one shared initialization Promise for concurrent WebViews.
Current-Agent persistence is awaited before dependent pet refresh events are
emitted. The pet-bubble host receives the accepted float-host configuration only
once while hidden; later movement and visibility remain no-activate operations.

Placement evaluates, in order, centered-above, above-left, above-right, right,
left, and below candidates with a 10px gap. Candidates remain in the work area
and never intersect the pet; if clamping leaves none pet-safe, Focus hides the
bubble. Focus chooses the first candidate that also avoids
the visible chat window, or otherwise the pet-safe candidate with least chat
overlap. The bubble is hidden during pet dragging and is repositioned and faded
in only after snap finalization has released persistence locks. A new final
message replaces the current pages immediately. Text uses measured two-line
pages, three-second turns, and fade transitions.

### Phase 4: Remaining controls

Hide application exit during standard and scholar work focus. Scholar focus
also hides pause and skip, and the timer action boundary rejects equivalent CLI
requests. Exit rejection uses the mode frozen when the current round began,
not mutable settings for the next round. The top status capsule is explicitly out of scope
and becomes a separate proposal.

## Non-Goals

- No arbitrary JSON compatibility or guessed package format.
- No changes to the accepted float-host chrome, grid geometry, tray operation,
  or desktop-lock architecture in Phase 1.
- No new background recovery process.

## Risks and Gates

Automated tests can prove ownership, geometry, color derivation, event counts,
and visibility predicates, not real Windows mouse behavior. Each phase requires
the standard build/test/rebuild gates and an Eval update. The user explicitly
authorized this automated checkpoint to be pushed to `main` before the deferred
manual visual gate; the change remains unarchived and untagged until that gate.
