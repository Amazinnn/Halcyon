import { describe, expect, it } from "vitest";
import source from "./SettingsPopover.vue?raw";

describe("per-Agent provider settings", () => {
  it("renders one compact Codex/Claude selector in each Agent management row", () => {
    expect(source).toContain(':value="a.tool"');
    expect(source).toContain('@change="onAgentProviderChange(a.id, $event)"');
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

  it("refreshes the row after a rejected provider switch", () => {
    const setProvider = source.slice(
      source.indexOf("async function setAgentProvider"),
      source.indexOf("async function deleteAgent"),
    );
    expect(setProvider).toContain("catch (e) {");
    expect(setProvider).toContain("await refreshAgents();");
    expect(setProvider).toContain("if (id === agent.characterId)");
    expect(setProvider).toContain("await agent.refreshStatus();");
  });
});
