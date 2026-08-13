import { describe, expect, it } from "vitest";
import {
  BubbleVisibilityRequest,
  bubbleDisplayDurationMs,
  bubbleShouldBeVisible,
  nextBubblePage,
  paginateBubbleText,
  paginateBubbleTextMeasured,
} from "./pet-bubble";

describe("pet bubble pagination", () => {
  it("keeps authored lines and never returns more than two lines per page", () => {
    const pages = paginateBubbleText("第一行\n第二行\n第三行", 8);
    expect(pages).toEqual([["第一行", "第二行"], ["第三行"]]);
  });

  it("wraps long text at natural boundaries without ellipsis", () => {
    const pages = paginateBubbleText("今天完成阅读，然后休息一下。", 8);
    expect(pages.flat().join("")).toBe("今天完成阅读，然后休息一下。");
    expect(pages.every((page) => page.length <= 2)).toBe(true);
    expect(pages.flat().every((line) => line.length <= 8)).toBe(true);
  });

  it("uses measured text width rather than a fixed character count", () => {
    const pages = paginateBubbleTextMeasured(
      "宽字 窄字 需要按实际宽度分页",
      (text) => text.replace(/宽/g, "WW").length * 10,
      80,
    );

    expect(pages.flat().join("").replace(/ /g, "")).toBe("宽字窄字需要按实际宽度分页");
    expect(pages.every((page) => page.length <= 2)).toBe(true);
    expect(pages.flat().every((line) => line.replace(/宽/g, "WW").length * 10 <= 80)).toBe(true);
  });

  it("keeps every page visible for a full three-second turn", () => {
    expect(bubbleDisplayDurationMs([["one"]])).toBe(3000);
    expect(bubbleDisplayDurationMs([["one"], ["two"], ["three"]])).toBe(9000);
  });

  it("resets a new message to its first page before rotating", () => {
    expect(nextBubblePage(2, 3)).toBe(0);
    expect(nextBubblePage(0, 3)).toBe(1);
    expect(nextBubblePage(2, 3)).toBe(0);
  });

  it("keeps a queued message visible while chat is open until it expires", () => {
    expect(bubbleShouldBeVisible({ hasMessage: true, dragging: false, now: 100, expiresAt: 101 })).toBe(true);
    expect(bubbleShouldBeVisible({ hasMessage: true, dragging: false, now: 101, expiresAt: 101 })).toBe(false);
    expect(bubbleShouldBeVisible({ hasMessage: false, dragging: false, now: 100, expiresAt: 101 })).toBe(false);
  });

  it("suppresses every show attempt while the pet is being dragged", () => {
    expect(bubbleShouldBeVisible({ hasMessage: true, dragging: true, now: 100, expiresAt: 1000 })).toBe(false);
  });

  it("invalidates an in-flight show when a hide or replacement starts", () => {
    const requests = new BubbleVisibilityRequest();
    const showing = requests.issue();
    const hiding = requests.issue();

    expect(requests.isCurrent(showing)).toBe(false);
    expect(requests.isCurrent(hiding)).toBe(true);

    requests.invalidate();
    expect(requests.isCurrent(hiding)).toBe(false);
  });
});
