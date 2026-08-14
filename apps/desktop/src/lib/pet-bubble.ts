function pagesFromLines(lines: string[]): string[][] {
  const pages: string[][] = [];
  for (let i = 0; i < lines.length; i += 2) pages.push(lines.slice(i, i + 2));
  return pages.length ? pages : [[""]];
}

export class BubbleVisibilityRequest {
  private generation = 0;

  issue(): number {
    this.generation += 1;
    return this.generation;
  }

  isCurrent(generation: number): boolean {
    return generation === this.generation;
  }

  invalidate(): void {
    this.generation += 1;
  }
}

export interface BubbleDelivery {
  deliveryId: string;
  agentId: string;
  text: string;
  priority: string;
}

/**
 * The bubble WebView owns its own delivery state.  It must be able to consume
 * a reply before any full Agent-store bootstrap has completed.
 */
export function acceptBubbleDelivery(
  currentAgentId: string,
  seenDeliveryIds: string[],
  delivery: BubbleDelivery,
): { message: BubbleDelivery | null; seenDeliveryIds: string[] } {
  if (!currentAgentId || delivery.agentId !== currentAgentId || seenDeliveryIds.includes(delivery.deliveryId)) {
    return { message: null, seenDeliveryIds };
  }
  return {
    message: delivery,
    seenDeliveryIds: [...seenDeliveryIds, delivery.deliveryId],
  };
}

export function bubbleDisplayDurationMs(pages: string[][]): number {
  return Math.max(1, pages.length) * 3000;
}

export function nextBubblePage(page: number, pageCount: number): number {
  if (pageCount <= 1) return 0;
  return (page + 1) % pageCount;
}

export function bubbleShouldBeVisible({
  hasMessage,
  dragging,
  now,
  expiresAt,
}: {
  hasMessage: boolean;
  dragging: boolean;
  now: number;
  expiresAt: number;
}): boolean {
  return hasMessage && !dragging && now < expiresAt;
}

export function paginateBubbleText(text: string, maxCharsPerLine = 18): string[][] {
  const lines: string[] = [];
  for (const authored of text.split(/\r?\n/)) {
    let remaining = authored.trim();
    if (!remaining) continue;
    while (remaining.length > maxCharsPerLine) {
      const limit = Math.min(maxCharsPerLine, remaining.length);
      const boundary = Math.max(
        remaining.lastIndexOf("。", limit),
        remaining.lastIndexOf("，", limit),
        remaining.lastIndexOf("、", limit),
        remaining.lastIndexOf(" ", limit),
      );
      const cut = boundary > Math.floor(maxCharsPerLine * 0.55) ? boundary + 1 : limit;
      lines.push(remaining.slice(0, cut).trim());
      remaining = remaining.slice(cut).trim();
    }
    if (remaining) lines.push(remaining);
  }
  return pagesFromLines(lines);
}

/**
 * Split a message according to the renderer's actual font metrics. The view
 * supplies one cheap canvas measurement function, keeping page boundaries
 * deterministic and testable outside the DOM.
 */
export function paginateBubbleTextMeasured(
  text: string,
  measure: (text: string) => number,
  maxWidth: number,
): string[][] {
  const lines: string[] = [];
  const naturalBreak = /[\s\u3002\uff01\uff1f\uff0c\u3001\uff1b\uff1a]/;

  for (const authored of text.split(/\r?\n/)) {
    let remaining = authored.trim();
    while (remaining) {
      if (measure(remaining) <= maxWidth) {
        lines.push(remaining);
        break;
      }

      let end = 1;
      while (end < remaining.length && measure(remaining.slice(0, end + 1)) <= maxWidth) end++;
      let boundary = 0;
      for (let i = end - 1; i >= Math.floor(end * 0.55); i--) {
        if (naturalBreak.test(remaining[i])) {
          boundary = i + 1;
          break;
        }
      }
      const cut = boundary || end;
      lines.push(remaining.slice(0, cut).trim());
      remaining = remaining.slice(cut).trim();
    }
  }

  return pagesFromLines(lines);
}

// ---------------------------------------------------------------------------
// Requirement #124: sentence-complete pages and content-sized geometry.
// ---------------------------------------------------------------------------

export const BUBBLE_MAX_WIDTH = 340;
export const BUBBLE_MIN_WIDTH = 180;
export const BUBBLE_MAX_LINES_PER_PAGE = 4;
export const BUBBLE_PADDING_X = 15;
export const BUBBLE_LINE_HEIGHT = 20; // 14px font x 1.4 line height
export const BUBBLE_VERTICAL_INSET = 31; // css padding 11+12 + tail room 8

export interface BubbleLayout {
  /** One page per array of lines; a line never contains a partial sentence
   *  start (sentence-final characters only appear at block/page boundaries). */
  pages: string[][];
  /** Content-sized host width, clamped between 180 and 340px. */
  width: number;
  /** Content-sized host height for the tallest page (stable across rotation). */
  height: number;
}

const SENTENCE_FINAL = "。！？…";
const SENTENCE_CLOSERS = "。！？…”」』）】";

function splitParagraphIntoSentences(paragraph: string): string[] {
  const sentences: string[] = [];
  let start = 0;
  let i = 0;
  while (i < paragraph.length) {
    if (SENTENCE_FINAL.includes(paragraph[i])) {
      let end = i + 1;
      while (end < paragraph.length && SENTENCE_CLOSERS.includes(paragraph[end])) end++;
      const sentence = paragraph.slice(start, end).trim();
      if (sentence) sentences.push(sentence);
      start = end;
      i = end;
    } else {
      i += 1;
    }
  }
  const tail = paragraph.slice(start).trim();
  if (tail) sentences.push(tail);
  return sentences;
}

function wrapBlock(block: string, measure: (text: string) => number, maxWidth: number): string[] {
  const lines: string[] = [];
  let remaining = block.trim();
  if (!remaining) return [""];
  while (remaining) {
    if (measure(remaining) <= maxWidth) {
      lines.push(remaining);
      break;
    }
    let end = 1;
    while (end < remaining.length && measure(remaining.slice(0, end + 1)) <= maxWidth) end++;
    let boundary = 0;
    for (let i = end - 1; i >= Math.floor(end * 0.55); i--) {
      if (/[\s\u3002\uff01\uff1f\uff0c\u3001\uff1b\uff1a]/.test(remaining[i])) {
        boundary = i + 1;
        break;
      }
    }
    let cut = boundary || end;
    // Keep sentence-final punctuation attached to the line (requirement #124:
    // a lone 。 on its own line looks like truncation).
    if (cut < remaining.length && SENTENCE_FINAL.includes(remaining[cut])) {
      cut += 1;
    }
    lines.push(remaining.slice(0, cut).trim());
    remaining = remaining.slice(cut).trim();
  }
  return lines.length ? lines : [""];
}

/**
 * Paginate a reply into sentence-complete pages (requirement #124):
 * - a natural paragraph that fits on one measured line is shown whole;
 * - a longer paragraph is split at sentence-final characters;
 * - pages hold at most 4 lines and a sentence block never starts mid-sentence;
 * - the layout also reports content-sized width/height for the host.
 */
export function layoutBubblePages(
  text: string,
  measure: (text: string) => number,
  maxWidth = BUBBLE_MAX_WIDTH - 2 * BUBBLE_PADDING_X,
  maxLinesPerPage = BUBBLE_MAX_LINES_PER_PAGE,
): BubbleLayout {
  const paragraphs = text.split(/\r?\n/).map((p) => p.trim()).filter(Boolean);
  if (!paragraphs.length) {
    return { pages: [[""]], width: BUBBLE_MIN_WIDTH, height: BUBBLE_VERTICAL_INSET };
  }
  const blocks: string[] = [];
  for (const paragraph of paragraphs) {
    if (measure(paragraph) <= maxWidth) {
      blocks.push(paragraph);
    } else {
      for (const sentence of splitParagraphIntoSentences(paragraph)) blocks.push(sentence);
    }
  }
  const pages: string[][] = [];
  let current: string[] = [];
  for (const block of blocks) {
    const lines = wrapBlock(block, measure, maxWidth);
    for (let i = 0; i < lines.length; i += maxLinesPerPage) {
      const chunk = lines.slice(i, i + maxLinesPerPage);
      if (current.length && current.length + chunk.length > maxLinesPerPage) {
        pages.push(current);
        current = [];
      }
      current.push(...chunk);
    }
  }
  if (current.length) pages.push(current);
  if (!pages.length) pages.push([""]);

  let longest = 0;
  let tallest = 0;
  for (const page of pages) {
    tallest = Math.max(tallest, page.length);
    for (const line of page) longest = Math.max(longest, measure(line));
  }
  const width = Math.min(BUBBLE_MAX_WIDTH, Math.max(BUBBLE_MIN_WIDTH, longest + 2 * BUBBLE_PADDING_X));
  const height = tallest * BUBBLE_LINE_HEIGHT + BUBBLE_VERTICAL_INSET;
  return { pages, width, height };
}
