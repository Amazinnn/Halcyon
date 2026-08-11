import { describe, expect, it } from "vitest";
import { composeSkillMessage, shouldRemoveSelectedSkill } from "./chat-composer";

describe("visible Skill composer", () => {
  it("prefixes the direct user message", () => {
    expect(composeSkillMessage("focus-cli", "check status")).toBe("$focus-cli  check status");
    expect(composeSkillMessage(null, "check status")).toBe("check status");
  });

  it("removes the Skill atomically at the input boundary", () => {
    expect(shouldRemoveSelectedSkill("Backspace", "", 0, 0)).toBe(true);
    expect(shouldRemoveSelectedSkill("Delete", "text", 0, 0)).toBe(true);
    expect(shouldRemoveSelectedSkill("Delete", "text", 2, 2)).toBe(false);
  });
});
