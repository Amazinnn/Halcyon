// M4 workflow v2 (ADR-0017): 8 node kinds + legacy `if`. Wire shapes match the
// Rust WorkflowDef. v2 has no bundled templates: blank canvas + guided empty
// state; the frontend auto-saves with an 800ms debounce.

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
  "bubble",
  "agent",
  "show_window",
  "wait",
  "branch",
  "focus",
  "idle",
  "ring",
] as const;

/** Legacy binary condition kept for old saved flows (ADR-0017). */
export const LEGACY_KINDS = ["if"] as const;

export const NODE_LABELS: Record<string, string> = {
  bubble: "气泡",
  agent: "发送给 Agent",
  show_window: "显示窗口",
  wait: "等待",
  branch: "分支",
  focus: "专注",
  idle: "空闲",
  ring: "响铃",
  if: "条件 IF（旧）",
};

export const NODE_DESC: Record<string, string> = {
  bubble: "显示一条气泡消息",
  agent: "调用角色 Agent，可等待结果",
  show_window: "打开一个内部窗口",
  wait: "等待 N 秒",
  branch: "按「选项1..N」多路路由",
  focus: "进入专注 N 秒",
  idle: "进入空闲 N 秒",
  ring: "响铃 N 秒",
  if: "旧式二分支条件",
};

/** Output fields a node exposes for {{nodeId.field}} references. */
export const NODE_OUT_FIELDS: Record<string, string[]> = {
  bubble: ["text"],
  agent: ["result", "status", "threadId", "slot"],
  show_window: ["opened"],
  wait: ["elapsedSec"],
  branch: ["matched", "value", "option"],
  focus: ["completed", "elapsedSec"],
  idle: ["completed", "elapsedSec"],
  ring: ["played", "seconds"],
  if: ["matched"],
};

export function defaultParams(kind: string): Record<string, unknown> {
  switch (kind) {
    case "bubble":
      return { text: "", priority: "normal" };
    case "agent":
      return { prompt: "", wait: true, timeout: 600, fillOptions: [] as string[] };
    case "show_window":
      return { target: "chat" };
    case "wait":
      return { seconds: 5 };
    case "branch":
      return { source: "", options: ["专注", "分心"] as string[] };
    case "focus":
      return { seconds: 1500 };
    case "idle":
      return { seconds: 300 };
    case "ring":
      return { seconds: 3 };
    case "if":
      return { source: "", op: "not_empty", value: "" };
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
  manual: "手动",
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