## Context

The current pagination cuts by measured width and fixes two lines per page in a
248x82 host. The user approved: whole short paragraphs, one sentence per page
for long paragraphs with rotation retained, 340px max width, height by lines.

## Decisions

### Sentence blocks, never partial sentences

Input is split into paragraphs by newlines. A paragraph whose measured width
fits the max content width becomes one block. Otherwise it is split at
sentence-final characters (。！？… and newlines); each fragment is a block.
Blocks are laid out into pages of at most 4 lines with wrapping inside a block;
a block never spans pages. Only pages > 1 rotate (3s, existing mechanism).

### Content-sized host with one-shot convergence

Width = clamp(longest line + padding, 180, 340); height = active page lines x
line height + padding + tail room. The view invokes `pet_bubble_resize`
only when text or pages change, so geometry converges in one step with no
oscillation. The Rust command uses the existing native `resize_window_raw`
(SWP_NOACTIVATE) and re-runs `position_pet_bubble` with the new outer size so
avoidance stays correct (candidates above/above-left/above-right/right/left/
below, clamped to the work area, zero pet overlap).

## Risks / Trade-offs

- [Very long sentences] -> a single sentence longer than 340px wraps across
  lines within its block; the sentence is still never truncated mid-text.
- [Tall pages] -> capped at 4 lines per page; longer content pages rotate
  instead of covering the screen.
- [Automated tests cannot prove visuals] -> require the mouse-driven
  acceptance gate; keep the change unarchived until it passes.
