<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { MarkerType, VueFlow, type Edge, type Node, type Connection } from "@vue-flow/core";
import { Background } from "@vue-flow/background";
import "@vue-flow/core/dist/style.css";
import "@vue-flow/core/dist/theme-default.css";
import FlowNode from "./FlowNode.vue";
import WindowHeader from "../../components/WindowHeader.vue";
import { useWorkflowStore } from "../../stores/workflow";
import {
  NODE_DESC,
  NODE_KINDS,
  NODE_LABELS,
  TRIGGER_LABELS,
  defaultParams,
  emptyWorkflow,
  triggerBadge,
  type WorkflowDef,
} from "../../lib/workflow";

const store = useWorkflowStore();
// any[] keeps Vue Flow's deep generics out of vue-tsc inference (TS2589)
const nodes = ref<any[]>([]);
const edges = ref<any[]>([]);
const selectedNodeId = ref<string | null>(null);
const errorMsg = ref("");
const triggerEditor = ref(false);
const running = ref(false);
const nodeStatus = ref<Record<string, string>>({});
// v1.10.5.1 (#66): top-bar save indicator.
const saveState = ref<"saving" | "saved">("saved");
const editingId = ref<string | null>(null);

// v1.10.5 (#59): self-originated auto-saves must never trigger a canvas reload.
let selfSave = false;

const nodeTypes = { workflow: FlowNode as any };

const selectedNode = computed(() =>
  (nodes.value.find((n) => n.id === selectedNodeId.value) as any) ?? null,
);
const sp = computed(() => ((selectedNode.value as any)?.data?.params ?? {}) as Record<string, unknown>);

const meta = ref({
  name: "新工作流",
  trigger: "manual" as string,
  scheduleType: null as string | null,
  intervalMinutes: null as number | null,
  dailyTime: null as string | null,
  guard: "none" as string,
});

const currentWf = computed(() =>
  store.workflows.find((w) => w.id === store.currentWorkflowId) ?? null,
);
const currentName = computed(() =>
  currentWf.value ? currentWf.value.name : meta.value.name,
);
const currentBadge = computed(() => {
  if (currentWf.value) return triggerBadge(currentWf.value);
  return TRIGGER_LABELS[meta.value.trigger] ?? meta.value.trigger;
});

// ---- preset chips (v1.10.5 #60) ----
const PRESETS: Record<string, { label: string; value: number }[]> = {
  wait: [5, 10, 30, 60, 300].map((s) => ({ label: s + " 秒", value: s })),
  focus: [25, 50, 90].map((m) => ({ label: m + " 分钟", value: m * 60 })),
  idle: [5, 10, 15].map((m) => ({ label: m + " 分钟", value: m * 60 })),
  ring: [1, 3, 5, 10].map((s) => ({ label: s + " 秒", value: s })),
  timeout: [5, 10, 30].map((m) => ({ label: m + " 分钟", value: m * 60 })),
  interval: [15, 30, 60, 120].map((m) => ({ label: m + " 分钟", value: m })),
};
const DAILY_PRESETS = ["09:00", "12:00", "14:00", "18:00", "21:00"];

// ---- card-list editors (fillOptions / branch options) ----
const addingFill = ref(false);
const addFillText = ref("");
const fillInput = ref<HTMLInputElement | null>(null);
const addingOpt = ref(false);
const addOptText = ref("");
const optInput = ref<HTMLInputElement | null>(null);

function loadDraft(wf: WorkflowDef | null) {
  if (wf) {
    editingId.value = wf.id;
    meta.value = {
      name: wf.name,
      trigger: wf.trigger,
      scheduleType: wf.scheduleType ?? null,
      intervalMinutes: wf.intervalMinutes ?? null,
      dailyTime: wf.dailyTime ?? null,
      guard: wf.guard,
    };
    nodes.value = wf.nodes.map((n) => ({
      id: n.id,
      type: "workflow",
      position: { x: n.x, y: n.y },
      data: { kind: n.kind, params: { ...n.params }, status: "" },
    }));
    edges.value = wf.edges.map((e) => ({
      id: e.id,
      source: e.source,
      target: e.target,
      sourceHandle: e.sourceHandle || "out",
      markerEnd: MarkerType.ArrowClosed,
    }));
  } else {
    editingId.value = null;
    meta.value = {
      name: "新工作流",
      trigger: "manual",
      scheduleType: null,
      intervalMinutes: null,
      dailyTime: null,
      guard: "none",
    };
    nodes.value = [];
    edges.value = [];
  }
  selectedNodeId.value = null;
  triggerEditor.value = false;
  nodeStatus.value = {};
  running.value = false;
  errorMsg.value = "";
}

function toDraft(): WorkflowDef {
  const base = editingId.value
    ? store.workflows.find((w) => w.id === editingId.value)
    : null;
  const draft: WorkflowDef = base
    ? JSON.parse(JSON.stringify(base))
    : emptyWorkflow();
  draft.name = meta.value.name.trim() || "新工作流";
  draft.trigger = meta.value.trigger;
  draft.scheduleType = meta.value.scheduleType;
  draft.intervalMinutes = meta.value.intervalMinutes;
  draft.dailyTime = meta.value.dailyTime;
  draft.guard = meta.value.guard;
  draft.nodes = nodes.value.map((n) => ({
    id: n.id,
    kind: String(n.data.kind),
    params: (n.data.params as Record<string, unknown>) ?? {},
    x: Math.round(n.position.x),
    y: Math.round(n.position.y),
  }));
  draft.edges = edges.value.map((e) => ({
    id: e.id,
    source: e.source,
    sourceHandle: e.sourceHandle ?? "out",
    target: e.target,
  }));
  return draft;
}

let saveTimer: number | null = null;
function scheduleAutoSave() {
  saveState.value = "saving";
  if (saveTimer !== null) window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => void saveNow(), 800);
}
async function saveNow() {
  saveTimer = null;
  if (nodes.value.length === 0) return; // empty canvas has nothing to persist
  const draft = toDraft();
  selfSave = true;
  try {
    const saved = await store.save(draft);
    editingId.value = saved.id;
    if (store.currentWorkflowId !== saved.id) await store.selectWorkflow(saved.id);
    errorMsg.value = "";
    saveState.value = "saved";
  } catch (e) {
    errorMsg.value = String(e);
    saveState.value = "saved";
  } finally {
    selfSave = false;
  }
}
function flushAutoSave() {
  if (saveTimer !== null) {
    window.clearTimeout(saveTimer);
    saveTimer = null;
    void saveNow();
  }
}

onMounted(async () => {
  // v1.10.5.1 (#66): flush pending auto-save when the window closes.
  window.addEventListener("beforeunload", flushAutoSave);
  await store.init();
  const wf = store.workflows.find((w) => w.id === store.currentWorkflowId);
  loadDraft(wf ?? null);
});

watch(
  () => store.currentWorkflowId,
  (wfId) => {
    // v1.10.5 (#59): never reload for our own auto-save (keeps the node and
    // the in-progress selection); only external switches reload.
    if (selfSave || wfId === editingId.value) return;
    const wf = store.workflows.find((w) => w.id === wfId);
    loadDraft(wf ?? null);
  },
);

watch(
  () => store.externalChangeRevision,
  () => {
    if (selfSave) return;
    const wf = store.workflows.find((w) => w.id === store.currentWorkflowId);
    loadDraft(wf ?? null);
  },
);

onBeforeUnmount(() => {
  window.removeEventListener("beforeunload", flushAutoSave);
  flushAutoSave();
});

// Node/edge edits -> auto-save (v1.10.4 #51: no save button)
watch(
  () => [nodes.value.length, edges.value.length] as const,
  () => scheduleAutoSave(),
);
watch(
  () => nodes.value.map((n) => n.position.x + "," + n.position.y).join("|"),
  () => scheduleAutoSave(),
);

// Latest run result -> per-node status colors
watch(
  () => (store.runs[0] ? store.runs[0].id + ":" + store.runs[0].status : ""),
  () => {
    const latest = store.runs[0];
    if (!latest) return;
    const map: Record<string, string> = {};
    try {
      const log = JSON.parse(latest.nodeLog || "[]") as {
        nodeId: string;
        status: string;
      }[];
      for (const l of log) {
        map[l.nodeId] =
          l.status === "ok" ? "success" : l.status === "failed" ? "failed" : "skipped";
      }
    } catch {
      /* ignore malformed log */
    }
    nodeStatus.value = map;
    running.value = false;
  },
);

function syncNodeStatus() {
  for (const n of nodes.value) {
    n.data.status = nodeStatus.value[n.id] ?? "";
  }
}

function onConnect(conn: Connection) {
  if (!conn.source || !conn.target) return;
  edges.value.push({
    id: "e-" + Date.now(),
    source: conn.source,
    sourceHandle: conn.sourceHandle ?? "out",
    target: conn.target,
    markerEnd: MarkerType.ArrowClosed,
  });
}

function onNodeClick(ev: { node: Node }) {
  selectedNodeId.value = ev.node.id;
}

function onPaneClick() {
  selectedNodeId.value = null;
}

function onEdgeClick(ev: { edge: Edge }) {
  edges.value = edges.value.filter((e) => e.id !== ev.edge.id);
}

function onNodeDragStop() {
  syncNodeStatus();
  scheduleAutoSave();
}

function addNode(kind: string) {
  const len = nodes.value.length;
  const n: Node = {
    id: "n" + Date.now() + "-" + len,
    type: "workflow",
    position: { x: 60 + (len % 4) * 240, y: 60 + Math.floor(len / 4) * 160 },
    data: {
      kind,
      params: defaultParams(kind, {
        characters: store.characters,
        persistedAgentId: localStorage.getItem("focus-agent"),
      }),
      status: "",
    },
  };
  nodes.value.push(n);
  selectedNodeId.value = n.id;
}

function removeSelectedNode() {
  if (!selectedNode.value) return;
  const id = selectedNode.value.id;
  nodes.value = nodes.value.filter((n) => n.id !== id);
  edges.value = edges.value.filter((e) => e.source !== id && e.target !== id);
  selectedNodeId.value = null;
}

function setParam(key: string, value: unknown) {
  const n = selectedNode.value;
  if (!n) return;
  (n.data.params as Record<string, unknown>)[key] = value;
  syncNodeStatus();
  scheduleAutoSave();
}

function setMeta(key: string, value: unknown) {
  (meta.value as Record<string, unknown>)[key] = value;
  scheduleAutoSave();
}

async function runCurrent() {
  if (!store.currentWorkflowId) return;
  const bad = nodes.value.find(
    (n) =>
      n.data.kind === "branch" &&
      n.data.params.condition !== "focus_state" &&
      (!Array.isArray(n.data.params.options) || n.data.params.options.length < 2),
  );
  if (bad) {
    errorMsg.value = "分支「" + bad.id + "」至少需要 2 个选项";
    return;
  }
  try {
    running.value = true;
    nodeStatus.value = {};
    for (const n of nodes.value) n.data.status = "running";
    await store.run(store.currentWorkflowId);
  } catch (e) {
    errorMsg.value = String(e);
    running.value = false;
    syncNodeStatus();
  }
}

// v1.11.1: manual stop — cancel the engine run immediately and reset the UI
// state (running flag + node status) without waiting for a runs_changed event.
function stopCurrent() {
  if (!store.currentWorkflowId) return;
  void store.cancel(store.currentWorkflowId);
  running.value = false;
  for (const n of nodes.value) n.data.status = "";
}

async function applyTrigger() {
  triggerEditor.value = false;
  scheduleAutoSave();
}

function newDraft() {
  flushAutoSave();
  void store.selectWorkflow(null);
  loadDraft(null);
}

async function toggleEnabled(w: { id: string; enabled: boolean }) {
  w.enabled = !w.enabled;
  try {
    await store.save(w as WorkflowDef);
  } catch (e) {
    errorMsg.value = String(e);
  }
}

async function removeWorkflow(w: { id: string; name: string }) {
  if (!window.confirm(`删除工作流「${w.name}」？`)) return;
  try {
    await store.remove(w.id);
    if (store.currentWorkflowId === null) loadDraft(null);
  } catch (e) {
    errorMsg.value = String(e);
  }
}

// ---- card-list helpers ----
function strArray(key: string): string[] {
  const a = sp.value[key];
  return Array.isArray(a) ? (a as string[]).filter((s) => typeof s === "string") : [];
}
function startAddFill() {
  addingFill.value = true;
  addFillText.value = "";
  void nextTick(() => fillInput.value?.focus());
}
function confirmAddFill() {
  const v = addFillText.value.trim();
  if (v) setParam("fillOptions", [...strArray("fillOptions"), v]);
  addingFill.value = false;
}
function removeFillOption(i: number) {
  const a = strArray("fillOptions");
  a.splice(i, 1);
  setParam("fillOptions", a);
}
function startAddOpt() {
  addingOpt.value = true;
  addOptText.value = "";
  void nextTick(() => optInput.value?.focus());
}
function confirmAddOpt() {
  const v = addOptText.value.trim();
  if (v) setParam("options", [...strArray("options"), v]);
  addingOpt.value = false;
}
function removeOpt(i: number) {
  const a = strArray("options");
  a.splice(i, 1);
  setParam("options", a);
}
</script>
<template>
  <div class="wf-window">
    <WindowHeader :title="currentName" collapsible />
    <div class="wf-top">
      <span class="sep" />
      <button class="badge-btn" :class="{ on: currentWf }" @click="triggerEditor = !triggerEditor">
        {{ currentBadge }}
        <span v-if="currentWf && !currentWf.enabled" class="badge-off">已停用</span>
      </button>
      <span v-if="nodes.length" class="save-state" :class="{ ok: saveState === 'saved' }">
        {{ saveState === "saved" ? "已保存✓" : "保存中…" }}
      </span>
      <button v-if="running" class="btn danger" @click="stopCurrent">停止</button>
      <button v-else class="btn accent" :disabled="!store.currentWorkflowId" @click="runCurrent">
        运行
      </button>
      <span v-if="errorMsg" class="err">{{ errorMsg }}</span>
    </div>

    <div v-if="triggerEditor" class="trigger-box">
      <div class="tr-row">
        <span class="label">触发</span>
        <select class="sel" :value="meta.trigger" @change="setMeta('trigger', ($event.target as HTMLSelectElement).value)">
          <option value="manual">保存</option>
          <option value="scheduled">定时</option>
          <option value="focus_end">专注结束</option>
          <option value="supervision_alert">监督告警</option>
        </select>
      </div>
      <template v-if="meta.trigger === 'scheduled'">
        <div class="tr-row">
          <span class="label">方式</span>
          <select class="sel" :value="meta.scheduleType ?? 'interval'" @change="setMeta('scheduleType', ($event.target as HTMLSelectElement).value)">
            <option value="interval">间隔</option>
            <option value="daily">每日</option>
          </select>
        </div>
        <div v-if="meta.scheduleType === 'interval'" class="tr-row">
          <span class="label">每</span>
          <div class="chips">
            <button
              v-for="c in PRESETS.interval"
              :key="c.value"
              class="chip"
              :class="{ on: meta.intervalMinutes === c.value }"
              @click="setMeta('intervalMinutes', c.value)"
            >{{ c.label }}</button>
          </div>
          <input class="ta num" type="number" min="1" :value="meta.intervalMinutes ?? 30"
            @change="setMeta('intervalMinutes', Number(($event.target as HTMLInputElement).value) || 30)" />
          <span class="label">分钟</span>
        </div>
        <div v-else class="tr-row">
          <span class="label">时间</span>
          <div class="chips">
            <button
              v-for="t in DAILY_PRESETS"
              :key="t"
              class="chip"
              :class="{ on: meta.dailyTime === t }"
              @click="setMeta('dailyTime', t)"
            >{{ t }}</button>
          </div>
          <input class="ta" type="time" :value="meta.dailyTime ?? '09:00'"
            @change="setMeta('dailyTime', ($event.target as HTMLInputElement).value || '09:00')" />
        </div>
      </template>
      <div class="tr-row">
        <span class="label">守卫</span>
        <select class="sel" :value="meta.guard" @change="setMeta('guard', ($event.target as HTMLSelectElement).value)">
          <option value="none">无</option>
          <option value="focusing">仅专注中</option>
          <option value="resting">仅休息中</option>
          <option value="idle">仅空闲中</option>
        </select>
      </div>
      <div class="tr-row">
        <button class="btn accent" @click="applyTrigger">完成</button>
        <button class="btn" @click="triggerEditor = false">取消</button>
      </div>
    </div>

    <div class="wf-body">
      <aside class="wf-side">
        <div class="side-head">
          <span>工作流</span>
          <button class="btn accent" @click="newDraft">+ 新建</button>
        </div>
        <div v-if="!store.workflows.length" class="muted">还没有工作流，点「+ 新建」创建</div>
        <div
          v-for="w in store.workflows"
          :key="w.id"
          class="wf-item"
          :class="{ on: w.id === store.currentWorkflowId }"
          @click="store.selectWorkflow(w.id)"
        >
          <div class="wf-item-main">
            <span class="wf-name">{{ w.name }}</span>
            <span class="wf-meta">{{ triggerBadge(w) }}</span>
          </div>
          <div class="wf-item-ops">
            <button class="ghost" :class="{ off: !w.enabled }" @click.stop="toggleEnabled(w)">{{ w.enabled ? "开" : "关" }}</button>
            <button class="ghost" @click.stop="void store.run(w.id)">▶</button>
            <button class="ghost" @click.stop="void store.cancel(w.id)">停</button>
            <button class="ghost danger" @click.stop="removeWorkflow(w)">删</button>
          </div>
        </div>
      </aside>

      <div class="wf-canvas">
        <div v-if="!nodes.length" class="empty-guide">
          <div class="eg-title">空白画布</div>
          <div class="eg-step">1. 从右侧「动作库」点击添加节点</div>
          <div class="eg-step">2. 从节点右侧圆点拖出连线（分支可连多个出口）</div>
          <div class="eg-step">3. 点击顶部「运行」手动触发，或设置定时/事件触发</div>
        </div>
        <VueFlow
          v-model:nodes="nodes"
          v-model:edges="edges"
          :node-types="nodeTypes"
          fit-view-on-init
          :min-zoom="0.3"
          :max-zoom="1.5"
          @connect="onConnect"
          @node-click="onNodeClick"
          @pane-click="onPaneClick"
          @edge-click="onEdgeClick"
          @node-drag-stop="onNodeDragStop"
        >
          <Background :gap="24" />
        </VueFlow>
      </div>
      <aside class="wf-inspector">
        <div class="insp-section">
          <div class="insp-head">动作库</div>
          <div class="palette">
            <button
              v-for="k in [...NODE_KINDS]"
              :key="k"
              class="pal-item"
              :title="NODE_DESC[k]"
              @click="addNode(k)"
            >
              <span class="pal-name">{{ NODE_LABELS[k] }}</span>
              <span class="pal-desc">{{ NODE_DESC[k] }}</span>
            </button>
          </div>
        </div>

        <div class="insp-section">
          <div class="insp-head">
            <span>{{ selectedNode ? NODE_LABELS[String(selectedNode.data.kind)] : "节点参数" }}</span>
            <button v-if="selectedNode" class="ghost danger" @click="removeSelectedNode">删除</button>
          </div>
          <template v-if="selectedNode">
            <div class="insp-body">
              <template v-if="selectedNode.data.kind === 'agent'">
                <label>目标Agent</label>
                <select
                  class="sel"
                  :value="String(sp.characterId ?? '')"
                  @change="setParam('characterId', ($event.target as HTMLSelectElement).value)"
                >
                  <option v-for="c in store.characters" :key="c.id" :value="c.id">{{ c.name }}</option>
                </select>
                <label>提示词（身份由 AGENTS.md 提供，输出纪律系统注入）</label>
                <textarea
                  class="ta"
                  :value="String(sp.prompt ?? '')"
                  @input="setParam('prompt', ($event.target as HTMLTextAreaElement).value)"
                />
                <label class="check">
                  <input type="checkbox" :checked="sp.wait !== false" @change="setParam('wait', ($event.target as HTMLInputElement).checked)" />
                  等待结果（取消则发完即走、不展示）
                </label>
                <template v-if="sp.wait !== false">
                  <label class="check">
                    <input type="checkbox" :checked="sp.showResult !== false" @change="setParam('showResult', ($event.target as HTMLInputElement).checked)" />
                    最终结果（完成后的答复）
                  </label>
                  <label>回复会显示在对话框（一条消息）与宠物泡泡</label>
                  <label>超时</label>
                  <div class="chips">
                    <button
                      v-for="c in PRESETS.timeout"
                      :key="c.value"
                      class="chip"
                      :class="{ on: Number(sp.timeout ?? 600) === c.value }"
                      @click="setParam('timeout', c.value)"
                    >{{ c.label }}</button>
                  </div>
                  <input class="ta num" type="number" min="10" :value="Number(sp.timeout ?? 600)"
                    @change="setParam('timeout', Number(($event.target as HTMLInputElement).value) || 600)" />
                  <label>填空槽候选项（留空 = 自由填空）</label>
                  <div class="card-list">
                    <div v-for="(o, i) in strArray('fillOptions')" :key="i" class="card-chip">
                      <span>{{ o }}</span>
                      <button class="chip-x" @click="removeFillOption(i)">×</button>
                    </div>
                    <div v-if="addingFill" class="card-add">
                      <input ref="fillInput" class="ta" :value="addFillText"
                        @input="addFillText = ($event.target as HTMLInputElement).value"
                        @keydown.enter="confirmAddFill" @keydown.esc="addingFill = false" />
                      <button class="btn accent" @click="confirmAddFill">确定</button>
                    </div>
                    <button v-else class="btn" @click="startAddFill">+ 添加</button>
                  </div>
                </template>
              </template>

              <template v-else-if="selectedNode.data.kind === 'show_window'">
                <label>目标窗口</label>
                <select class="sel" :value="String(sp.target ?? 'chat')" @change="setParam('target', ($event.target as HTMLSelectElement).value)">
                  <option value="chat">对话</option>
                  <option value="stats">统计</option>
                  <option value="music">音乐</option>
                  <option value="workflow">工作流</option>
                </select>
              </template>

              <template v-else-if="selectedNode.data.kind === 'wait'">
                <label>等待秒数</label>
                <div class="chips">
                  <button
                    v-for="c in PRESETS.wait"
                    :key="c.value"
                    class="chip"
                    :class="{ on: Number(sp.seconds ?? 30) === c.value }"
                    @click="setParam('seconds', c.value)"
                  >{{ c.label }}</button>
                </div>
                <input class="ta num" type="number" min="1" max="3600" :value="Number(sp.seconds ?? 30)"
                  @change="setParam('seconds', Number(($event.target as HTMLInputElement).value) || 1)" />
              </template>

              <template v-else-if="selectedNode.data.kind === 'branch'">
                <label>条件来源</label>
                <div class="seg">
                  <button :class="{ on: sp.condition !== 'focus_state' }" @click="setParam('condition', 'slot')">上游 Agent 槽值</button>
                  <button :class="{ on: sp.condition === 'focus_state' }" @click="setParam('condition', 'focus_state')">当前专注状态</button>
                </div>
                <template v-if="sp.condition === 'focus_state'">
                  <label>状态</label>
                  <div class="seg">
                    <button :class="{ on: sp.focusState === 'focus' }" @click="setParam('focusState', 'focus')">专注中</button>
                    <button :class="{ on: sp.focusState === 'rest' }" @click="setParam('focusState', 'rest')">休息中</button>
                    <button :class="{ on: sp.focusState === 'idle' }" @click="setParam('focusState', 'idle')">空闲中</button>
                  </div>
                  <label class="muted">出口：符合 / 不符合</label>
                </template>
                <template v-else>
                  <label>选项（≥2，需连接上游 Agent）</label>
                  <div class="card-list">
                    <div v-for="(o, i) in strArray('options')" :key="i" class="card-chip">
                      <span>{{ o }}</span>
                      <button class="chip-x" @click="removeOpt(i)">×</button>
                    </div>
                    <div v-if="addingOpt" class="card-add">
                      <input ref="optInput" class="ta" :value="addOptText"
                        @input="addOptText = ($event.target as HTMLInputElement).value"
                        @keydown.enter="confirmAddOpt" @keydown.esc="addingOpt = false" />
                      <button class="btn accent" @click="confirmAddOpt">确定</button>
                    </div>
                    <button v-else class="btn" @click="startAddOpt">+ 添加</button>
                  </div>
                </template>
              </template>

              <template v-else-if="selectedNode.data.kind === 'focus' || selectedNode.data.kind === 'idle' || selectedNode.data.kind === 'ring'">
                <label>秒数（focus/idle ≤3600，ring ≤120）</label>
                <div class="chips">
                  <button
                    v-for="c in PRESETS[selectedNode.data.kind as string] ?? []"
                    :key="c.value"
                    class="chip"
                    :class="{ on: Number(sp.seconds ?? c.value) === c.value }"
                    @click="setParam('seconds', c.value)"
                  >{{ c.label }}</button>
                </div>
                <input class="ta num" type="number" min="1" :value="Number(sp.seconds ?? (selectedNode.data.kind === 'ring' ? 3 : 1500))"
                  @change="setParam('seconds', Number(($event.target as HTMLInputElement).value) || 1)" />
              </template>
            </div>
          </template>
          <div v-else class="insp-empty">选择画布中的节点编辑参数</div>
        </div>
      </aside>
    </div>
  </div>
</template>
<style scoped>
.wf-window {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: transparent;
  border: 1px solid var(--glass-border);
  border-radius: var(--r-lg);
  overflow: hidden;
  box-sizing: border-box;
  color: var(--text-hi);
  font-size: 12px;
}
.wf-top {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 8px;
  border-bottom: 1px solid var(--glass-border);
  flex-wrap: wrap;
}
.label { color: var(--text-low); font-size: 11px; }
.sep { flex: 1; }
.sel {
  border: 1px solid var(--glass-border);
  border-radius: var(--r-sm);
  padding: 3px 6px;
  font-size: 11px;
  background: #101a15;
  color: var(--text-hi);
}
.btn {
  border: 1px solid var(--glass-border);
  background: transparent;
  color: var(--text-mid);
  border-radius: var(--r-sm);
  font-size: 10px;
  padding: 2px 8px;
  cursor: pointer;
}
.btn:hover { color: var(--text-hi); border-color: var(--accent); }
.btn.accent { background: var(--accent); color: #0a110e; border-color: var(--accent); font-weight: 600; }
.btn:disabled { opacity: 0.45; cursor: default; }
.ghost {
  border: 1px solid var(--glass-border);
  background: transparent;
  color: var(--text-mid);
  border-radius: var(--r-sm);
  font-size: 10px;
  padding: 1px 5px;
  cursor: pointer;
}
.ghost:hover { color: var(--text-hi); border-color: var(--accent); }
.ghost.danger:hover { color: #ff7b72; border-color: #ff7b72; }
.ghost.off { opacity: 0.5; }
.btn.danger { background: #b23c3c; border-color: #b23c3c; color: #fff; }
.btn.danger:hover { background: #c94f4f; }
.save-state { font-size: 10px; color: var(--text-low); white-space: nowrap; }
.save-state.ok { color: var(--accent); }
.err { color: #ff7b72; font-size: 11px; }
.muted { color: var(--text-low); font-size: 11px; }
.badge-btn {
  border: 1px solid var(--glass-border);
  background: rgba(10, 18, 14, 0.6);
  color: var(--text-mid);
  border-radius: var(--r-pill);
  font-size: 10px;
  padding: 2px 10px;
  cursor: pointer;
}
.badge-btn:hover, .badge-btn.on { border-color: var(--accent); color: var(--accent-bright); }
.badge-off { margin-left: 6px; color: #ff7b72; }
.trigger-box {
  border-bottom: 1px solid var(--glass-border);
  padding: 8px 10px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  background: rgba(10, 18, 14, 0.6);
}
.tr-row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.wf-body {
  flex: 1;
  display: flex;
  min-height: 0;
}
.wf-side {
  width: 150px;
  border-right: 1px solid var(--glass-border);
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 6px;
  overflow-y: auto;
  background: rgba(10, 18, 14, 0.5);
  flex-shrink: 0;
}
.side-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 700;
  font-size: 12px;
  color: var(--accent-bright);
}
.wf-item {
  border: 1px solid var(--glass-border);
  border-radius: var(--r-sm);
  padding: 6px;
  cursor: pointer;
}
.wf-item.on { border-color: var(--accent); background: rgba(163, 230, 53, 0.08); }
.wf-item:hover .wf-item-ops { opacity: 1; }
.wf-item-main { display: flex; flex-direction: column; gap: 2px; }
.wf-name { font-weight: 600; font-size: 11px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.wf-meta { color: var(--text-low); font-size: 9px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.wf-item-ops { display: flex; gap: 3px; margin-top: 4px; flex-wrap: wrap; opacity: 0; transition: opacity 0.12s; }
.wf-canvas { flex: 1; min-width: 0; position: relative; }
.empty-guide {
  position: absolute;
  inset: 20px;
  z-index: 5;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  pointer-events: none;
  color: var(--text-low);
  background: rgba(8, 14, 11, 0.35);
  border: 1px dashed var(--glass-border);
  border-radius: var(--r-md);
}
.eg-title { font-size: 14px; font-weight: 700; color: var(--text-mid); }
.eg-step { font-size: 11px; }
.wf-inspector {
  width: 210px;
  border-left: 1px solid var(--glass-border);
  display: flex;
  flex-direction: column;
  background: rgba(10, 18, 14, 0.5);
  overflow-y: auto;
  flex-shrink: 0;
}
.insp-section { border-bottom: 1px solid var(--glass-border); }
.insp-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px;
  font-weight: 700;
  color: var(--accent-bright);
  font-size: 12px;
}
.palette {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
  padding: 0 8px 10px;
}
.pal-item {
  border: 1px solid var(--glass-border);
  border-radius: var(--r-sm);
  background: rgba(10, 18, 14, 0.6);
  color: var(--text-mid);
  padding: 6px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  cursor: pointer;
  text-align: left;
}
.pal-item:hover { border-color: var(--accent); color: var(--text-hi); }
.pal-name { font-weight: 600; font-size: 11px; color: var(--accent-bright); }
.pal-desc { font-size: 9px; color: var(--text-low); }
.insp-body { padding: 10px; display: flex; flex-direction: column; gap: 6px; }
.insp-body label { color: var(--text-low); font-size: 11px; }
.insp-body label.check { display: flex; align-items: center; gap: 6px; color: var(--text-mid); cursor: pointer; }
.ta {
  width: 100%;
  border: 1px solid var(--glass-border);
  border-radius: var(--r-sm);
  padding: 5px 8px;
  font-size: 11px;
  background: #101a15;
  color: var(--text-hi);
  box-sizing: border-box;
  font-family: inherit;
}
textarea.ta { min-height: 56px; resize: vertical; }
.ta.num { width: 90px; }
.insp-empty { padding: 14px; color: var(--text-low); }
.chips { display: flex; flex-wrap: wrap; gap: 4px; }
.chip {
  border: 1px solid var(--glass-border);
  background: transparent;
  color: var(--text-mid);
  border-radius: var(--r-pill);
  font-size: 10px;
  padding: 2px 8px;
  cursor: pointer;
}
.chip:hover { color: var(--text-hi); border-color: var(--accent); }
.chip.on { background: var(--accent-wash); color: var(--accent-bright); border-color: var(--accent); }
.card-list { display: flex; flex-direction: column; gap: 4px; }
.card-chip {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  border: 1px solid var(--glass-border);
  background: rgba(10, 18, 14, 0.6);
  border-radius: var(--r-sm);
  padding: 3px 8px;
  font-size: 11px;
  color: var(--text-hi);
}
.chip-x {
  border: none;
  background: transparent;
  color: var(--text-low);
  font-size: 12px;
  cursor: pointer;
  line-height: 1;
}
.chip-x:hover { color: #ff7b72; }
.card-add { display: flex; gap: 4px; align-items: center; }
.card-add .ta { flex: 1; }
.seg { display: flex; gap: 4px; flex-wrap: wrap; }
.seg button {
  border: 1px solid var(--glass-border);
  background: transparent;
  color: var(--text-mid);
  border-radius: var(--r-sm);
  padding: 3px 8px;
  font-size: 10px;
  cursor: pointer;
}
.seg button.on { background: var(--accent-wash); color: var(--accent-bright); border-color: var(--accent); }
</style>
