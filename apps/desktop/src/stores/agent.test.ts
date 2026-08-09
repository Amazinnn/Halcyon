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
      if (command === "characters_list") {
        return [
          { id: "char-a", name: "小专" },
          { id: "char-b", name: "小助" },
        ];
      }
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

  it("delivers one pending workflow result and bubble when its target Agent is selected", async () => {
    const agent = useAgentStore();
    await agent.init();
    agent.messages = [];

    const resultHandler = handlers.get("workflow:agent_result");
    const bubbleHandler = handlers.get("bubble:requested");
    expect(resultHandler).toBeDefined();
    expect(bubbleHandler).toBeDefined();
    if (!resultHandler || !bubbleHandler) return;

    resultHandler({
      payload: {
        workflowId: "wf-other",
        workflowName: "别人的日程",
        agentId: "char-b",
        text: "不应出现",
      },
    });
    bubbleHandler({
      payload: { text: "不应出现", priority: "normal", agentId: "char-b" },
    });
    expect(agent.messages).toEqual([]);
    expect(agent.bubble).toBeNull();

    await agent.selectCharacter("char-b");

    expect(agent.messages.filter((message) => message.source)).toEqual([
      {
        role: "agent",
        text: "不应出现",
        kind: "completed",
        source: "日程 · 别人的日程",
      },
    ]);
    expect(agent.bubble).toMatchObject({ text: "不应出现", priority: "normal" });

    await agent.selectCharacter("char-a");
    await agent.selectCharacter("char-b");
    expect(agent.messages.filter((message) => message.source)).toEqual([]);
  });
});
