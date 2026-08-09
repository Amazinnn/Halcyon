import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const { invoke, listen } = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen }));

import { useAgentStore } from "./agent";

describe("workflow result messages", () => {
  const handlers = new Map<string, (event: { payload: unknown }) => void>();
  const storage = new Map<string, string>();

  beforeEach(() => {
    setActivePinia(createPinia());
    handlers.clear();
    storage.clear();
    storage.set("focus-agent", "char-a");
    invoke.mockReset();
    listen.mockReset();
    listen.mockImplementation(async (event: string, handler: (event: { payload: unknown }) => void) => {
      handlers.set(event, handler);
      return () => undefined;
    });
    invoke.mockImplementation(async (command: string) => {
      if (command === "characters_list") return [{ id: "char-a", name: "小专" }];
      if (command === "agent_status") {
        return {
          provider: "codex",
          fallback: false,
          ready: true,
          exePath: null,
          workspaceDir: "D:\\Agent",
        };
      }
      if (command === "agent_list_skills") return [];
      return undefined;
    });
    Object.assign(globalThis, {
      localStorage: {
        getItem: (key: string) => storage.get(key) ?? null,
        setItem: (key: string, value: string) => storage.set(key, value),
      },
    });
  });

  it("shows one labeled result only for the selected Agent", async () => {
    const agent = useAgentStore();
    await agent.init();
    agent.messages = [];

    const handler = handlers.get("workflow:agent_result");
    expect(handler).toBeDefined();
    if (!handler) return;

    handler({
      payload: {
        workflowId: "wf-other",
        workflowName: "别人的日程",
        agentId: "char-b",
        text: "不应出现",
      },
    });
    expect(agent.messages).toEqual([]);

    handler({
      payload: {
        workflowId: "wf-morning",
        workflowName: "晨间整理",
        agentId: "char-a",
        text: "整理完成",
      },
    });
    expect(agent.messages).toEqual([
      {
        role: "agent",
        text: "整理完成",
        kind: "completed",
        source: "日程 · 晨间整理",
      },
    ]);
  });
});
