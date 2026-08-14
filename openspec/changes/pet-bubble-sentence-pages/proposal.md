## Why

The bubble hard-wraps two lines per page by character width, so sentences can
be cut mid-word. Requirement #124 asks for sentence-complete pages and a
bubble that resizes to its content (up to 340px wide, height by lines).

## What Changes

- Paragraphs that fit on one measured line are shown whole (any sentence
  count). Longer paragraphs are split at sentence boundaries (。！？… and
  newlines) and each page carries complete sentence blocks only.
- Pages hold at most 4 lines; multiple pages keep the 3-second rotation, a
  single page never rotates.
- The bubble resizes to its content: width clamps between 180 and 340px, height
  follows the active page's line count. A new `pet_bubble_resize` command
  applies the size through the native no-activate path and re-runs placement so
  the bubble still avoids the pet and visible chat.

## Capabilities

### New Capabilities

- `pet-bubble-sentence-pages`: sentence-complete pagination and content-sized
  bubble geometry.

### Modified Capabilities

- `pet-companion-bubble` (in `pet-state-pack-and-settings`): pagination and
  geometry behavior is superseded by this change's algorithm.

## Impact

Changes the pagination library, PetBubbleView sizing, and one native resize
command. Delivery, drag suppression, avoidance candidates, chat independence,
and the Bubble Controller are unchanged. Windows visual acceptance is required
before archiving.
