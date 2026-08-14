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
