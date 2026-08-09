import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import type { WorkflowDef } from "../lib/workflow";

const { invoke, listen } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

import { useWorkflowStore } from "./workflow";

function workflow(id: string): WorkflowDef {
  return {
    id,
    characterId: "",
    name: id,
    trigger: "manual",
    guard: "none",
    nodes: [],
    edges: [],
    enabled: true,
  };
}

describe("unified workflow list", () => {
  const handlers = new Map<string, (event: { payload: unknown }) => void>();
  const storage = new Map<string, string>();
  let listed = [workflow("wf-1")];

  beforeEach(() => {
    setActivePinia(createPinia());
    handlers.clear();
    storage.clear();
    listed = [workflow("wf-1")];
    invoke.mockReset();
    listen.mockReset();
    listen.mockImplementation(async (event: string, handler: (event: { payload: unknown }) => void) => {
      handlers.set(event, handler);
      return () => undefined;
    });
    invoke.mockImplementation(async (command: string) => {
      if (command === "characters_list") return [{ id: "char-a", name: "小专" }];
      if (command === "workflow_list") return listed;
      return [];
    });
    Object.assign(globalThis, {
      localStorage: {
        getItem: (key: string) => storage.get(key) ?? null,
        setItem: (key: string, value: string) => storage.set(key, value),
      },
    });
  });

  it("loads every workflow and refreshes the list after workflow:changed", async () => {
    const store = useWorkflowStore();
    await store.init();

    expect(invoke).toHaveBeenCalledWith("workflow_list", { characterId: "" });
    expect(store.workflows.map((item) => item.id)).toEqual(["wf-1"]);

    const handler = handlers.get("workflow:changed");
    expect(handler).toBeDefined();
    if (!handler) return;

    listed = [workflow("wf-1"), workflow("wf-2")];
    handler({ payload: { action: "created", workflowId: "wf-2" } });

    await vi.waitFor(() => {
      expect(store.workflows.map((item) => item.id)).toEqual(["wf-1", "wf-2"]);
    });
    const listCalls = invoke.mock.calls.filter(([command]) => command === "workflow_list");
    expect(listCalls).toEqual([
      ["workflow_list", { characterId: "" }],
      ["workflow_list", { characterId: "" }],
    ]);
  });
});
