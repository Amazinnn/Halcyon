import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import type { AgentEventEnvelope, AgentState, PetReaction } from "@focus/event-schema";
import { isAgentOwnedPetState, nextFocusPetState, type FocusPetState, type TimerPetBaseState } from "../lib/pet-state";

export interface ChatMessage {
  role: "agent" | "user";
  text: string;
  kind: "delta" | "completed" | "system";
  source?: string;
  /** Provider-visible thinking stream (Claude, streaming switch on; ADR-0036). */
  thinking?: string;
}

const CHAT_HISTORY_PREFIX = "focus.chat.history.v1";

export function chatHistoryKey(characterId: string, provider: "codex" | "claude", date = new Date()): string {
  const localDate = [date.getFullYear(), String(date.getMonth() + 1).padStart(2, "0"), String(date.getDate()).padStart(2, "0")].join("-");
  return `${CHAT_HISTORY_PREFIX}:${characterId}:${provider}:${localDate}`;
}

function visibleMessages(messages: ChatMessage[]): ChatMessage[] {
  return messages.filter((message) => message.kind === "completed" && (message.role === "user" || message.role === "agent"));
}

function readVisibleHistory(key: string): ChatMessage[] {
  try {
    const saved = JSON.parse(localStorage.getItem(key) ?? "[]") as unknown;
    if (!Array.isArray(saved)) return [];
    return saved.filter(
      (message): message is ChatMessage =>
        typeof message === "object" &&
        message !== null &&
        ((message as ChatMessage).role === "user" || (message as ChatMessage).role === "agent") &&
        (message as ChatMessage).kind === "completed" &&
        typeof (message as ChatMessage).text === "string" &&
        ((message as ChatMessage).source === undefined || typeof (message as ChatMessage).source === "string"),
    );
  } catch {
    return [];
  }
}

export interface WorkflowAgentResult {
  workflowId: string;
  workflowName: string;
  agentId: string;
  text: string;
}

export interface AgentThread {
  id: string;
  preview: string;
  cwd: string;
  status: string;
  updatedAt: number;
  /** True when created by a workflow agent node (ADR-0012). */
  automation?: boolean;
}

export interface AgentStatus {
  characterId: string;
  provider: "codex" | "claude";
  ready: boolean;
  exePath: string | null;
  workspaceDir: string;
}

export type AgentPhase = "idle" | "connecting" | "streaming" | "completed" | "error";

export const useAgentStore = defineStore("agent", {
  state: () => ({
    // M5 (ADR-0022): one Agent per character; the current one is selected
    // from the dropdown. Messages/session state belong to the current Agent.
    characterId: "" as string,
    characters: [] as { id: string; name: string; tool: "codex" | "claude"; petPackId?: string | null }[],
    agentId: "focus-codex",
    sessionId: "",
    state: "idle" as AgentState,
    petState: "resting" as FocusPetState,
    timerPetBase: "resting" as TimerPetBaseState,
    messages: [] as ChatMessage[],
    // Workflow outcomes are not conversation history. Keep only results that
    // arrived while their target Agent was not selected, then consume them on
    // that Agent's next selection.
    pendingWorkflowResults: {} as Record<string, WorkflowAgentResult[]>,
    tools: [] as { tool: string; summary: string; status: "started" | "completed" }[],
    bubble: null as { id: number; text: string; priority: string; deliveryId?: string } | null,
    _bubbleSequence: 0,
    _seenBubbleDeliveryIds: [] as string[],
    reaction: null as PetReaction | null,
    lastEvent: null as AgentEventEnvelope | null,
    provider: "codex" as "codex" | "claude",
    ready: false,
    workspaceDir: "",
    threads: [] as AgentThread[],
    currentThreadId: null as string | null,
    phase: "idle" as AgentPhase,
    publicTextDeltaSeen: false,
    historyKey: "",
    skills: [] as string[],
    errorMessage: "",
    characterName: "对话",
    initialized: false,
    _initPromise: null as Promise<void> | null,
    _happyTimer: null as ReturnType<typeof setTimeout> | null,
  }),
  actions: {
    showBubble(text: string, priority = "high") {
      this.bubble = { id: ++this._bubbleSequence, text, priority };
    },
    persistVisibleHistory() {
      if (!this.characterId) return;
      const key = chatHistoryKey(this.characterId, this.provider);
      if (!this.historyKey) this.historyKey = key;
      // Callers synchronize before appending. Never copy an in-memory
      // yesterday into a newly generated date key as a fallback.
      if (this.historyKey !== key) return;
      localStorage.setItem(
        key,
        JSON.stringify(visibleMessages(this.messages)),
      );
    },
    syncVisibleHistoryDay() {
      if (!this.characterId) return;
      const key = chatHistoryKey(this.characterId, this.provider);
      if (this.historyKey === key) return;
      this.messages = readVisibleHistory(key);
      this.historyKey = key;
    },
    restoreVisibleHistory() {
      if (!this.characterId) {
        this.messages = [];
        this.historyKey = "";
        return;
      }
      this.historyKey = chatHistoryKey(this.characterId, this.provider);
      this.messages = readVisibleHistory(this.historyKey);
    },
    async refreshCharacters() {
      // v1.12.2: retry a transient empty list (startup race — workflow's
      // ensure_characters may not have run yet) instead of leaving
      // characterId empty and failing later with "角色不存在".
      for (let attempt = 0; attempt < 3; attempt++) {
        try {
          const chars = await invoke<{ id: string; name: string; tool: "codex" | "claude"; petPackId?: string | null }[]>("characters_list");
          this.characters = chars;
          if (chars.length) {
            // The persisted desktop identity is authoritative. Browser storage
            // is only a fallback for settings written before v1.13.
            const bootstrap = await invoke<{ currentAgentId?: string | null }>("get_bootstrap");
            const saved = localStorage.getItem("focus-agent");
            const persisted = bootstrap?.currentAgentId;
            const target = persisted && chars.some((c) => c.id === persisted)
              ? persisted
              : saved && chars.some((c) => c.id === saved) ? saved : chars[0].id;
            if (target !== this.characterId) {
              await this.selectCharacter(target, false);
            } else {
              await this.refreshStatus(target);
              await this.refreshSkills(target);
              this.restoreVisibleHistory();
            }
            return;
          }
        } catch (e) {
          console.error("[agent] characters_list failed", e);
        }
        if (attempt < 2) await new Promise((r) => setTimeout(r, 500));
      }
    },
    /** M5 (ADR-0022): switch Agent = replace the dialog context immediately. */
    async selectCharacter(id: string, broadcast = true) {
      if (!id || id === this.characterId) return;
      if (this._happyTimer) {
        clearTimeout(this._happyTimer);
        this._happyTimer = null;
      }
      this.characterId = id;
      // M5: remember the choice across restarts.
      localStorage.setItem("focus-agent", id);
      this.sessionId = "";
      this.currentThreadId = null;
      this.messages = [];
      this.tools = [];
      this.phase = "idle";
      this.publicTextDeltaSeen = false;
      this.errorMessage = "";
      this.bubble = null;
      this.reaction = null;
      this.lastEvent = null;
      this.state = "idle";
      this.petState = this.timerPetBase;
      const pendingForSelection = this.pendingWorkflowResults[id] ?? [];
      const c = this.characters.find((x) => x.id === id);
      this.characterName = c?.name ?? "对话";
      // Agent selection is also the desktop identity selection.
      await invoke("agent_set_current", { characterId: id });
      void emit("pet:changed", {});
      await this.refreshStatus(id);
      await this.refreshSkills(id);
      this.restoreVisibleHistory();
      // Today's session hash is resumed server-side (lazy runtime build).
      this.pushSystem(`已切换到 ${this.characterName}`);
      const pending = pendingForSelection;
      delete this.pendingWorkflowResults[id];
      for (const result of pending) this.appendWorkflowResult(result);
      if (pending.length) this.persistVisibleHistory();
      if (broadcast) await emit("agent:selected", { characterId: id });
    },
    async init(opts?: { thin?: boolean }) {
      if (this._initPromise) return this._initPromise;
      this._initPromise = this.initInternal(opts);
      try {
        await this._initPromise;
      } finally {
        this._initPromise = null;
      }
    },
    async initInternal(opts?: { thin?: boolean }) {
      if (this.initialized) return;
      this.initialized = true;
      if (opts?.thin) {
        // Thin mode (extensibility plan C3): light windows (topbar,
        // pet-bubble, grid-overlay) only track the pet state dot; no
        // characters, sessions, or workflow state is initialized.
        await listen<{ state: AgentState; animation: string }>("pet:state_changed", (e) => {
          this.state = e.payload.state;
        });
        return;
      }
      await listen<AgentEventEnvelope>("agent:event", (e) => {
        // M5 (ADR-0022): only handle events whose agentId matches the current
        // character — other Agents' events never pollute this dialog.
        if (e.payload.agentId !== this.characterId) return;
        this.lastEvent = e.payload;
        this.handleEvent(e.payload);
      });
      await listen<{ text: string; priority: string; agentId?: string; deliveryId?: string }>("bubble:requested", (e) => {
        if (e.payload.agentId && e.payload.agentId !== this.characterId) return;
        if (e.payload.deliveryId && this._seenBubbleDeliveryIds.includes(e.payload.deliveryId)) return;
        if (e.payload.deliveryId) this._seenBubbleDeliveryIds.push(e.payload.deliveryId);
        this.bubble = {
          id: ++this._bubbleSequence,
          text: e.payload.text,
          priority: e.payload.priority,
          deliveryId: e.payload.deliveryId,
        };
      });
      await listen<{ state: AgentState; animation: string }>("pet:state_changed", (e) => {
        this.state = e.payload.state;
      });
      await listen<{ state: "idle" | "focus" | "rest" }>("focus:state_changed", (e) => {
        this.timerPetBase = e.payload.state === "focus" ? "focusing" : "resting";
        if (!isAgentOwnedPetState(this.petState)) this.petState = this.timerPetBase;
      });
      await listen<AgentStatus>("agent:status", (e) => {
        if (e.payload.characterId !== this.characterId) return;
        this.provider = e.payload.provider;
        this.ready = e.payload.ready;
        this.workspaceDir = e.payload.workspaceDir;
      });
      await listen<WorkflowAgentResult>("workflow:agent_result", (e) => {
        if (e.payload.agentId === this.characterId) {
          this.appendWorkflowResult(e.payload);
          this.persistVisibleHistory();
          return;
        }
        (this.pendingWorkflowResults[e.payload.agentId] ??= []).push(e.payload);
      });
      await listen<{ characterId: string }>("agent:selected", (e) => {
        void this.selectCharacter(e.payload.characterId, false);
      });
      await this.refreshCharacters();
      if (!this.characterId) await this.refreshStatus();
    },
    async refreshStatus(characterId?: string) {
      try {
        const targetCharacterId = characterId ?? this.characterId;
        const s = await invoke<AgentStatus>("agent_status", targetCharacterId ? { characterId: targetCharacterId } : undefined);
        if (s.characterId !== this.characterId) return;
        this.provider = s.provider;
        this.ready = s.ready;
        this.workspaceDir = s.workspaceDir;
      } catch (e) {
        console.error("[agent] agent_status failed", e);
      }
    },
    async refreshSkills(characterId?: string) {
      try {
        const targetCharacterId = characterId ?? this.characterId;
        const skills = await invoke<string[]>("agent_list_skills", targetCharacterId ? { characterId: targetCharacterId } : undefined);
        if (targetCharacterId === this.characterId) this.skills = skills;
      } catch (e) {
        console.error("[agent] agent_list_skills failed", e);
      }
    },
    async refreshThreads() {
      // M5 (ADR-0022): thread list no longer shown in the UI; kept for
      // compatibility (automation cleanup).
      try {
        this.threads = await invoke<AgentThread[]>("agent_list_threads", {
          characterId: this.characterId,
        });
      } catch (e) {
        console.error("[agent] agent_list_threads failed", e);
      }
    },
    async startThread(initialMessage: string) {
      this.phase = "connecting";
      this.publicTextDeltaSeen = false;
      this.errorMessage = "";
      try {
        const info = await invoke<AgentThread>("agent_start_thread", {
          characterId: this.characterId,
          initialMessage,
        });
        this.currentThreadId = info.id;
        this.sessionId = info.id;
        this.phase = initialMessage.trim() ? "streaming" : "completed";
        await this.refreshStatus();
      } catch (e) {
        this.phase = "error";
        this.errorMessage = `启动失败：${e}`;
        this.pushSystem(`启动失败：${e}`);
        await this.refreshStatus();
      }
    },
    async resumeThread(threadId: string) {
      this.phase = "connecting";
      try {
        const info = await invoke<AgentThread>("agent_resume_thread", {
          characterId: this.characterId,
          threadId,
        });
        this.currentThreadId = info.id;
        this.sessionId = info.id;
        this.phase = "completed";
      } catch (e) {
        this.phase = "error";
        this.errorMessage = `恢复会话失败：${e}`;
        this.pushSystem(`恢复会话失败：${e}`);
      }
    },
    async send(text: string) {
      const trimmed = text.trim();
      if (!trimmed) return;
      const message = trimmed;
      this.addUserMessage(message);
      if (!this.currentThreadId) {
        await this.startThread(message);
        return;
      }
      this.phase = "connecting";
      this.publicTextDeltaSeen = false;
      this.errorMessage = "";
      try {
        await invoke("agent_send", {
          characterId: this.characterId,
          threadId: this.currentThreadId,
          text: message,
        });
        this.phase = "streaming";
      } catch (e) {
        this.phase = "error";
        this.errorMessage = `发送失败：${e}`;
        this.pushSystem(`发送失败：${e}`);
      }
    },
    async cleanupAutomationThreads() {
      try {
        await invoke("workflow_cleanup_threads");
      } catch (e) {
        console.error("[agent] workflow_cleanup_threads failed", e);
      }
    },
    async interrupt() {
      if (!this.currentThreadId) return;
      try {
        await invoke("agent_interrupt", {
          characterId: this.characterId,
          threadId: this.currentThreadId,
        });
      } catch (e) {
        console.error("[agent] agent_interrupt failed", e);
      }
    },
    async setWorkspaceDir(dir: string) {
      await invoke("set_agent_workspace_dir", { dir });
      this.workspaceDir = dir;
    },
    async setProvider(characterId: string, provider: "codex" | "claude") {
      const previousProvider = this.characters.find((character) => character.id === characterId)?.tool;
      const status = await invoke<AgentStatus>("agent_set_provider", { characterId, provider });
      if (characterId === this.characterId && previousProvider !== status.provider) this.newThread();
      await this.refreshCharacters();
      await this.refreshStatus(characterId);
      if (characterId === this.characterId) {
        await this.refreshSkills(characterId);
        this.restoreVisibleHistory();
      }
    },
    async createCharacter(name: string, provider: "codex" | "claude") {
      const row = await invoke<{ id: string }>("agent_create", { name, provider });
      await this.refreshCharacters();
      await this.selectCharacter(row.id);
      return row.id;
    },
    async setCurrentCharacter(id: string) {
      await invoke("agent_set_current", { characterId: id });
      await this.selectCharacter(id);
      await emit("pet:changed", {});
    },
    newThread() {
      this.currentThreadId = null;
      this.sessionId = "";
      this.phase = "idle";
      this.messages = [];
      this.tools = [];
      this.errorMessage = "";
    },
    pushSystem(_text: string) {
    },
    appendWorkflowResult(result: WorkflowAgentResult) {
      this.syncVisibleHistoryDay();
      this.messages.push({
        role: "agent",
        text: result.text,
        kind: "completed",
        source: `日程 · ${result.workflowName}`,
      });
    },
    handleEvent(env: AgentEventEnvelope) {
      const ev = env.event;
      switch (ev.type) {
        case "session.started":
          this.sessionId = env.sessionId;
          this.pushSystem("会话已开始");
          break;
        case "message.delta":
          this.appendDelta(ev.text);
          break;
        case "message.thinking":
          this.appendThinking(ev.text);
          break;
        case "message.completed":
          this.finalizeDelta(ev.text);
          break;
        case "tool.started":
          this.tools.push({ tool: ev.tool, summary: ev.inputSummary, status: "started" });
          break;
        case "tool.completed":
          this.tools.push({ tool: ev.tool, summary: ev.resultSummary, status: "completed" });
          break;
        case "status.changed":
          this.state = ev.state;
          this.applyProviderPetState(ev.state);
          this.reaction = { agentId: env.agentId, state: ev.state, animation: this.petState };
          if (["thinking", "reading", "searching", "editing", "running", "testing", "waiting_permission", "waiting_user"].includes(ev.state)) {
            this.phase = "streaming";
          } else if (ev.state === "success") {
            this.phase = "completed";
          } else if (ev.state === "error" || ev.state === "warning") {
            this.phase = "error";
          } else if (ev.state === "idle" || ev.state === "cancelled") {
            this.phase = "idle";
          }
          break;
        case "permission.requested":
          this.phase = "error";
          this.errorMessage = "Agent 正在等待其本地权限确认。";
          this.pushSystem(`请求权限：${ev.requestId}（风险 ${ev.risk}）`);
          break;
        case "session.completed":
          if (ev.outcome === "error") {
            this.phase = "error";
            this.pushSystem(`会话结束：${ev.outcome}`);
          } else if (ev.outcome === "cancelled") {
            this.phase = "idle";
            this.pushSystem("会话已中断");
          } else {
            this.phase = "completed";
            this.pushSystem(`会话结束：${ev.outcome}`);
          }
          break;
        case "session.error":
          this.phase = "error";
          this.errorMessage = `Agent 错误：${ev.message}`;
          this.pushSystem(`错误：${ev.message}`);
          break;
      }
    },
    applyProviderPetState(state: AgentState) {
      if (state === "idle" && this._happyTimer && this.petState === "happy") return;
      if (this._happyTimer) {
        clearTimeout(this._happyTimer);
        this._happyTimer = null;
      }
      const next = nextFocusPetState(this.timerPetBase, state, this.petState);
      this.petState = next;
      if (state === "success") {
        this._happyTimer = setTimeout(() => {
          this.petState = "waiting";
          this._happyTimer = null;
        }, 5000);
      }
    },
    appendDelta(text: string) {
      this.publicTextDeltaSeen = true;
      this.syncVisibleHistoryDay();
      const last = this.messages[this.messages.length - 1];
      if (last && last.role === "agent" && last.kind === "delta") {
        last.text += text;
      } else {
        this.messages.push({ role: "agent", text, kind: "delta" });
      }
    },
    /** ADR-0036: Claude thinking increments accumulate on the live agent
     * message; they stay with the completed message and never enter the pet
     * bubble or workflow results. */
    appendThinking(text: string) {
      this.publicTextDeltaSeen = true;
      this.syncVisibleHistoryDay();
      const last = this.messages[this.messages.length - 1];
      if (last && last.role === "agent" && last.kind === "delta") {
        last.thinking = (last.thinking ?? "") + text;
      } else {
        this.messages.push({ role: "agent", text: "", thinking: text, kind: "delta" });
      }
    },
    finalizeDelta(text: string) {
      this.syncVisibleHistoryDay();
      const last = this.messages[this.messages.length - 1];
      if (last && last.role === "agent" && last.kind === "delta") {
        last.text = text;
        last.kind = "completed";
      } else {
        this.messages.push({ role: "agent", text, kind: "completed" });
      }
      this.persistVisibleHistory();
    },
    addUserMessage(text: string) {
      this.syncVisibleHistoryDay();
      this.messages.push({ role: "user", text, kind: "completed" });
      this.persistVisibleHistory();
    },
    clearBubble() {
      this.bubble = null;
    },
  },
});
