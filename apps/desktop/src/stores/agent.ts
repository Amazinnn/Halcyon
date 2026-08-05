import { defineStore } from "pinia";
import { listen } from "@tauri-apps/api/event";
import type { AgentEventEnvelope, AgentState, PetReaction } from "@focus/event-schema";

export interface ChatMessage {
  role: "agent" | "user";
  text: string;
  kind: "delta" | "completed" | "system";
}

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
    agentId: "mock-opencode",
    sessionId: "sess-001",
    state: "idle" as AgentState,
    animation: "idle",
    messages: [] as ChatMessage[],
    tools: [] as { tool: string; summary: string; status: "started" | "completed" }[],
    bubble: null as { text: string; priority: string; expiresAt: number } | null,
    reaction: null as PetReaction | null,
    lastEvent: null as AgentEventEnvelope | null,
  }),
  actions: {
    showBubble(text: string, priority = "high") {
      this.bubble = { text, priority, expiresAt: Date.now() + 5000 };
    },
    async init() {
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
    },
    handleEvent(env: AgentEventEnvelope) {
      const ev = env.event;
      switch (ev.type) {
        case "session.started":
          this.sessionId = env.sessionId;
          this.messages.push({ role: "agent", text: "会话已开始", kind: "system" });
          break;
        case "message.delta":
          this.messages.push({ role: "agent", text: ev.text, kind: "delta" });
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
          break;
        case "permission.requested":
          this.messages.push({
            role: "agent",
            text: `请求权限：${ev.requestId}（风险 ${ev.risk}）`,
            kind: "system",
          });
          break;
        case "session.completed":
          this.messages.push({ role: "agent", text: `会话结束：${ev.outcome}`, kind: "system" });
          break;
        case "session.error":
          this.messages.push({ role: "agent", text: `错误：${ev.message}`, kind: "system" });
          break;
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