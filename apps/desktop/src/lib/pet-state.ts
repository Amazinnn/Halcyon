import type { AgentState } from "@focus/event-schema";

export type FocusPetState = "resting" | "focusing" | "working" | "waiting" | "happy" | "troubled";
export type TimerPetBaseState = "resting" | "focusing";

export function nextFocusPetState(
  timerBase: TimerPetBaseState,
  agentState: AgentState,
  _previous?: FocusPetState,
): FocusPetState {
  if (["thinking", "reading", "searching", "editing", "running", "testing"].includes(agentState)) return "working";
  if (["waiting_permission", "waiting_user"].includes(agentState)) return "waiting";
  if (agentState === "success") return "happy";
  if (agentState === "error" || agentState === "warning") return "troubled";
  return timerBase;
}

export function isAgentOwnedPetState(state: FocusPetState): boolean {
  return state === "working" || state === "waiting" || state === "happy" || state === "troubled";
}
