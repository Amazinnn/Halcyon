import { describe, expect, it } from "vitest";
import source from "./SettingsPopover.vue?raw";

describe("per-Agent provider settings", () => {
  it("renders one compact Codex/Claude selector in each Agent management row", () => {
    expect(source).toContain('v-model="a.tool"');
    expect(source).toContain('@change="setAgentProvider(a.id, a.tool)"');
    expect(source).toContain('<option value="codex">Codex</option>');
    expect(source).toContain('<option value="claude">Claude</option>');
    expect(source).toContain("agent.setProvider");
    expect(source).toContain("await refreshAgents()");
    expect(source).toContain("await agent.refreshStatus()");
  });

  it("has no Mock fallback behavior", () => {
    expect(source).not.toContain("Mock");
    expect(source).not.toContain("agent.fallback");
  });
});
