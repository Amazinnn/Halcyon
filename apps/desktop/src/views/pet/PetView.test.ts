import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const source = readFileSync(new URL("./PetView.vue", import.meta.url), "utf8");

describe("pet canvas resize lifecycle", () => {
  it("observes the stable stage and refits after async pet DOM updates", () => {
    expect(source).toContain('import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue"');
    expect(source).toContain("const stageRef = ref<HTMLElement | null>(null);");
    expect(source).toContain("resizeObserver.observe(stageRef.value);");
    expect(source).toContain("await nextTick();\n  observePetStage();\n  fitCanvas();");
    expect(source).toContain('<div ref="stageRef" class="pet-stage">');
  });

  it("renders an explicit translucent primary-color glass surface behind the sprite", () => {
    expect(source).toContain("linear-gradient(135deg");
    expect(source).toContain("backdrop-filter: blur(18px) saturate(125%)");
    expect(source).toContain("color-mix(in srgb, var(--pet-host-tint, #122018) 58%, transparent)");
  });
});
