import { describe, expect, it } from "vitest";
import source from "./SettingsPopover.vue?raw";

describe("real Codex provider settings", () => {
  it("shows only Codex availability and no Mock control or fallback wording", () => {
    expect(source).toContain("已找到 Codex");
    expect(source).toContain("未找到 Codex");
    expect(source).toContain("agent.ready");
    expect(source).not.toContain("Mock");
    expect(source).not.toContain("setAgentProvider");
    expect(source).not.toContain("agent.fallback");
  });
});
