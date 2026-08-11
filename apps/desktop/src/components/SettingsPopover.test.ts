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

  it("keeps concise setup guidance beside user-facing settings", () => {
    expect(source).toContain('const version = "v1.12.10"');
    expect(source).toContain("支持 PNG、JPG、JPEG、WebP");
    expect(source).toContain("只提醒，不会强制关闭应用");
    expect(source).toContain("pet.json + spritesheet.webp/png");
    expect(source).toContain("1536×1872");
    expect(source).toContain("MP3、FLAC、M4A");
    expect(source).toContain("登录由对应 CLI 自行管理");
    expect(source).toContain("AGPLv3");
  });
});
