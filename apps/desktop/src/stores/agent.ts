import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AgentEventEnvelope, AgentState, PetReaction } from "@focus/event-schema";

export interface ChatMessage {
  role: "agent" | "user";
  text: string;
  kind: "delta" | "completed" | "system";
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
  provider: "codex" | "mock";
  fallback: boolean;
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
    agentId: "focus-codex",
    sessionId: "focus-codex",
    state: "idle" as AgentState,
    animation: "idle",
    messages: [] as ChatMessage[],
    tools: [] as { tool: string; summary: string; status: "started" | "completed" }[],
    bubble: null as { text: string; priority: string; expiresAt: number } | null,
    reaction: null as PetReaction | null,
    lastEvent: null as AgentEventEnvelope | null,
    provider: "codex" as "codex" | "mock",
    fallback: false,
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
    async init() {
      if (this.initialized) return;
      this.initialized = true;
      await listen<AgentEventEnvelope>("agent:event", (e) => {
        this.lastEvent = e.payload;
        this.handleEvent(e.payload);
      });
      await listen<{ text: string; priority: string }>("bubble:requested", (e) => {
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
        this.provider = e.payload.provider;
        this.fallback = e.payload.fallback;
        this.ready = e.payload.ready;
        this.workspaceDir = e.payload.workspaceDir;
      });
      await this.refreshStatus();
      await this.refreshSkills();
      try {
        const chars = await invoke<{ id: string; name: string }[]>("characters_list");
        if (chars.length) this.characterName = chars[0].name;
      } catch (e) {
        console.error("[agent] characters_list failed", e);
      }
    },
    async refreshStatus() {
      try {
        const s = await invoke<AgentStatus>("agent_status");
        this.provider = s.provider;
        this.fallback = s.fallback;
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
      try {
        this.threads = await invoke<AgentThread[]>("agent_list_threads");
      } catch (e) {
        console.error("[agent] agent_list_threads failed", e);
      }
    },
    async startThread(initialMessage: string) {
      this.phase = "connecting";
      try {
        const info = await invoke<AgentThread>("agent_start_thread", {
          initialMessage,
        });
        this.currentThreadId = info.id;
        this.sessionId = info.id;
        this.phase = initialMessage.trim() ? "streaming" : "completed";
        await this.refreshThreads();
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
        const info = await invoke<AgentThread>("agent_resume_thread", { threadId });
        this.currentThreadId = info.id;
        this.sessionId = info.id;
        this.phase = "completed";
        await this.refreshThreads();
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
        await invoke("agent_send", { threadId: this.currentThreadId, text: trimmed });
        this.phase = "streaming";
      } catch (e) {
        this.phase = "error";
        this.pushSystem(`发送失败：${e}`);
      }
    },
    async cleanupAutomationThreads() {
      try {
        await invoke("workflow_cleanup_threads");
        await this.refreshThreads();
      } catch (e) {
        console.error("[agent] workflow_cleanup_threads failed", e);
      }
    },
    async interrupt() {
      if (!this.currentThreadId) return;
      try {
        await invoke("agent_interrupt", { threadId: this.currentThreadId });
        this.phase = "idle";
      } catch (e) {
        console.error("[agent] agent_interrupt failed", e);
      }
    },
    async setProvider(provider: "codex" | "mock") {
      await invoke("set_agent_provider", { provider });
      await this.refreshStatus();
    },
    async setWorkspaceDir(dir: string) {
      await invoke("set_agent_workspace_dir", { dir });
      this.workspaceDir = dir;
    },
    newThread() {
      this.currentThreadId = null;
      this.sessionId = "focus-codex";
      this.phase = "idle";
      this.messages = [];
      this.tools = [];
    },
    pushSystem(text: string) {
      this.messages.push({ role: "agent", text, kind: "system" });
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
          this.messages.push({ role: "agent", text: ev.text, kind: "completed" });
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
    addUserMessage(text: string) {
      this.messages.push({ role: "user", text, kind: "completed" });
    },
    clearBubble() {
      this.bubble = null;
    },
  },
});