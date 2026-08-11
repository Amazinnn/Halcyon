import { describe, expect, it } from "vitest";
import {
  insertSkillToken,
  removeAdjacentSkillToken,
  serializeInlineComposer,
  type InlineComposerPart,
} from "./inline-composer";

describe("inline Skill composer", () => {
  it("keeps stacked Skill tokens in their visible order when serializing the user message", () => {
    const parts: InlineComposerPart[] = [
      { kind: "skill", name: "focus-cli" },
      { kind: "text", text: "  " },
      { kind: "skill", name: "readme" },
      { kind: "text", text: "  查询状态" },
    ];

    expect(serializeInlineComposer(parts)).toBe("$focus-cli  $readme  查询状态");
  });

  it("inserts at the caret and removes only the adjacent atomic Skill token", () => {
    const initial: InlineComposerPart[] = [{ kind: "text", text: "查询状态" }];
    const inserted = insertSkillToken(initial, 0, "focus-cli");

    expect(serializeInlineComposer(inserted)).toBe("$focus-cli  查询状态");
    expect(serializeInlineComposer(removeAdjacentSkillToken(inserted, 1, "backward"))).toBe("查询状态");
    expect(serializeInlineComposer(removeAdjacentSkillToken(inserted, 0, "forward"))).toBe("查询状态");
  });
});
