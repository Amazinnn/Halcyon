import { describe, expect, it } from "vitest";
import source from "./WorkflowView.vue?raw";

describe("external workflow editor refresh", () => {
  it("reloads only the draft named by an affected external change", () => {
    expect(source).toContain("store.lastExternalChange");
    expect(source).toMatch(
      /change\?\.affectsCurrentDraft[\s\S]*change\.workflowId !== editingId\.value[\s\S]*store\.workflows\.find\([\s\S]*loadDraft\(wf \?\? null\)/,
    );
  });
});
