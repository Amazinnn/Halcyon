<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { MarkerType, VueFlow, type Edge, type Node, type Connection } from "@vue-flow/core";
import { Background } from "@vue-flow/background";
import "@vue-flow/core/dist/style.css";
import "@vue-flow/core/dist/theme-default.css";
import FlowNode from "./FlowNode.vue";
import FlowTriggerNode from "./FlowTriggerNode.vue";
import FocusWindowFrame from "../../components/focus/FocusWindowFrame.vue";
import FocusButton from "../../components/focus/FocusButton.vue";
import FocusSegmented from "../../components/focus/FocusSegmented.vue";
import FocusSelect from "../../components/focus/FocusSelect.vue";
import { useWorkflowStore } from "../../stores/workflow";
import {
  NODE_DESC,
  NODE_KINDS,
  NODE_LABELS,
  defaultParams,
  emptyWorkflow,
  triggerBadge,
  type WorkflowDef,
} from "../../lib/workflow";

const store = useWorkflowStore();
const TRIGGER_NODE_ID = "__trigger__";
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

const nodeTypes = { workflow: FlowNode as any, trigger: FlowTriggerNode as any };

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
  weeklyDay: null as number | null,
  weeklyTime: null as string | null,
  guard: "none" as string,
});

const currentWf = computed(() =>
  store.workflows.find((w) => w.id === store.currentWorkflowId) ?? null,
);
const currentName = computed(() =>
  currentWf.value ? currentWf.value.name : meta.value.name,
);
const currentBadge = computed(() => {
  return triggerBadge({ ...emptyWorkflow(), ...meta.value });
});

const actionNodes = computed(() =>
  nodes.value.filter((node) => node.id !== TRIGGER_NODE_ID),
);

function triggerNode() {
  return {
    id: TRIGGER_NODE_ID,
    type: "trigger",
    position: { x: -150, y: 96 },
    data: { detail: currentBadge.value },
    draggable: false,
    connectable: false,
  };
}

function refreshTriggerNode() {
  const node = nodes.value.find((item) => item.id === TRIGGER_NODE_ID);
  if (node) node.data.detail = currentBadge.value;
}

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
      weeklyDay: wf.weeklyDay ?? null,
      weeklyTime: wf.weeklyTime ?? null,
      guard: wf.guard,
    };
    nodes.value = [triggerNode(), ...wf.nodes.map((n) => ({
      id: n.id,
      type: "workflow",
      position: { x: n.x, y: n.y },
      data: { kind: n.kind, params: { ...n.params }, status: "" },
    }))];
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
      weeklyDay: null,
      weeklyTime: null,
      guard: "none",
    };
    nodes.value = [triggerNode()];
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
  draft.weeklyDay = meta.value.weeklyDay;
  draft.weeklyTime = meta.value.weeklyTime;
  draft.guard = meta.value.guard;
  draft.nodes = actionNodes.value.map((n) => ({
    id: n.id,
    kind: String(n.data.kind),
    params: (n.data.params as Record<string, unknown>) ?? {},
    x: Math.round(n.position.x),
    y: Math.round(n.position.y),
  }));
  draft.edges = edges.value
    .filter((e) => e.source !== TRIGGER_NODE_ID && e.target !== TRIGGER_NODE_ID)
    .map((e) => ({
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
  if (actionNodes.value.length === 0) return; // empty canvas has nothing to persist
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
  () => store.lastExternalChange,
  (change) => {
    if (selfSave || !change?.affectsCurrentDraft || change.workflowId !== editingId.value) return;
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
  () => [actionNodes.value.length, edges.value.length] as const,
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
  if (!conn.source || !conn.target || conn.source === TRIGGER_NODE_ID || conn.target === TRIGGER_NODE_ID) return;
  edges.value.push({
    id: "e-" + Date.now(),
    source: conn.source,
    sourceHandle: conn.sourceHandle ?? "out",
    target: conn.target,
    markerEnd: MarkerType.ArrowClosed,
  });
}

function onNodeClick(ev: { node: Node }) {
  if (ev.node.id === TRIGGER_NODE_ID) {
    triggerEditor.value = true;
    selectedNodeId.value = null;
    return;
  }
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
  const len = actionNodes.value.length;
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
  if (id === TRIGGER_NODE_ID) return;
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
  refreshTriggerNode();
  scheduleAutoSave();
}

async function runCurrent() {
  if (!store.currentWorkflowId) return;
  const bad = actionNodes.value.find(
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
    for (const n of actionNodes.value) n.data.status = "running";
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
  for (const n of actionNodes.value) n.data.status = "";
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
    <FocusWindowFrame :title="currentName" collapsible />
    <div class="wf-top">
      <span class="sep" />
      <button class="badge-btn" :class="{ on: currentWf }" @click="triggerEditor = !triggerEditor">
        {{ currentBadge }}
        <span v-if="currentWf && !currentWf.enabled" class="badge-off">已停用</span>
      </button>
      <span v-if="nodes.length" class="save-state" :class="{ ok: saveState === 'saved' }">
        {{ saveState === "saved" ? "已保存✓" : "保存中…" }}
      </span>
      <FocusButton v-if="running" variant="danger" size="xs" @click="stopCurrent">停止</FocusButton>
      <FocusButton v-else variant="accent" size="xs" :disabled="!store.currentWorkflowId" @click="runCurrent">运行</FocusButton>
      <span v-if="errorMsg" class="err">{{ errorMsg }}</span>
    </div>

    <div v-if="triggerEditor" class="trigger-box">
      <div class="tr-row">
        <span class="label">触发</span>
        <FocusSelect class="sel" :model-value="meta.trigger" :options="[{ label: '保存', value: 'manual' }, { label: '定时', value: 'scheduled' }, { label: '专注结束', value: 'focus_end' }, { label: '监督告警', value: 'supervision_alert' }]" @update:model-value="(v) => setMeta('trigger', v)" />
      </div>
      <template v-if="meta.trigger === 'scheduled'">
        <div class="tr-row">
          <span class="label">方式</span>
          <FocusSelect class="sel" :model-value="meta.scheduleType ?? 'interval'" :options="[{ label: '间隔', value: 'interval' }, { label: '每日', value: 'daily' }, { label: '每周', value: 'weekly' }]" @update:model-value="(v) => setMeta('scheduleType', v)" />
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
        <div v-else-if="meta.scheduleType === 'daily'" class="tr-row">
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
        <div v-else class="tr-row">
          <span class="label">每周</span>
          <FocusSelect class="sel" :model-value="String(meta.weeklyDay ?? 0)" :options="[{ label: '周一', value: '0' }, { label: '周二', value: '1' }, { label: '周三', value: '2' }, { label: '周四', value: '3' }, { label: '周五', value: '4' }, { label: '周六', value: '5' }, { label: '周日', value: '6' }]" @update:model-value="(v) => setMeta('weeklyDay', Number(v))" />
          <input class="ta" type="time" :value="meta.weeklyTime ?? '09:00'"
            @change="setMeta('weeklyTime', ($event.target as HTMLInputElement).value || '09:00')" />
        </div>
      </template>
      <div class="tr-row">
        <span class="label">守卫</span>
        <FocusSelect class="sel" :model-value="meta.guard" :options="[{ label: '无', value: 'none' }, { label: '仅专注中', value: 'focusing' }, { label: '仅休息中', value: 'resting' }, { label: '仅空闲中', value: 'idle' }]" @update:model-value="(v) => setMeta('guard', v)" />
      </div>
      <div class="tr-row">
        <FocusButton variant="accent" size="xs" @click="applyTrigger">完成</FocusButton>
        <FocusButton variant="default" size="xs" @click="triggerEditor = false">取消</FocusButton>
      </div>
    </div>

    <div class="wf-body">
      <aside class="wf-side">
        <div class="side-head">
          <span>工作流</span>
          <FocusButton variant="accent" size="xs" @click="newDraft">+ 新建</FocusButton>
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
            <FocusButton variant="ghost" size="tight" :off="!w.enabled" @click.stop="toggleEnabled(w)">{{ w.enabled ? "开" : "关" }}</FocusButton>
            <FocusButton variant="ghost" size="tight" @click.stop="void store.run(w.id)">▶</FocusButton>
            <FocusButton variant="ghost" size="tight" @click.stop="void store.cancel(w.id)">停</FocusButton>
            <FocusButton variant="ghost" size="tight" class="danger" @click.stop="removeWorkflow(w)">删</FocusButton>
          </div>
        </div>
      </aside>

      <div class="wf-canvas">
        <div v-if="!actionNodes.length" class="empty-guide">
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
            <FocusButton v-if="selectedNode" variant="ghost" size="tight" class="danger" @click="removeSelectedNode">删除</FocusButton>
          </div>
          <template v-if="selectedNode">
            <div class="insp-body">
              <template v-if="selectedNode.data.kind === 'agent'">
                <label>目标Agent</label>
                <FocusSelect class="sel" :model-value="String(sp.characterId ?? '')" :options="store.characters.map((c) => ({ label: c.name, value: c.id }))" @update:model-value="(v) => setParam('characterId', v)" />
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
                      <FocusButton variant="accent" size="xs" @click="confirmAddFill">确定</FocusButton>
                    </div>
                    <FocusButton v-else variant="default" size="xs" @click="startAddFill">+ 添加</FocusButton>
                  </div>
                </template>
              </template>

              <template v-else-if="selectedNode.data.kind === 'show_window'">
                <label>目标窗口</label>
                <FocusSelect class="sel" :model-value="String(sp.target ?? 'chat')" :options="[{ label: '对话', value: 'chat' }, { label: '统计', value: 'stats' }, { label: '音乐', value: 'music' }, { label: '工作流', value: 'workflow' }]" @update:model-value="(v) => setParam('target', v)" />
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
                <FocusSegmented variant="soft" :model-value="String(sp.condition ?? 'slot')" :options="[{ label: '上游 Agent 槽值', value: 'slot' }, { label: '当前专注状态', value: 'focus_state' }]" @update:model-value="(v) => setParam('condition', v)" />
                <template v-if="sp.condition === 'focus_state'">
                  <label>状态</label>
                  <FocusSegmented variant="soft" :model-value="String(sp.focusState ?? '')" :options="[{ label: '专注中', value: 'focus' }, { label: '休息中', value: 'rest' }, { label: '空闲中', value: 'idle' }]" @update:model-value="(v) => setParam('focusState', v)" />
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
                      <FocusButton variant="accent" size="xs" @click="confirmAddOpt">确定</FocusButton>
                    </div>
                    <FocusButton v-else variant="default" size="xs" @click="startAddOpt">+ 添加</FocusButton>
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
  border-radius: var(--window-host-radius);
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
.sel { font-size: 11px; }
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
</style>