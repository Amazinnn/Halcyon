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

  it("publishes an external change only after refreshed same-ID data is available", async () => {
    const store = useWorkflowStore();
    await store.init();

    expect(invoke).toHaveBeenCalledWith("workflow_list", { characterId: "" });
    expect(store.workflows[0]?.name).toBe("wf-1");
    expect(store.lastExternalChange).toBeNull();

    const handler = handlers.get("workflow:changed");
    expect(handler).toBeDefined();
    if (!handler) return;

    listed = [workflow("wf-1", "Agent 更新后的名字")];
    handler({ payload: { action: "updated", workflowId: "wf-1" } });

    await vi.waitFor(() => {
      expect(store.workflows[0]?.name).toBe("Agent 更新后的名字");
    });
    expect(store.lastExternalChange).toBeNull();
    expect(store.workflows[0]?.name).toBe("Agent 更新后的名字");
    const listCalls = invoke.mock.calls.filter(([command]) => command === "workflow_list");
    expect(listCalls).toEqual([
      ["workflow_list", { characterId: "" }],
      ["workflow_list", { characterId: "" }],
    ]);
  });

  it("keeps external draft-refresh signals scoped to the selected workflow", async () => {
    storage.set("focus.workflow.currentWorkflowId", "wf-a");
    listed = [workflow("wf-a", "正在编辑"), workflow("wf-b", "另一个日程")];
    const store = useWorkflowStore();
    await store.init();
    const handler = handlers.get("workflow:changed");
    expect(handler).toBeDefined();
    if (!handler) return;

    listed = [workflow("wf-a", "正在编辑"), workflow("wf-b", "由 Agent 更新")];
    handler({ payload: { action: "updated", workflowId: "wf-b" } });
    await vi.waitFor(() => {
      expect(store.workflows.find((item) => item.id === "wf-b")?.name).toBe("由 Agent 更新");
    });
    expect(store.lastExternalChange).toBeNull();

    listed = [workflow("wf-a", "当前日程已更新"), workflow("wf-b", "由 Agent 更新")];
    handler({ payload: { action: "updated", workflowId: "wf-a" } });
    await vi.waitFor(() => {
      expect(store.lastExternalChange).toEqual({
        workflowId: "wf-a",
        action: "updated",
        affectsCurrentDraft: true,
      });
    });

    listed = [workflow("wf-b", "由 Agent 更新")];
    handler({ payload: { action: "deleted", workflowId: "wf-a" } });
    await vi.waitFor(() => {
      expect(store.lastExternalChange).toEqual({
        workflowId: "wf-a",
        action: "deleted",
        affectsCurrentDraft: true,
      });
    });
    expect(store.currentWorkflowId).toBeNull();
  });

  it("keeps a selected draft signal when an unrelated external change follows in the same tick", async () => {
    storage.set("focus.workflow.currentWorkflowId", "wf-a");
    listed = [workflow("wf-a", "正在编辑"), workflow("wf-b", "另一个日程")];
    const store = useWorkflowStore();
    await store.init();
    const handler = handlers.get("workflow:changed");
    expect(handler).toBeDefined();
    if (!handler) return;

    listed = [workflow("wf-a", "当前日程已更新"), workflow("wf-b", "另一个日程已更新")];
    handler({ payload: { action: "updated", workflowId: "wf-a" } });
    handler({ payload: { action: "updated", workflowId: "wf-b" } });

    await vi.waitFor(() => {
      expect(store.workflows.find((item) => item.id === "wf-b")?.name).toBe("另一个日程已更新");
      expect(store.lastExternalChange).toEqual({
        workflowId: "wf-a",
        action: "updated",
        affectsCurrentDraft: true,
      });
    });
  });

  it("keeps an interleaved external update visible and consumes only the matching local event", async () => {
    storage.set("focus.workflow.currentWorkflowId", "wf-1");
    const store = useWorkflowStore();
    await store.init();
    const handler = handlers.get("workflow:changed");
    expect(handler).toBeDefined();
    if (!handler) return;

    let finishSave!: (saved: WorkflowDef) => void;
    const pendingSave = new Promise<WorkflowDef>((resolve) => {
      finishSave = resolve;
    });
    invoke.mockImplementation(async (command: string) => {
      if (command === "workflow_save") return pendingSave;
      if (command === "workflow_list") return listed;
      if (command === "workflow_runs") return [];
      return [];
    });

    const local = workflow("wf-local", "本地保存");
    const saving = store.save(local);
    await vi.waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("workflow_save", { workflow: local });
    });

    listed = [workflow("wf-1", "外部更新"), local];
    handler({ payload: { action: "updated", workflowId: "wf-1" } });
    await vi.waitFor(() => {
      expect(store.lastExternalChange).toEqual({
        workflowId: "wf-1",
        action: "updated",
        affectsCurrentDraft: true,
      });
    });
    expect(store.currentWorkflowId).toBe("wf-1");
    expect(store.workflows.find((item) => item.id === "wf-1")?.name).toBe("外部更新");

    finishSave(local);
    await saving;
    handler({ payload: { action: "updated", workflowId: "wf-local" } });
    await vi.waitFor(() => {
      expect(invoke.mock.calls.filter(([command]) => command === "workflow_list")).toHaveLength(4);
    });
    expect(store.lastExternalChange).toEqual({
      workflowId: "wf-1",
      action: "updated",
      affectsCurrentDraft: true,
    });
  });

  it("does not invent a local reservation for a create with an unknown generated ID", async () => {
    const store = useWorkflowStore();
    await store.init();
    const handler = handlers.get("workflow:changed");
    expect(handler).toBeDefined();
    if (!handler) return;

    const draft = workflow("", "新建日程");
    const saved = workflow("wf-created", "新建日程");
    invoke.mockImplementation(async (command: string) => {
      if (command === "workflow_save") return saved;
      if (command === "workflow_list") return [saved];
      if (command === "workflow_runs") return [];
      return [];
    });

    await store.save(draft);
    handler({ payload: { action: "created", workflowId: "wf-created" } });

    await vi.waitFor(() => {
      expect(store.lastExternalChange).toEqual({
        workflowId: "wf-created",
        action: "created",
        affectsCurrentDraft: true,
      });
    });
  });
});
