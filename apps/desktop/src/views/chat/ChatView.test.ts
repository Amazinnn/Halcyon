import { describe, expect, it } from "vitest";
import source from "./ChatView.vue?raw";

describe("simplified chat surface", () => {
  it("keeps only the Agent/status/messages/interrupt/composer controls", () => {
    expect(source).toContain("agent.refreshCharacters()");
    expect(source).toContain("agent.provider");
    expect(source).toContain("phaseText");
    expect(source).toContain("agent.messages");
    expect(source).toContain("agent.interrupt()");
    expect(source).toContain('class="composer"');
    expect(source).toContain("m.source");

    for (const removed of [
      "新会话",
      "清理自动化",
      "选项",
      "QUICK",
      "focus-cli",
      "workspaceInput",
      "useSkill",
      "tool-strip",
    ]) {
      expect(source).not.toContain(removed);
    }
  });
});
