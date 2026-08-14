import { describe, expect, it } from "vitest";
import {
  acceptBubbleDelivery,
  BubbleVisibilityRequest,
  bubbleDisplayDurationMs,
  bubbleShouldBeVisible,
  nextBubblePage,
  layoutBubblePages,
  paginateBubbleText,
  paginateBubbleTextMeasured,
} from "./pet-bubble";

describe("pet bubble pagination", () => {
  it("accepts an immediate delivery for the local bubble window without an Agent-store initialization", () => {
    const received = acceptBubbleDelivery("char-a", [], {
      deliveryId: "reply-1",
      agentId: "char-a",
      text: "第一条真实回复",
      priority: "normal",
    });

    expect(received.message?.text).toBe("第一条真实回复");
    expect(received.seenDeliveryIds).toEqual(["reply-1"]);
  });

  it("keeps the local bubble window from replaying a delivery or accepting another Agent's reply", () => {
    const duplicate = acceptBubbleDelivery("char-a", ["reply-1"], {
      deliveryId: "reply-1", agentId: "char-a", text: "重复", priority: "normal",
    });
    const otherAgent = acceptBubbleDelivery("char-a", [], {
      deliveryId: "reply-2", agentId: "char-b", text: "别人的回复", priority: "normal",
    });

    expect(duplicate.message).toBeNull();
    expect(otherAgent.message).toBeNull();
  });

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

describe("sentence-complete bubble layout (requirement #124)", () => {
  const cjkMeasure = (s: string) => s.length * 14;

  it("shows a whole short paragraph regardless of sentence count", () => {
    const { pages, width } = layoutBubblePages("你好。谢谢。再见。", cjkMeasure);
    expect(pages).toEqual([["你好。谢谢。再见。"]]);
    expect(width).toBeGreaterThanOrEqual(180);
  });

  it("splits a long paragraph at sentence finals and never mid-sentence", () => {
    const long = "第一句是完整的陈述。第二句带感叹号！第三句呢？后面还有很长的一段文字需要继续补充完整直到超出宽度限制为止。";
    const { pages } = layoutBubblePages(long, cjkMeasure);
    const flat = pages.flat().join("");
    expect(flat).toBe(long);
    for (const page of pages) {
      const text = page.join("");
      expect(text.endsWith("。") || text.endsWith("！") || text.endsWith("？") || text === "" || /^第一句/.test(text)).toBe(true);
    }
    expect(pages.every((page) => page.length <= 4)).toBe(true);
  });

  it("keeps authored paragraphs as separate blocks", () => {
    const { pages } = layoutBubblePages("短段一。\n短段二。", cjkMeasure);
    expect(pages.flat().join("")).toBe("短段一。短段二。");
  });

  it("clamps width to the 180-340 range and grows height by lines", () => {
    const tiny = layoutBubblePages("好", cjkMeasure);
    expect(tiny.width).toBeGreaterThanOrEqual(180);
    expect(tiny.width).toBeLessThanOrEqual(340);
    const tall = layoutBubblePages("这是一个没有标点符号的长句用来验证宽度上限应当被钳制在三百四十像素处不多不少。", cjkMeasure);
    expect(tall.width).toBeGreaterThan(300);
    expect(tall.width).toBeLessThanOrEqual(340);
    expect(tall.height).toBeGreaterThan(60);
  });

  it("converges: identical input yields identical layout", () => {
    const a = layoutBubblePages("稳定的文本。再来一句。", cjkMeasure);
    const b = layoutBubblePages("稳定的文本。再来一句。", cjkMeasure);
    expect(a).toEqual(b);
  });

  it("rotates only when content spans multiple pages", () => {
    const short = layoutBubblePages("一句话。", cjkMeasure);
    expect(short.pages.length).toBe(1);
    const multi = layoutBubblePages("第一句。第二句。第三句。第四句。第五句。第六句。", cjkMeasure);
    expect(multi.pages.length).toBeGreaterThan(1);
  });

  it("keeps sentence-final punctuation attached to the previous line", () => {
    const long = "这是一个非常长的句子用来测试句号是否会被单独拆到下一行造成孤立标点。";
    const { pages } = layoutBubblePages(long, cjkMeasure);
    const lines = pages.flat();
    for (let i = 0; i < lines.length; i++) {
      if (lines[i] === "。") throw new Error("lone sentence-final punctuation on line " + i);
      if (lines[i].startsWith("。")) throw new Error("line starts with sentence-final punctuation: " + lines[i]);
    }
    expect(lines.join("")).toBe(long);
  });
});
