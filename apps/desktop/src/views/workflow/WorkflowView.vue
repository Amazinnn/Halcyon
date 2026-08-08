<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { VueFlow, type Edge, type Node, type Connection } from "@vue-flow/core";
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
  NODE_OUT_FIELDS,
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
const activeParam = ref("");
const refMenu = ref(false);
const errorMsg = ref("");
const copyTarget = ref<{ id: string; action: "copy" | "move" } | null>(null);
const copyCharId = ref("");
const triggerEditor = ref(false);
const running = ref(false);
const nodeStatus = ref<Record<string, string>>({});
const editingId = ref<string | null>(null);

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

const refCandidates = computed(() => {
  if (!selectedNode.value) return [];
  const list: { nodeId: string; field: string }[] = [];
  for (const n of nodes.value) {
    if (n.id === selectedNode.value.id) continue;
    for (const f of NODE_OUT_FIELDS[n.data.kind as string] ?? []) {
      list.push({ nodeId: n.id, field: f });
    }
  }
  return list;
});

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
      markerEnd: "url(#wf-arrow)",
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
  activeParam.value = "";
  refMenu.value = false;
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
    : emptyWorkflow(store.currentCharacterId ?? "");
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
  if (saveTimer !== null) window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => void saveNow(), 800);
}
async function saveNow() {
  saveTimer = null;
  if (nodes.value.length === 0) return; // empty canvas has nothing to persist
  try {
    const saved = await store.save(toDraft());
    editingId.value = saved.id;
    if (store.currentWorkflowId !== saved.id) await store.selectWorkflow(saved.id);
    errorMsg.value = "";
  } catch (e) {
    errorMsg.value = String(e);
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
  await store.init();
  const wf = store.workflows.find((w) => w.id === store.currentWorkflowId);
  loadDraft(wf ?? null);
});

watch(
  () => [store.currentCharacterId, store.currentWorkflowId] as const,
  ([, wfId]) => {
    // Skip reload when the change came from our own auto-save (keeps the
    // in-progress selection/typing intact).
    if (wfId === editingId.value) return;
    const wf = store.workflows.find((w) => w.id === wfId);
    loadDraft(wf ?? null);
  },
);

onBeforeUnmount(() => {
  if (saveTimer !== null) {
    window.clearTimeout(saveTimer);
    saveTimer = null;
  }
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
    markerEnd: "url(#wf-arrow)",
  });
}

function onNodeClick(ev: { node: Node }) {
  selectedNodeId.value = ev.node.id;
  activeParam.value = "";
  refMenu.value = false;
}

function onPaneClick() {
  selectedNodeId.value = null;
  refMenu.value = false;
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
    data: { kind, params: defaultParams(kind), status: "" },
  };
  nodes.value.push(n);
  selectedNodeId.value = n.id;
  activeParam.value = "";
  refMenu.value = false;
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

function insertRef(nodeId: string, field: string) {
  if (!selectedNode.value || !activeParam.value) return;
  const cur = String((selectedNode.value.data.params as Record<string, unknown>)[activeParam.value] ?? "");
  setParam(activeParam.value, cur + "{{" + nodeId + "." + field + "}}");
  refMenu.value = false;
}

function insertSystem(token: string) {
  if (!selectedNode.value || !activeParam.value) return;
  const cur = String((selectedNode.value.data.params as Record<string, unknown>)[activeParam.value] ?? "");
  setParam(activeParam.value, cur + "{{" + token + "}}");
  refMenu.value = false;
}

async function runCurrent() {
  if (!store.currentWorkflowId) return;
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

async function applyTrigger() {
  triggerEditor.value = false;
  scheduleAutoSave();
}

function newDraft() {
  flushAutoSave();
  if (!store.currentCharacterId) return;
  void store.selectWorkflow(null);
  loadDraft(null);
}

function startCopy(w: { id: string; characterId: string }, action: "copy" | "move") {
  copyTarget.value = { id: w.id, action };
  const other = store.characters.find((c) => c.id !== w.characterId);
  copyCharId.value = other?.id ?? store.characters[0]?.id ?? "";
}

async function confirmCopy() {
  if (!copyTarget.value || !copyCharId.value) return;
  try {
    await store.copyTo(copyTarget.value.id, copyCharId.value, copyTarget.value.action === "move");
    copyTarget.value = null;
  } catch (e) {
    errorMsg.value = String(e);
  }
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


function fillOptionsText(): string {
  const a = Array.isArray(sp.value.fillOptions) ? (sp.value.fillOptions as string[]) : [];
  return a.join("\n");
}
function setFillOptions(text: string) {
  const a = text.split("\n").map((s) => s.trim()).filter(Boolean);
  setParam("fillOptions", a);
}
function branchOptionsText(): string {
  const a = Array.isArray(sp.value.options) ? (sp.value.options as string[]) : [];
  return a.join("\n");
}
function setBranchOptions(text: string) {
  const a = text.split("\n").map((s) => s.trim()).filter(Boolean);
  setParam("options", a.length >= 2 ? a : ["专注", "分心"]);
}
</script>

<template>
  <div class="wf-window">
    <WindowHeader :title="currentName" collapsible />
    <div class="wf-top">
      <span class="label">角色</span>
      <select
        class="sel"
        :value="store.currentCharacterId ?? ''"
        @change="store.selectCharacter(($event.target as HTMLSelectElement).value)"
      >
        <option v-for="c in store.characters" :key="c.id" :value="c.id">{{ c.name }}</option>
      </select>
      <span class="sep" />
      <button class="badge-btn" :class="{ on: currentWf }" @click="triggerEditor = !triggerEditor">
        {{ currentBadge }}
        <span v-if="currentWf && !currentWf.enabled" class="badge-off">已停用</span>
      </button>
      <button class="btn accent" :disabled="!store.currentWorkflowId || running" @click="runCurrent">
        {{ running ? "运行中…" : "运行" }}
      </button>
      <span v-if="errorMsg" class="err">{{ errorMsg }}</span>
    </div>

    <div v-if="triggerEditor" class="trigger-box">
      <div class="tr-row">
        <span class="label">触发</span>
        <select class="sel" :value="meta.trigger" @change="setMeta('trigger', ($event.target as HTMLSelectElement).value)">
          <option value="manual">手动</option>
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
          <input class="ta num" type="number" min="1" :value="meta.intervalMinutes ?? 30"
            @change="setMeta('intervalMinutes', Number(($event.target as HTMLInputElement).value) || 30)" />
          <span class="label">分钟</span>
        </div>
        <div v-else class="tr-row">
          <span class="label">时间</span>
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
            <button class="ghost" @click.stop="startCopy(w, 'copy')">复制</button>
            <button class="ghost" @click.stop="startCopy(w, 'move')">迁移</button>
            <button class="ghost danger" @click.stop="removeWorkflow(w)">删</button>
          </div>
        </div>
        <div v-if="copyTarget" class="copy-box">
          <select v-model="copyCharId" class="sel">
            <option v-for="c in store.characters" :key="c.id" :value="c.id">{{ c.name }}</option>
          </select>
          <button class="btn accent" @click="confirmCopy">确认{{ copyTarget.action === "move" ? "迁移" : "复制" }}</button>
          <button class="ghost" @click="copyTarget = null">取消</button>
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
          <svg width="0" height="0">
            <defs>
              <marker id="wf-arrow" markerWidth="10" markerHeight="10" refX="8" refY="3" orient="auto" markerUnits="strokeWidth">
                <path d="M0,0 L0,6 L9,3 z" fill="#7fae8f" />
              </marker>
            </defs>
          </svg>
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
              <template v-if="selectedNode.data.kind === 'bubble'">
                <label>气泡文本</label>
                <textarea class="ta" :value="String(sp.text ?? '')"
                  @focus="activeParam = 'text'; refMenu = true"
                  @input="setParam('text', ($event.target as HTMLTextAreaElement).value)" />
                <label>优先级</label>
                <select class="sel" :value="String(sp.priority ?? 'normal')" @change="setParam('priority', ($event.target as HTMLSelectElement).value)">
                  <option value="normal">normal</option>
                  <option value="high">high</option>
                  <option value="critical">critical</option>
                </select>
              </template>

              <template v-else-if="selectedNode.data.kind === 'agent'">
                <label>提示词（自动注入角色 agents.md 人格）</label>
                <textarea class="ta" :value="String(sp.prompt ?? '')"
                  @focus="activeParam = 'prompt'; refMenu = true"
                  @input="setParam('prompt', ($event.target as HTMLTextAreaElement).value)" />
                <label class="check">
                  <input type="checkbox" :checked="sp.wait !== false" @change="setParam('wait', ($event.target as HTMLInputElement).checked)" />
                  等待结果（取消则发完即走、无输出）
                </label>
                <label>填空槽候选项（每行一个；留空=自由填空）</label>
                <textarea class="ta" rows="3" :value="fillOptionsText()" @input="setFillOptions(($event.target as HTMLTextAreaElement).value)" />
                <label>超时秒数（默认 600）</label>
                <input class="ta num" type="number" min="10" :value="Number(sp.timeout ?? 600)"
                  @change="setParam('timeout', Number(($event.target as HTMLInputElement).value) || 600)" />
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
                <label>等待秒数（1–3600）</label>
                <input class="ta num" type="number" min="1" max="3600" :value="Number(sp.seconds ?? 5)"
                  @change="setParam('seconds', Number(($event.target as HTMLInputElement).value) || 1)" />
              </template>

              <template v-else-if="selectedNode.data.kind === 'branch'">
                <label>判断来源（可引用 <span v-pre>{{node.field}}</span> / <span v-pre>{{system.focus_state}}</span>）</label>
                <input class="ta" :value="String(sp.source ?? '')"
                  @focus="activeParam = 'source'; refMenu = true"
                  @input="setParam('source', ($event.target as HTMLInputElement).value)" />
                <label>选项（每行一个，≥2；命中「选项N」出口，都不命中则流程停在此节点）</label>
                <textarea class="ta" rows="3" :value="branchOptionsText()" @input="setBranchOptions(($event.target as HTMLTextAreaElement).value)" />
              </template>

              <template v-else-if="selectedNode.data.kind === 'focus' || selectedNode.data.kind === 'idle' || selectedNode.data.kind === 'ring'">
                <label>秒数（focus/idle ≤3600，ring ≤120）</label>
                <input class="ta num" type="number" min="1" :value="Number(sp.seconds ?? (selectedNode.data.kind === 'ring' ? 3 : 1500))"
                  @change="setParam('seconds', Number(($event.target as HTMLInputElement).value) || 1)" />
              </template>

              <template v-else-if="selectedNode.data.kind === 'if'">
                <label>判断来源</label>
                <input class="ta" :value="String(sp.source ?? '')"
                  @focus="activeParam = 'source'; refMenu = true"
                  @input="setParam('source', ($event.target as HTMLInputElement).value)" />
                <label>运算</label>
                <select class="sel" :value="String(sp.op ?? 'not_empty')" @change="setParam('op', ($event.target as HTMLSelectElement).value)">
                  <option value="not_empty">非空</option>
                  <option value="contains">包含</option>
                  <option value="equals">等于</option>
                </select>
                <template v-if="sp.op !== 'not_empty'">
                  <label>比较值</label>
                  <input class="ta" :value="String(sp.value ?? '')" @input="setParam('value', ($event.target as HTMLInputElement).value)" />
                </template>
              </template>

              <div v-if="refMenu" class="ref-menu">
                <div class="ref-title">插入引用（{{ activeParam }}）</div>
                <button v-for="r in refCandidates" :key="r.nodeId + r.field" class="ref-item" @click="insertRef(r.nodeId, r.field)">
                  {{ r.nodeId }}.{{ r.field }}
                </button>
                <div class="ref-title">系统字段</div>
                <button class="ref-item" @click="insertSystem('system.focus_state')">system.focus_state</button>
                <button class="ref-item" @click="insertSystem('system.time')">system.time</button>
                <div v-if="!refCandidates.length" class="ref-item muted">没有可引用的前序节点输出</div>
              </div>
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
.tr-row { display: flex; align-items: center; gap: 8px; }
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
.copy-box { display: flex; gap: 4px; align-items: center; padding: 6px 0; flex-wrap: wrap; }
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
.ref-menu { border: 1px solid var(--glass-border); border-radius: var(--r-sm); padding: 6px; display: flex; flex-direction: column; gap: 4px; }
.ref-title { color: var(--text-low); font-size: 10px; }
.ref-item {
  text-align: left;
  border: none;
  background: transparent;
  color: var(--accent-bright);
  font-size: 11px;
  cursor: pointer;
  padding: 2px 0;
}
.ref-item.muted { color: var(--text-low); }
</style>