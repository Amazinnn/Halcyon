import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "./settings";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

describe("global glass opacity settings", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.mocked(invoke).mockReset();
  });

  it("loads the persisted opacity from bootstrap", async () => {
    vi.mocked(invoke).mockResolvedValue({ acrylicOpacity: 61 });
    const settings = useSettingsStore();
    await settings.load();
    expect(settings.acrylicOpacity).toBe(61);
  });

  it("defaults to 22 when bootstrap omits the value", async () => {
    vi.mocked(invoke).mockResolvedValue({});
    const settings = useSettingsStore();
    await settings.load();
    expect(settings.acrylicOpacity).toBe(22);
  });

  it("clamps and persists through set_acrylic_opacity", async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    const settings = useSettingsStore();
    await settings.setAcrylicOpacity(140);
    expect(invoke).toHaveBeenCalledWith("set_acrylic_opacity", { opacity: 100 });
    expect(settings.acrylicOpacity).toBe(100);
    await settings.setAcrylicOpacity(2);
    expect(invoke).toHaveBeenCalledWith("set_acrylic_opacity", { opacity: 5 });
    expect(settings.acrylicOpacity).toBe(5);
  });
});
