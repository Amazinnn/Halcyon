## Purpose

Stop cutting pet-bubble sentences mid-text: paginate on complete sentence
blocks, show whole short paragraphs, keep rotation for multi-page content, and
let the bubble host resize to its content (max width 340px, height by lines).

## ADDED Requirements

### Requirement: Sentence-complete bubble pages
The system SHALL paginate bubble text by natural paragraphs and sentence
boundaries: a paragraph whose measured width fits one line is shown whole as
one block regardless of sentence count; a longer paragraph is split at
sentence-final characters (。！？… and newlines) into blocks. Blocks SHALL
never be split across pages, pages SHALL hold at most four lines, and only
multi-page content SHALL rotate (3 seconds per page).

#### Scenario: Short paragraph with multiple sentences
- **WHEN** a reply contains a paragraph that fits on one measured line
- **THEN** the bubble shows the whole paragraph including every sentence
  without rotation

#### Scenario: Long paragraph
- **WHEN** a reply contains a paragraph wider than the content area
- **THEN** the bubble splits it at sentence boundaries and rotates one complete
  sentence per page

#### Scenario: Sentence never cut
- **WHEN** any reply is displayed
- **THEN** no page ever ends in the middle of a sentence and every sentence
  final character (。！？…) appears at a block boundary

### Requirement: Content-sized bubble geometry
The system SHALL size the pet-bubble host to its content: width clamped between
180 and 340 px following the longest measured line, height following the active
page's line count. The resize SHALL use the native no-activate path and
re-run placement so the bubble still avoids the pet and the visible chat
window.

#### Scenario: Short reply
- **WHEN** a one-line reply arrives
- **THEN** the bubble shrinks to fit that line (within the 180-340px range)

#### Scenario: Long reply with many lines
- **WHEN** a multi-line page is displayed
- **THEN** the bubble height grows to the page's line count and the bubble is
  repositioned without overlapping the pet or visible chat

#### Scenario: Geometry converges
- **WHEN** the same text is displayed repeatedly
- **THEN** the bubble reports the same size every time and never oscillates
