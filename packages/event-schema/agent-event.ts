/**
 * AgentEvent protocol v1 — TypeScript mirror of `agent-event.schema.json`.
 *
 * Protocol decision (ADR-0001): the envelope carries identity
 * (`agentId`/`sessionId`) and protocol version; the inner `event` is a
 * discriminated union carrying only domain payload. This refines the design
 * document (v0.2, §8.2) where each event variant repeated `sessionId`.
 */

export const AGENT_STATES = [
  "offline",
  "idle",
  "thinking",
  "reading",
  "searching",
  "editing",
  "running",
  "testing",
  "waiting_permission",
  "waiting_user",
  "success",
  "warning",
  "error",
  "cancelled",
] as const;

export type AgentState = (typeof AGENT_STATES)[number];

export const AGENT_EVENT_TYPES = [
  "session.started",
  "message.delta",
  "message.completed",
  "tool.started",
  "tool.completed",
  "file.read",
  "file.changed",
  "permission.requested",
  "status.changed",
  "session.completed",
  "session.error",
] as const;

export type AgentEventType = (typeof AGENT_EVENT_TYPES)[number];

export type RiskLevel = "low" | "medium" | "high" | "critical";

export interface AgentEventEnvelope {
  schemaVersion: 1;
  agentId: string;
  sessionId: string;
  /** ISO-8601 UTC timestamp, e.g. "2026-08-05T08:00:00.000Z". */
  timestamp: string;
  event: AgentEventBody;
}

export type AgentEventBody =
  | { type: "session.started" }
  | { type: "message.delta"; text: string }
  | { type: "message.completed"; text: string }
  | { type: "tool.started"; tool: string; inputSummary: string }
  | { type: "tool.completed"; tool: string; resultSummary: string }
  | { type: "file.read"; path: string }
  | { type: "file.changed"; path: string; diffId: string }
  | { type: "permission.requested"; requestId: string; risk: RiskLevel }
  | { type: "status.changed"; state: AgentState }
  | { type: "session.completed"; outcome: string }
  | { type: "session.error"; message: string };

/** Pet bubble priority, per design document §5.2. */
export type BubblePriority = "low" | "normal" | "high" | "critical";

/** UI mapping from an AgentState to a pet reaction, per design document §5.2. */
export interface PetReaction {
  agentId: string;
  state: AgentState;
  animation: string;
  bubble?: {
    text: string;
    priority: BubblePriority;
    durationMs: number;
  };
  sound?: string;
  badge?: string;
  progress?: number;
}