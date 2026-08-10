import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import type { AgentEventEnvelope, AgentState, PetReaction } from "@focus/event-schema";

export interface ChatMessage {
  role: "agent" | "user";
  text: string;
  kind: "delta" | "completed" | "system";
  source?: string;
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

export function stateToAnimation(state: AgentState): string {
  switch (state) {
    case "thinking":
    case "reading":
    case "searching":
      return "thinking";
    case "editing":
    case "running":
    case "testing":
      return "editing";
    case "waiting_permission":
    case "waiting_user":
      return "waiting";
    case "success":
      return "success";
    case "error":
    case "warning":
      return "error";
    default:
      return "idle";
  }
}

export const useAgentStore = defineStore("agent", {
  state: () => ({
    // M5 (ADR-0022): one Agent per character; the current one is selected
    // from the dropdown. Messages/session state belong to the current Agent.
    characterId: "" as string,
    characters: [] as { id: string; name: string; tool: "codex" | "claude" }[],
    agentId: "focus-codex",
    sessionId: "",
    state: "idle" as AgentState,
    animation: "idle",
    messages: [] as ChatMessage[],
    // Workflow outcomes are not conversation history. Keep only results that
    // arrived while their target Agent was not selected, then consume them on
    // that Agent's next selection.
    pendingWorkflowResults: {} as Record<string, WorkflowAgentResult[]>,
    tools: [] as { tool: string; summary: string; status: "started" | "completed" }[],
    bubble: null as { text: string; priority: string; expiresAt: number } | null,
    reaction: null as PetReaction | null,
    lastEvent: null as AgentEventEnvelope | null,
    provider: "codex" as "codex" | "claude",
    ready: false,
    workspaceDir: "",
    threads: [] as AgentThread[],
    currentThreadId: null as string | null,
    phase: "idle" as AgentPhase,
    skills: [] as string[],
    characterName: "对话",
    initialized: false,
  }),
  actions: {
    showBubble(text: string, priority = "high") {
      this.bubble = { text, priority, expiresAt: Date.now() + 5000 };
    },
    async refreshCharacters() {
      // v1.12.2: retry a transient empty list (startup race — workflow's
      // ensure_characters may not have run yet) instead of leaving
      // characterId empty and failing later with "角色不存在".
      for (let attempt = 0; attempt < 3; attempt++) {
        try {
          const chars = await invoke<{ id: string; name: string; tool: "codex" | "claude" }[]>("characters_list");
          this.characters = chars;
          if (chars.length) {
            // M5 (ADR-0022): restore the last-selected Agent, else pick first.
            const saved = localStorage.getItem("focus-agent");
            const target = saved && chars.some((c) => c.id === saved) ? saved : chars[0].id;
            if (target !== this.characterId) {
              await this.selectCharacter(target, false);
            } else {
              await this.refreshStatus(target);
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
      this.characterId = id;
      // M5: remember the choice across restarts.
      localStorage.setItem("focus-agent", id);
      this.sessionId = "";
      this.currentThreadId = null;
      this.messages = [];
      this.tools = [];
      this.phase = "idle";
      const c = this.characters.find((x) => x.id === id);
      this.characterName = c?.name ?? "对话";
      await this.refreshStatus(id);
      // Today's session hash is resumed server-side (lazy runtime build).
      this.pushSystem(`已切换到 ${this.characterName}`);
      const pending = this.pendingWorkflowResults[id] ?? [];
      delete this.pendingWorkflowResults[id];
      for (const result of pending) this.appendWorkflowResult(result);
      const latest = pending[pending.length - 1];
      if (latest) this.showBubble(latest.text, "normal");
      if (broadcast) await emit("agent:selected", { characterId: id });
    },
    async init() {
      if (this.initialized) return;
      this.initialized = true;
      await listen<AgentEventEnvelope>("agent:event", (e) => {
        // M5 (ADR-0022): only handle events whose agentId matches the current
        // character — other Agents' events never pollute this dialog.
        if (e.payload.agentId !== this.characterId) return;
        this.lastEvent = e.payload;
        this.handleEvent(e.payload);
      });
      await listen<{ text: string; priority: string; agentId?: string }>("bubble:requested", (e) => {
        if (e.payload.agentId && e.payload.agentId !== this.characterId) return;
        this.bubble = {
          text: e.payload.text,
          priority: e.payload.priority,
          expiresAt: Date.now() + 4000,
        };
      });
      await listen<{ state: AgentState; animation: string }>("pet:state_changed", (e) => {
        this.state = e.payload.state;
        this.animation = e.payload.animation;
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
          return;
        }
        (this.pendingWorkflowResults[e.payload.agentId] ??= []).push(e.payload);
      });
      await listen<{ characterId: string }>("agent:selected", (e) => {
        void this.selectCharacter(e.payload.characterId, false);
      });
      await this.refreshStatus();
      await this.refreshSkills();
      await this.refreshCharacters();
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
    async refreshSkills() {
      try {
        this.skills = await invoke<string[]>("agent_list_skills");
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
        this.pushSystem(`恢复会话失败：${e}`);
      }
    },
    async send(text: string) {
      const trimmed = text.trim();
      if (!trimmed) return;
      this.messages.push({ role: "user", text: trimmed, kind: "completed" });
      if (!this.currentThreadId) {
        await this.startThread(trimmed);
        return;
      }
      this.phase = "connecting";
      try {
        await invoke("agent_send", {
          characterId: this.characterId,
          threadId: this.currentThreadId,
          text: trimmed,
        });
        this.phase = "streaming";
      } catch (e) {
        this.phase = "error";
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
    },
    newThread() {
      this.currentThreadId = null;
      this.sessionId = "";
      this.phase = "idle";
      this.messages = [];
      this.tools = [];
    },
    pushSystem(text: string) {
      this.messages.push({ role: "agent", text, kind: "system" });
    },
    appendWorkflowResult(result: WorkflowAgentResult) {
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
          this.animation = stateToAnimation(ev.state);
          this.reaction = { agentId: env.agentId, state: ev.state, animation: this.animation };
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
          this.pushSystem(`错误：${ev.message}`);
          break;
      }
    },
    appendDelta(text: string) {
      const last = this.messages[this.messages.length - 1];
      if (last && last.role === "agent" && last.kind === "delta") {
        last.text += text;
      } else {
        this.messages.push({ role: "agent", text, kind: "delta" });
      }
    },
    finalizeDelta(text: string) {
      const last = this.messages[this.messages.length - 1];
      if (last && last.role === "agent" && last.kind === "delta") {
        last.text = text;
        last.kind = "completed";
      } else {
        this.messages.push({ role: "agent", text, kind: "completed" });
      }
    },
    addUserMessage(text: string) {
      this.messages.push({ role: "user", text, kind: "completed" });
    },
    clearBubble() {
      this.bubble = null;
    },
  },
});
