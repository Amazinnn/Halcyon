## 1. Sentence pagination

- [x] 1.1 Red-first pagination tests: whole short paragraphs, sentence splits,
  blocks never split across pages, max 4 lines per page, sentence-final
  characters always at block ends, rotation only for multi-page content.
- [x] 1.2 Implement the pagination rewrite in pet-bubble.ts and wire
  PetBubbleView to it.

## 2. Dynamic geometry

- [x] 2.1 Red-first size tests: width clamp 180-340, height by lines,
  identical input yields identical size (convergence).
- [x] 2.2 Add the `pet_bubble_resize` command using the native no-activate
  resize and re-placement with the new size; initial host size 340x120.
- [x] 2.3 Keep drag suppression, expiry, delivery, and placement avoidance
  unchanged; update affected unit tests.

## 3. Evidence and acceptance

- [x] 3.1 Run all automated gates and rebuild.
- [x] 3.2 User mouse-driven Windows acceptance: short reply shows whole, long
  reply rotates sentence-complete pages, bubble sizes to content, avoids pet
  and chat, drag hides/restores.
