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

function workflow(id: string, name = id): WorkflowDef {
  return {
    id,
    characterId: "",
    name,
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

  it("publishes an external-change revision after refreshed same-ID data is available", async () => {
    const store = useWorkflowStore();
    await store.init();

    expect(invoke).toHaveBeenCalledWith("workflow_list", { characterId: "" });
    expect(store.workflows[0]?.name).toBe("wf-1");
    expect(store.externalChangeRevision).toBe(0);

    const handler = handlers.get("workflow:changed");
    expect(handler).toBeDefined();
    if (!handler) return;

    listed = [workflow("wf-1", "Agent 更新后的名字")];
    handler({ payload: { action: "updated", workflowId: "wf-1" } });

    await vi.waitFor(() => {
      expect(store.externalChangeRevision).toBe(1);
    });
    expect(store.workflows[0]?.name).toBe("Agent 更新后的名字");
    const listCalls = invoke.mock.calls.filter(([command]) => command === "workflow_list");
    expect(listCalls).toEqual([
      ["workflow_list", { characterId: "" }],
      ["workflow_list", { characterId: "" }],
    ]);
  });

  it("does not publish the external signal for its own save event", async () => {
    const store = useWorkflowStore();
    await store.init();
    const handler = handlers.get("workflow:changed");
    expect(handler).toBeDefined();
    if (!handler) return;

    const saved = workflow("wf-1", "本地保存");
    invoke.mockImplementation(async (command: string) => {
      if (command === "workflow_save") return saved;
      if (command === "workflow_list") return [saved];
      if (command === "workflow_runs") return [];
      return [];
    });

    await store.save(saved);
    handler({ payload: { action: "updated", workflowId: "wf-1" } });

    await vi.waitFor(() => {
      expect(invoke.mock.calls.filter(([command]) => command === "workflow_list")).toHaveLength(3);
    });
    expect(store.externalChangeRevision).toBe(0);
  });
});
