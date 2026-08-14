import { describe, expect, it } from "vitest";
import source from "./TopbarView.vue?raw";

describe("global glass opacity in the topbar capsule", () => {
  it("scales the pill backdrop from the shared --glass-opacity variable", () => {
    expect(source).toContain("--glass-opacity");
    expect(source).toContain("rgb(12 24 17 / var(--glass-opacity, 0.84))");
  });

  it("listens for settings:acrylic-changed and re-applies the opacity", () => {
    expect(source).toContain("settings:acrylic-changed");
    expect(source).toContain('e.payload.opacity');
    expect(source).toContain("document.documentElement.style.setProperty");
    expect(source).toContain("0.84 * factor");
  });

  it("reads the persisted opacity from bootstrap", () => {
    expect(source).toContain("acrylicOpacity");
    expect(source).toContain("get_bootstrap");
  });
});
