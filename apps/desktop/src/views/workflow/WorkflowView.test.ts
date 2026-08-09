import { describe, expect, it } from "vitest";
import source from "./WorkflowView.vue?raw";

describe("external workflow editor refresh", () => {
  it("reloads the selected draft when the store publishes an external revision", () => {
    expect(source).toContain("store.externalChangeRevision");
    expect(source).toMatch(
      /store\.externalChangeRevision[\s\S]*store\.workflows\.find\([\s\S]*loadDraft\(wf \?\? null\)/,
    );
  });
});
