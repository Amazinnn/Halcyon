import { describe, expect, it } from "vitest";
import { defaultParams, emptyWorkflow, type WorkflowDef } from "./workflow";

type AgentDefaults = (
  kind: string,
  context?: {
    characters: { id: string }[];
    persistedAgentId: string | null;
  },
) => Record<string, unknown>;

describe("workflow draft defaults", () => {
  const characters = [{ id: "char-a" }, { id: "char-b" }];

  it("targets the persisted chat Agent when it still exists", () => {
    const params = (defaultParams as AgentDefaults)("agent", {
      characters,
      persistedAgentId: "char-b",
    });

    expect(params).toMatchObject({ characterId: "char-b", showResult: true });
    expect(params).not.toHaveProperty("showInitial");
    expect(params).not.toHaveProperty("showThinking");
  });

  it("falls back to the first character when the persisted Agent is unavailable", () => {
    const params = (defaultParams as AgentDefaults)("agent", {
      characters,
      persistedAgentId: "missing",
    });

    expect(params.characterId).toBe("char-a");
  });

  it("creates a unified workflow with no workflow-level character", () => {
    const draft = (emptyWorkflow as unknown as (name?: string) => WorkflowDef)();

    expect(draft.characterId).toBe("");
  });
});
