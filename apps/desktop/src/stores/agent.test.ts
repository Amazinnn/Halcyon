import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const { emit, invoke, listen } = vi.hoisted(() => ({
  emit: vi.fn(),
  invoke: vi.fn(),
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ emit, listen }));

import { chatHistoryKey, useAgentStore } from "./agent";

const handlers = new Map<string, ((event: { payload: unknown }) => void)[]>();
const storage = new Map<string, string>();

function installEventHarness() {
  handlers.clear();
  listen.mockReset();
  listen.mockImplementation(async (event: string, handler: (event: { payload: unknown }) => void) => {
    (handlers.get(event) ?? handlers.set(event, []).get(event)!).push(handler);
    return () => undefined;
  });
  Object.assign(globalThis, {
    localStorage: {
      getItem: (key: string) => storage.get(key) ?? null,
      setItem: (key: string, value: string) => storage.set(key, value),
    },
  });
}

describe("workflow result messages", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    installEventHarness();
    storage.clear();
    storage.set("focus-agent", "char-a");
    invoke.mockReset();
    emit.mockReset();
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
          characterId: "char-a",
          provider: "codex",
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

  it("delivers one pending workflow result without synthesizing a second bubble", async () => {
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
    expect(agent.bubble).toBeNull();

    await agent.selectCharacter("char-a");
    await agent.selectCharacter("char-b");
    expect(agent.messages.filter((message) => message.source)).toHaveLength(1);
  });

  it("keeps pending workflow history separate from the authoritative bubble event", async () => {
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
    expect(pet.bubble).toBeNull();
  });

  it("does not synthesize a bubble from a current Agent workflow history event", async () => {
    const agent = useAgentStore();
    await agent.init();
    const resultHandler = handlers.get("workflow:agent_result")?.[0];
    expect(resultHandler).toBeDefined();
    resultHandler?.({
      payload: {
        workflowId: "wf-current",
        workflowName: "Current result",
        agentId: "char-a",
        text: "history only",
      },
    });
    expect(agent.messages[agent.messages.length - 1]?.source).toBe("日程 · Current result");
    expect(agent.bubble).toBeNull();
  });
});

describe("dated visible history", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    installEventHarness();
    storage.clear();
  });

  it("starts an empty visible history when a message is sent after midnight", () => {
    vi.useFakeTimers();
    try {
      const yesterday = new Date(2026, 7, 10, 23, 59);
      const today = new Date(2026, 7, 11, 0, 1);
      vi.setSystemTime(yesterday);
      storage.set(
        chatHistoryKey("char-a", "codex"),
        JSON.stringify([{ role: "agent", text: "yesterday", kind: "completed" }]),
      );
      const agent = useAgentStore();
      agent.characterId = "char-a";
      agent.provider = "codex";
      agent.restoreVisibleHistory();

      vi.setSystemTime(today);
      agent.addUserMessage("today");

      expect(agent.messages).toEqual([{ role: "user", text: "today", kind: "completed" }]);
      expect(JSON.parse(storage.get(chatHistoryKey("char-a", "codex")) ?? "[]")).toEqual([
        { role: "user", text: "today", kind: "completed" },
      ]);
    } finally {
      vi.useRealTimers();
    }
  });
});

describe("direct chat stream convergence", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    installEventHarness();
    storage.clear();
  });

  it("finalizes the active delta instead of appending a duplicate completed message", () => {
    const agent = useAgentStore();
    agent.messages = [{ role: "agent", text: "正在回答", kind: "delta" }];

    agent.handleEvent({
      schemaVersion: 1,
      agentId: "focus-demo-pet",
      sessionId: "thread-1",
      timestamp: "2026-08-09T00:00:00.000Z",
      event: { type: "message.completed", text: "正在回答" },
    });

    expect(agent.messages).toEqual([{ role: "agent", text: "正在回答", kind: "completed" }]);
    expect(agent.bubble).toBeNull();
  });

  it("accumulates Claude thinking on the live message and keeps it after finalize", () => {
    const agent = useAgentStore();
    agent.messages = [{ role: "agent", text: "", thinking: "", kind: "delta" }];

    agent.handleEvent({
      schemaVersion: 1,
      agentId: "char-a",
      sessionId: "thread-1",
      timestamp: "2026-08-14T00:00:00.000Z",
      event: { type: "message.thinking", text: "先想想" },
    });
    agent.handleEvent({
      schemaVersion: 1,
      agentId: "char-a",
      sessionId: "thread-1",
      timestamp: "2026-08-14T00:00:00.001Z",
      event: { type: "message.thinking", text: "再想想" },
    });
    expect(agent.messages[0].thinking).toBe("先想想再想想");
    expect(agent.publicTextDeltaSeen).toBe(true);

    agent.handleEvent({
      schemaVersion: 1,
      agentId: "char-a",
      sessionId: "thread-1",
      timestamp: "2026-08-14T00:00:00.002Z",
      event: { type: "message.completed", text: "最终回答" },
    });
    expect(agent.messages).toEqual([
      { role: "agent", text: "最终回答", thinking: "先想想再想想", kind: "completed" },
    ]);
  });

  it("creates a live message when thinking arrives before any text delta", () => {
    const agent = useAgentStore();
    agent.handleEvent({
      schemaVersion: 1,
      agentId: "char-a",
      sessionId: "thread-1",
      timestamp: "2026-08-14T00:00:00.000Z",
      event: { type: "message.thinking", text: "思考中" },
    });
    agent.handleEvent({
      schemaVersion: 1,
      agentId: "char-a",
      sessionId: "thread-1",
      timestamp: "2026-08-14T00:00:00.001Z",
      event: { type: "message.delta", text: "回答" },
    });
    expect(agent.messages[0]).toMatchObject({
      role: "agent",
      kind: "delta",
      text: "回答",
      thinking: "思考中",
    });
  });

  it("round-trips thinking with completed history", () => {
    const agent = useAgentStore();
    agent.characterId = "char-a";
    agent.provider = "claude";
    agent.messages = [
      { role: "agent", text: "最终回答", thinking: "过程思考", kind: "completed" },
    ];
    agent.persistVisibleHistory();
    agent.messages = [];
    agent.restoreVisibleHistory();
    expect(agent.messages).toEqual([
      { role: "agent", text: "最终回答", thinking: "过程思考", kind: "completed" },
    ]);
  });

  it("uses only the authoritative bubble event after a completed direct reply", async () => {
    const agent = useAgentStore();
    agent.characterId = "char-a";
    await agent.init();

    agent.handleEvent({
      schemaVersion: 1,
      agentId: "char-a",
      sessionId: "thread-1",
      timestamp: "2026-08-13T00:00:00.000Z",
      event: { type: "message.completed", text: "同一条回复" },
    });
    expect(agent.bubble).toBeNull();

    handlers.get("bubble:requested")?.[0]?.({
      payload: { text: "同一条回复", priority: "normal", agentId: "char-a" },
    });
    expect(agent.bubble?.text).toBe("同一条回复");
  });

  it("gives identical consecutive bubble replies distinct playback identities", async () => {
    const agent = useAgentStore();
    agent.characterId = "char-a";
    await agent.init();
    const bubbleHandler = handlers.get("bubble:requested")?.[0];

    bubbleHandler?.({ payload: { text: "重复回复", priority: "normal", agentId: "char-a" } });
    const firstId = agent.bubble?.id;
    bubbleHandler?.({ payload: { text: "重复回复", priority: "normal", agentId: "char-a" } });

    expect(agent.bubble?.id).not.toBe(firstId);
  });

  it("ignores a repeated delivery id from immediate and claimed bubble events", async () => {
    const agent = useAgentStore();
    await agent.init();
    const bubbleHandler = handlers.get("bubble:requested")?.[0];
    expect(bubbleHandler).toBeDefined();
    bubbleHandler?.({ payload: { deliveryId: "delivery-1", text: "同一条", priority: "normal", agentId: "char-a" } });
    const first = agent.bubble?.id;
    bubbleHandler?.({ payload: { deliveryId: "delivery-1", text: "同一条", priority: "normal", agentId: "char-a" } });
    expect(agent.bubble?.id).toBe(first);
  });

  it("shares concurrent initialization work instead of registering duplicate listeners", async () => {
    const agent = useAgentStore();
    await Promise.all([agent.init(), agent.init()]);
    expect(listen).toHaveBeenCalledTimes(7);
  });

  it("keeps same-day visible history isolated by character and provider", async () => {
    installEventHarness();
    storage.clear();
    invoke.mockReset();
    invoke.mockImplementation(async (command: string, args?: { characterId?: string }) => {
      if (command === "agent_status") {
        return {
          characterId: args?.characterId ?? "char-a",
          provider: args?.characterId === "char-b" ? "claude" : "codex",
          ready: true,
          exePath: null,
          workspaceDir: "D:\\Agent",
        };
      }
      if (command === "agent_list_skills") return [];
      return undefined;
    });
    const agent = useAgentStore();
    agent.characters = [
      { id: "char-a", name: "A", tool: "codex" },
      { id: "char-b", name: "B", tool: "claude" },
    ];

    await agent.selectCharacter("char-a", false);
    agent.addUserMessage("Codex history");
    await agent.selectCharacter("char-b", false);
    expect(agent.messages).toEqual([]);
    agent.addUserMessage("Claude history");
    await agent.selectCharacter("char-a", false);
    expect(agent.messages).toEqual([{ role: "user", text: "Codex history", kind: "completed" }]);
    await agent.selectCharacter("char-b", false);
    expect(agent.messages).toEqual([{ role: "user", text: "Claude history", kind: "completed" }]);
  });

  it("does not add lifecycle rows and sends the composer's visible text unchanged", async () => {
    const agent = useAgentStore();
    agent.characterId = "char-a";
    agent.currentThreadId = "thread-a";
    invoke.mockReset();
    invoke.mockImplementation(async (command: string, args?: unknown) => {
      if (command === "agent_send") {
        expect(args).toEqual({
          characterId: "char-a",
          threadId: "thread-a",
          text: "$focus-cli  $readme  check status",
        });
      }
      return undefined;
    });

    agent.handleEvent({
      schemaVersion: 1,
      agentId: "char-a",
      sessionId: "thread-a",
      timestamp: "2026-08-10T00:00:00.000Z",
      event: { type: "session.started" },
    });
    agent.handleEvent({
      schemaVersion: 1,
      agentId: "char-a",
      sessionId: "thread-a",
      timestamp: "2026-08-10T00:00:00.000Z",
      event: { type: "session.completed", outcome: "success" },
    });
    expect(agent.messages).toEqual([]);

    await agent.send("$focus-cli  $readme  check status");
    expect(agent.messages).toEqual([{
      role: "user",
      text: "$focus-cli  $readme  check status",
      kind: "completed",
    }]);
  });
});

describe("Focus-owned pet states", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    setActivePinia(createPinia());
    installEventHarness();
    storage.clear();
  });

  afterEach(() => vi.useRealTimers());

  it("keeps happy for five seconds before waiting", () => {
    const agent = useAgentStore();
    agent.handleEvent({
      schemaVersion: 1, agentId: "char-a", sessionId: "turn", timestamp: "2026-08-13T00:00:00Z",
      event: { type: "status.changed", state: "success" },
    });
    expect(agent.petState).toBe("happy");
    vi.advanceTimersByTime(4999);
    expect(agent.petState).toBe("happy");
    vi.advanceTimersByTime(1);
    expect(agent.petState).toBe("waiting");
  });

  it("does not let the Provider success-to-idle tail cancel the happy duration", () => {
    const agent = useAgentStore();
    agent.applyProviderPetState("success");
    agent.applyProviderPetState("idle");
    expect(agent.petState).toBe("happy");
    vi.advanceTimersByTime(5000);
    expect(agent.petState).toBe("waiting");
  });

  it("keeps troubled until another Agent task begins", () => {
    const agent = useAgentStore();
    agent.handleEvent({
      schemaVersion: 1, agentId: "char-a", sessionId: "turn", timestamp: "2026-08-13T00:00:00Z",
      event: { type: "status.changed", state: "error" },
    });
    expect(agent.petState).toBe("troubled");
    agent.handleEvent({
      schemaVersion: 1, agentId: "char-a", sessionId: "turn", timestamp: "2026-08-13T00:00:01Z",
      event: { type: "status.changed", state: "thinking" },
    });
    expect(agent.petState).toBe("working");
  });
});

describe("interrupt lifecycle", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
  });

  it("stays streaming after interrupt RPC acknowledgement until the terminal event arrives", async () => {
    const agent = useAgentStore();
    agent.characterId = "char-a";
    agent.currentThreadId = "thread-1";
    agent.phase = "streaming";
    let acknowledge: (() => void) | undefined;
    invoke.mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          acknowledge = resolve;
        }),
    );

    const pendingInterrupt = agent.interrupt();
    expect(agent.phase).toBe("streaming");
    acknowledge?.();
    await pendingInterrupt;
    expect(agent.phase).toBe("streaming");

    agent.handleEvent({
      schemaVersion: 1,
      agentId: "char-a",
      sessionId: "thread-1",
      timestamp: "2026-08-09T00:00:00.000Z",
      event: { type: "session.completed", outcome: "cancelled" },
    });
    expect(agent.phase).toBe("idle");
  });
});

describe("per-character provider selection", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    installEventHarness();
    storage.clear();
    storage.set("focus-agent", "char-a");
    invoke.mockReset();
  });

  it("updates an inactive provider without overwriting the current chat status", async () => {
    const agent = useAgentStore();
    agent.characterId = "char-a";
    agent.provider = "codex";
    agent.workspaceDir = "D:\\Agents\\char-a";
    agent.characters = [
      { id: "char-a", name: "Codex pet", tool: "codex" },
      { id: "char-b", name: "Claude pet", tool: "claude" },
    ];
    invoke.mockImplementation(async (command: string, args?: unknown) => {
      if (command === "agent_set_provider") {
        expect(args).toEqual({ characterId: "char-b", provider: "claude" });
        return { characterId: "char-b", provider: "claude", ready: true, exePath: "C:\\Tools\\claude.exe", workspaceDir: "D:\\Agents\\char-b" };
      }
      if (command === "characters_list") {
        return [
          { id: "char-a", name: "Codex pet", tool: "codex" },
          { id: "char-b", name: "Claude pet", tool: "claude" },
        ];
      }
      if (command === "agent_status") {
        expect(args).toEqual({ characterId: "char-b" });
        return { characterId: "char-b", provider: "claude", ready: true, exePath: "C:\\Tools\\claude.exe", workspaceDir: "D:\\Agents\\char-b" };
      }
      return undefined;
    });

    await agent.setProvider("char-b", "claude");

    expect(agent.characters).toEqual([
      { id: "char-a", name: "Codex pet", tool: "codex" },
      { id: "char-b", name: "Claude pet", tool: "claude" },
    ]);
    expect(agent.provider).toBe("codex");
    expect(agent.workspaceDir).toBe("D:\\Agents\\char-a");
  });

  it("clears the active conversation when the selected character changes provider", async () => {
    const agent = useAgentStore();
    agent.characterId = "char-a";
    agent.currentThreadId = "codex-thread";
    agent.sessionId = "codex-thread";
    agent.messages = [{ role: "agent", text: "old", kind: "completed" }];
    agent.phase = "completed";
    agent.characters = [{ id: "char-a", name: "Pet", tool: "codex" }];
    invoke.mockImplementation(async (command: string) => {
      if (command === "agent_set_provider") return { characterId: "char-a", provider: "claude", ready: true, exePath: null, workspaceDir: "" };
      if (command === "characters_list") return [{ id: "char-a", name: "Pet", tool: "claude" }];
      if (command === "agent_status") return { characterId: "char-a", provider: "claude", ready: true, exePath: null, workspaceDir: "" };
      return undefined;
    });

    await agent.setProvider("char-a", "claude");

    expect(agent.currentThreadId).toBeNull();
    expect(agent.sessionId).toBe("");
    expect(agent.messages).toEqual([]);
    expect(agent.phase).toBe("idle");
  });

  it("refreshes Provider status when chat selects another character", async () => {
    const agent = useAgentStore();
    agent.characterId = "char-a";
    agent.provider = "codex";
    agent.characters = [
      { id: "char-a", name: "Codex pet", tool: "codex" },
      { id: "char-b", name: "Claude pet", tool: "claude" },
    ];
    invoke.mockImplementation(async (command: string, args?: unknown) => {
      if (command === "agent_status") {
        expect(args).toEqual({ characterId: "char-b" });
        return { characterId: "char-b", provider: "claude", ready: true, exePath: null, workspaceDir: "claude-workspace" };
      }
      return undefined;
    });

    await agent.selectCharacter("char-b", false);

    expect(agent.provider).toBe("claude");
    expect(agent.workspaceDir).toBe("claude-workspace");
  });

  it("clears transient pet presentation when selecting another character", async () => {
    vi.useFakeTimers();
    try {
      const agent = useAgentStore();
      agent.characterId = "char-a";
      agent.characters = [
        { id: "char-a", name: "A", tool: "codex" },
        { id: "char-b", name: "B", tool: "claude" },
      ];
      agent.showBubble("A reply");
      agent.applyProviderPetState("success");
      invoke.mockImplementation(async (command: string) => {
        if (command === "agent_status") return { characterId: "char-b", provider: "claude", ready: true, exePath: null, workspaceDir: "B" };
        if (command === "agent_list_skills") return [];
        return undefined;
      });

      await agent.selectCharacter("char-b", false);

      expect(agent.bubble).toBeNull();
      expect(agent.petState).toBe("resting");
      vi.advanceTimersByTime(5000);
      expect(agent.petState).toBe("resting");
    } finally {
      vi.useRealTimers();
    }
  });

  it("ignores status events for a different character", async () => {
    const agent = useAgentStore();
    agent.characterId = "char-a";
    agent.provider = "codex";
    invoke.mockImplementation(async (command: string) => {
      if (command === "characters_list") return [{ id: "char-a", name: "Pet", tool: "codex" }];
      if (command === "agent_status") return { characterId: "char-a", provider: "codex", ready: true, exePath: null, workspaceDir: "current" };
      if (command === "agent_list_skills") return [];
      return undefined;
    });
    await agent.init();
    const statusHandler = handlers.get("agent:status")?.[0];
    expect(statusHandler).toBeDefined();
    statusHandler?.({ payload: { characterId: "char-b", provider: "claude", ready: true, exePath: null, workspaceDir: "other" } });
    expect(agent.provider).toBe("codex");
    expect(agent.workspaceDir).not.toBe("other");
  });
});
