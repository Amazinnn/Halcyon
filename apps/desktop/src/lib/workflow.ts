// M4 workflow v2 (ADR-0017) + v1.10.5 convergence (ADR-0018): 7 node kinds.
// No bubble / no IF. UI is variable-free: data flows through edges, the
// engine keeps {{nodeId.field}} internally only. Params are card-based;
// the only free-text field in the UI is the Agent prompt.

export interface WorkflowNode {
  id: string;
  kind: string;
  params: Record<string, unknown>;
  x: number;
  y: number;
}

export interface WorkflowEdge {
  id: string;
  source: string;
  sourceHandle: string;
  target: string;
}

export interface WorkflowDef {
  id: string;
  characterId: string;
  name: string;
  trigger: string;
  scheduleType?: string | null;
  intervalMinutes?: number | null;
  dailyTime?: string | null;
  guard: string;
  nodes: WorkflowNode[];
  edges: WorkflowEdge[];
  enabled: boolean;
  nextRunAt?: number | null;
}

export interface CharacterRow {
  id: string;
  name: string;
  persona: string;
  petPackId: string | null;
}

export interface WorkflowRunRow {
  id: string;
  workflowId: string;
  triggeredBy: string;
  startedAt: number;
  finishedAt: number | null;
  status: string;
  error: string | null;
  nodeLog: string;
}

/** v1.10.4 (#51): settings-page run record with workflow display name. */
export interface RecentRunRow {
  id: string;
  workflowId: string;
  workflowName: string;
  triggeredBy: string;
  startedAt: number;
  finishedAt: number | null;
  status: string;
  error: string | null;
  nodeLog: string;
}

export const NODE_KINDS = [
  "agent",
  "show_window",
  "wait",
  "branch",
  "focus",
  "idle",
  "ring",
] as const;

export const NODE_LABELS: Record<string, string> = {
  agent: "发送给 Agent",
  show_window: "显示窗口",
  wait: "等待",
  branch: "分支",
  focus: "专注",
  idle: "空闲",
  ring: "响铃",
};

export const NODE_DESC: Record<string, string> = {
  agent: "调用角色 Agent；回复显示在对话框与宠物",
  show_window: "打开一个内部窗口",
  wait: "等待 N 秒",
  branch: "按选项多路路由",
  focus: "进入专注 N 秒",
  idle: "进入空闲 N 秒",
  ring: "响铃 N 秒",
};

export function defaultParams(kind: string): Record<string, unknown> {
  switch (kind) {
    case "agent":
      // M5 (ADR-0022): display switches — showInitial (first short sentence,
      // on), showThinking (stream, off), showResult (final, on).
      return {
        prompt: "",
        wait: true,
        timeout: 600,
        fillOptions: [] as string[],
        showInitial: true,
        showThinking: false,
        showResult: true,
      };
    case "show_window":
      return { target: "chat" };
    case "wait":
      return { seconds: 30 };
    case "branch":
      return { condition: "slot", options: ["专注", "分心"] as string[], focusState: "focus" };
    case "focus":
      return { seconds: 1500 };
    case "idle":
      return { seconds: 300 };
    case "ring":
      return { seconds: 3 };
    default:
      return {};
  }
}

export function emptyWorkflow(characterId: string, name = "新工作流"): WorkflowDef {
  return {
    id: "",
    characterId,
    name,
    trigger: "manual",
    scheduleType: null,
    intervalMinutes: null,
    dailyTime: null,
    guard: "none",
    nodes: [],
    edges: [],
    enabled: true,
    nextRunAt: null,
  };
}

export const TRIGGER_LABELS: Record<string, string> = {
  // v1.11.1: "manual" reads as a save toggle to the user — label it 保存.
  manual: "保存",
  scheduled: "定时",
  focus_end: "专注结束",
  supervision_alert: "监督告警",
};

export const SCHEDULE_LABELS: Record<string, string> = {
  interval: "间隔",
  daily: "每日",
};

export const GUARD_LABELS: Record<string, string> = {
  none: "无",
  focusing: "仅专注中",
  resting: "仅休息中",
  idle: "仅空闲中",
};

/** Compact trigger badge text, e.g. "定时 · 每30分 · 仅专注中". */
export function triggerBadge(wf: WorkflowDef): string {
  const t = TRIGGER_LABELS[wf.trigger] ?? wf.trigger;
  let detail = "";
  if (wf.trigger === "scheduled") {
    if (wf.scheduleType === "interval") detail = `每${wf.intervalMinutes ?? 30}分`;
    else if (wf.scheduleType === "daily") detail = `每日 ${wf.dailyTime ?? "09:00"}`;
    else detail = "定时";
  }
  const g = GUARD_LABELS[wf.guard] ?? wf.guard;
  const parts = [t];
  if (detail) parts.push(detail);
  if (wf.guard && wf.guard !== "none") parts.push(`守卫:${g}`);
  return parts.join(" · ");
}