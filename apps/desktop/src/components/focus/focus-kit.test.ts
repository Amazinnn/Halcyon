import { describe, expect, it } from "vitest";
import buttonSource from "./FocusButton.vue?raw";
import toggleSource from "./FocusToggle.vue?raw";
import segmentedSource from "./FocusSegmented.vue?raw";
import inputSource from "./FocusInput.vue?raw";
import sliderSource from "./FocusSlider.vue?raw";
import selectSource from "./FocusSelect.vue?raw";
import cardSource from "./FocusCard.vue?raw";
import frameSource from "./FocusWindowFrame.vue?raw";
import { readFileSync } from "node:fs";
const stylesSource = readFileSync(new URL("../../styles.css", import.meta.url), "utf8");

describe("focus kit controls", () => {
  it("FocusButton declares every variant and size", () => {
    for (const v of ["default", "glass", "ghost", "accent", "danger"]) {
      expect(buttonSource).toContain(`v-${v}`);
    }
    for (const s of ["tight", "xs", "sm", "md", "lg", "icon"]) {
      expect(buttonSource).toContain(`s-${s}`);
    }
    expect(buttonSource).toContain(":disabled");
  });

  it("FocusToggle binds modelValue and exposes on state", () => {
    expect(toggleSource).toContain("update:modelValue");
    expect(toggleSource).toContain("aria-pressed");
    expect(toggleSource).toContain("class=\"focus-toggle\"");
    expect(toggleSource).toContain(".focus-toggle.on");
  });

  it("FocusSegmented supports soft/solid/pill variants", () => {
    expect(segmentedSource).toContain('variant ?? \'soft\'');
    expect(segmentedSource).toContain(".focus-seg.solid");
    expect(segmentedSource).toContain(".focus-seg.pill");
    expect(segmentedSource).toContain("update:modelValue");
  });

  it("FocusInput/FocusSlider/FocusSelect are native inputs with glass style", () => {
    expect(inputSource).toContain("class=\"focus-input\"");
    expect(sliderSource).toContain("type=\"range\"");
    expect(sliderSource).toContain("accent-color: var(--accent)");
    expect(sliderSource).toContain(":disabled=\"disabled\"");
    expect(selectSource).toContain("class=\"focus-select\"");
    expect(selectSource).toContain(":disabled=\"disabled\"");
    expect(inputSource).toContain("min-width: var(--ctrl-min-input)");
    expect(inputSource).toContain("autosize");
    expect(inputSource).toContain("field-sizing: content");
    expect(selectSource).toContain("min-width: var(--ctrl-min-select)");
  });

  it("FocusCard and FocusWindowFrame keep header behavior", () => {
    expect(cardSource).toContain("glass");
    expect(frameSource).toContain("set_topmost");
    expect(frameSource).toContain("collapse");
    expect(frameSource).toContain("useGridDrag");
  });

  it("tokens provide control-level values", () => {
    for (const t of ["--fs-xs", "--fs-sm", "--fs-md", "--fs-lg", "--shadow-pop", "--shadow-float", "--z-tray", "--z-popover", "--ctrl-min-input", "--ctrl-min-select", "--text-min-row", "--ctrl-min-input-auto", "--ctrl-max-input-h"]) {
      expect(stylesSource).toContain(t);
    }
  });
});