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
    expect(source).toContain("黑名单应用在专注时只提醒，不会强制关闭");
    expect(source).toContain('format 为 focus-hatch-pet');
    expect(source).toContain("官方 Hatch Pet 原样 pet.json");
    expect(source).not.toContain("hatch-pet-draft-0.2");
    expect(source).toContain("pet_get_state_mapping");
    expect(source).toContain("MP3、FLAC、M4A");
    expect(source).toContain("Provider 登录由 Codex 或 Claude CLI 自行管理");
    expect(source).toContain("AGPLv3");
  });

  it("removes retired task, preset, allow-list, and supervision controls", () => {
    expect(source).not.toContain("PRESETS");
    expect(source).not.toContain("saveTask");
    expect(source).not.toContain("setCurrentTask");
    expect(source).not.toContain("allowedApps");
    expect(source).not.toContain("setSupervisionEnabled");
  });

  it("keeps the current Agent pet-state mapping collapsed and package-driven", () => {
    expect(source).toContain("pet_get_state_mapping");
    expect(source).toContain("pet_save_state_mapping");
    expect(source).toContain("stateMappingOpen");
    expect(source).toContain("petAnimations");
    expect(source).toContain("resting");
    expect(source).toContain("troubled");
  });

  it("offers package-scoped pet correction and non-blocking quality warnings", () => {
    expect(source).toContain("宽高校正");
    expect(source).toContain('min="0.75"');
    expect(source).toContain('max="1.33"');
    expect(source).toContain('step="0.01"');
    expect(source).toContain("pet_set_horizontal_correction");
    expect(source).toContain("qualityWarnings");
  });
});
