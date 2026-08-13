# ADR-0032: Pet Package, Focus State, and Companion Boundary

Status: Accepted
Date: 2026-08-13
Requirements: #106, #107, #109, #110, #111, #112, #113

## Context

Focus accepted only one fixed pet atlas, exposed technical Provider states to
playback, and placed a clipped message bubble inside the pet host. Users need
to import the official Hatch output and configure animation meanings without
being forced into filenames chosen by Focus.

## Decision

1. Focus will accept exactly two explicit package adapters: the official Hatch
   Pet package unchanged and a Focus package whose manifest declares
   `format: "focus-hatch-pet"`. Both will normalize to one validated animation
   model; unrecognized JSON will be rejected rather than guessed. The adapters
   validate JSON-declared relative assets and atlas dimensions. Existing fixed
   192x208 `pet.json` atlases remain readable only for previously imported
   users; `hatch-pet-draft-0.2` is not a supported import format.
2. Each Agent owns a persistent mapping from six Focus continuous states
   (`resting`, `focusing`, `working`, `waiting`, `happy`, `troubled`) to any
   declared animation. Provider protocol states and transient events are not
   mapping choices. Happy lasts five seconds then returns to waiting; troubled
   lasts until the next task.
3. Import derives one stable source rectangle per animation from alpha masks,
   3x3 opening, small-component filtering, and the union of all declared
   frames. Cleaned alpha must retain at least 60% of raw alpha; ambiguous
   analysis falls back to the full cell. Source packages are
   never rewritten: Focus stores derived rectangles, warnings, colors, and the
   package-scoped horizontal correction in `pet-pack/.focus-display.json`.
4. Rendering uses one CSS/DPR canvas geometry and proportional contain inside
   the existing safety inset. Horizontal correction is an explicit package
   display parameter from 0.75 to 1.33, defaults to 1.00, and resets when the
   package is replaced. Replacement is staged and validated before exchanging
   the current directory. Calibrated retained pixels produce a restrained dark
   tint for only the pet host and a lighter related companion accent.
5. Pet messages render in a hidden-at-start `pet-bubble` companion window. It
   is non-activating, mouse-through, outside the grid/tray lifecycle, and
   remains eligible while chat is open. A successful non-empty direct Codex or
   Claude reply emits exactly one targeted bubble; `bubble:requested` is the
   sole presentation authority. Failure and cancellation do
   not. Workflow results keep their sourced chat message and emit only their
   existing authoritative bubble event.
6. Bubble placement evaluates centered-above, above-left, above-right, right,
   left, and below with a 10px pet gap. It stays in the work area, never
   intersects the pet, and prefers to avoid the visible chat window. Dragging
   hides it; snap completion repositions and fades it in after persistence
   locks are released. New text replaces old pages; complete two-line pages
   rotate every three seconds.
7. Settings use concise summaries plus shared hover details. Timer presets,
   task tracking, allow-listing, supervision toggles, and pauses are retired.
   Foreground monitoring remains an internal black-list reminder mechanism.

## Consequences

Phase 1 establishes the prerequisite native boundary: browser and poller
release reports share one active-drag owner, and the pet host is not shown for
an Agent without a readable package. Package creators may name animations
freely, while users choose their meaning in the current Agent's settings. The
companion avoids the pet host's clipping boundary but requires native Windows
mouse-driven acceptance. Calibration can exclude sparse generated artifacts
without altering the imported package and reports its decisions to the user.
Retired settings data is removed on upgrade and is not preserved as an active
compatibility API.
