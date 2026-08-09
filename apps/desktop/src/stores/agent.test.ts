import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const { emit, invoke, listen } = vi.hoisted(() => ({
  emit: vi.fn(),
  invoke: vi.fn(),
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ emit, listen }));

import { useAgentStore } from "./agent";

describe("workflow result messages", () => {
  const handlers = new Map<string, ((event: { payload: unknown }) => void)[]>();
  const storage = new Map<string, string>();

  beforeEach(() => {
    setActivePinia(createPinia());
    handlers.clear();
    storage.clear();
    storage.set("focus-agent", "char-a");
    invoke.mockReset();
    emit.mockReset();
    listen.mockReset();
    listen.mockImplementation(async (event: string, handler: (event: { payload: unknown }) => void) => {
      (handlers.get(event) ?? handlers.set(event, []).get(event)!).push(handler);
      return () => undefined;
    });
    emit.mockImplementation(async (event: string, payload: unknown) => {
      for (const handler of handlers.get(event) ?? []) handler({ payload });
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

    const resultHandler = handlers.get("workflow:agent_result")?.[0];
    const bubbleHandler = handlers.get("bubble:requested")?.[0];
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

  it("shows a pending target Agent result in the separate Pet store after Chat broadcasts selection", async () => {
    const chatPinia = createPinia();
    setActivePinia(chatPinia);
    const chat = useAgentStore();
    await chat.init();

    const petPinia = createPinia();
    setActivePinia(petPinia);
    const pet = useAgentStore();
    await pet.init();
    expect(chat.characterId).toBe("char-a");
    expect(pet.characterId).toBe("char-a");

    await emit("workflow:agent_result", {
      workflowId: "wf-char-b",
      workflowName: "Targeted result",
      agentId: "char-b",
      text: "Only the selected pet should show this",
    });
    expect(pet.bubble).toBeNull();

    setActivePinia(chatPinia);
    await chat.selectCharacter("char-b");

    expect(pet.characterId).toBe("char-b");
    expect(pet.bubble).toMatchObject({ text: "Only the selected pet should show this", priority: "normal" });
  });
});
