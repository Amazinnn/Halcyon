import { describe, expect, it } from "vitest";
import source from "./ChatView.vue?raw";

describe("simplified chat surface", () => {
  it("keeps only the Agent/status/messages/interrupt/composer controls", () => {
    expect(source).toContain("agent.refreshCharacters()");
    expect(source).toContain("Codex");
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
      "Mock",
    ]) {
      expect(source).not.toContain(removed);
    }
  });

  it("locks selection and sending for the whole active turn and only exposes Stop when it can interrupt", () => {
    expect(source).toContain(':disabled="isBusy"');
    expect(source).toContain(':disabled="isBusy || !agent.characterId"');
    expect(source).toContain("agent.phase === 'streaming'");
    expect(source).not.toContain("agent.phase === 'streaming' || agent.phase === 'connecting'");
  });
});
