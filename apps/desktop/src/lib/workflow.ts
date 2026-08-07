// M4 workflow templates (ADR-0012): three presets the user can one-click
// create and then edit freely. Wire shapes match the Rust WorkflowDef.

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

export const NODE_KINDS = ["bubble", "agent", "show_window", "wait", "if"] as const;

export const NODE_LABELS: Record<string, string> = {
  bubble: "气泡",
  agent: "发送给 Agent",
  show_window: "显示窗口",
  wait: "等待",
  if: "条件 IF",
};

export interface TemplateSpec {
  key: string;
  label: string;
  desc: string;
  build: () => { name: string; trigger: string; scheduleType?: string | null; intervalMinutes?: number | null; dailyTime?: string | null; guard: string; nodes: WorkflowNode[]; edges: WorkflowEdge[] };
}

function nid(prefix: string, n: number): string {
  return prefix + n;
}

export const WORKFLOW_TEMPLATES: TemplateSpec[] = [
  {
    key: "focus_end",
    label: "专注结束收尾",
    desc: "专注结束后让角色总结这一轮",
    build: () => ({
      name: "专注结束收尾",
      trigger: "focus_end",
      guard: "none",
      nodes: [
        { id: nid("a", 1), kind: "agent", params: { prompt: "我刚完成一轮专注，请帮我简短总结这一轮做了什么，并建议下一步。", wait: true }, x: 40, y: 60 },
        { id: nid("b", 1), kind: "bubble", params: { text: "{{a1.result}}", priority: "normal" }, x: 360, y: 60 },
      ],
      edges: [{ id: "e1", source: "a1", sourceHandle: "out", target: "b1" }],
    }),
  },
  {
    key: "scheduled_check",
    label: "定时自检",
    desc: "每 30 分钟让角色自检（仅专注中）",
    build: () => ({
      name: "定时自检",
      trigger: "scheduled",
      scheduleType: "interval",
      intervalMinutes: 30,
      guard: "focusing",
      nodes: [
        { id: nid("a", 1), kind: "agent", params: { prompt: "定时自检：请简短确认我当前是否仍在专注，并提醒我当前任务。", wait: true }, x: 40, y: 60 },
        { id: nid("i", 1), kind: "if", params: { source: "{{a1.result}}", op: "contains", value: "分心" }, x: 380, y: 60 },
        { id: nid("b", 1), kind: "bubble", params: { text: "检测到可能分心，回到当前任务！", priority: "high" }, x: 700, y: 10 },
      ],
      edges: [
        { id: "e1", source: "a1", sourceHandle: "out", target: "i1" },
        { id: "e2", source: "i1", sourceHandle: "true", target: "b1" },
      ],
    }),
  },
  {
    key: "supervision_soothe",
    label: "监督安抚",
    desc: "监督告警后提醒并让角色补一句",
    build: () => ({
      name: "监督安抚",
      trigger: "supervision_alert",
      guard: "none",
      nodes: [
        { id: nid("b", 1), kind: "bubble", params: { text: "检测到分心，快回到当前任务！", priority: "high" }, x: 40, y: 20 },
        { id: nid("w", 1), kind: "wait", params: { seconds: 60 }, x: 380, y: 60 },
        { id: nid("a", 1), kind: "agent", params: { prompt: "我刚刚分心了，请提醒我回到当前任务。", wait: false }, x: 700, y: 100 },
      ],
      edges: [
        { id: "e1", source: "b1", sourceHandle: "out", target: "w1" },
        { id: "e2", source: "w1", sourceHandle: "out", target: "a1" },
      ],
    }),
  },
];

export function emptyWorkflow(characterId: string, name = "新工作流"): WorkflowDef {
  return {
    id: "",
    characterId,
    name,
    trigger: "manual",
    guard: "none",
    nodes: [
      { id: "n1", kind: "bubble", params: { text: "你好，我是这个工作流的第一个节点", priority: "normal" }, x: 40, y: 60 },
    ],
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

export const GUARD_LABELS: Record<string, string> = {
  none: "无",
  focusing: "仅专注中",
  resting: "仅休息中",
  idle: "仅空闲中",
};