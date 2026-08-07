<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { VueFlow, type Edge, type Node, type Connection } from "@vue-flow/core";
import { Background } from "@vue-flow/background";
import "@vue-flow/core/dist/style.css";
import "@vue-flow/core/dist/theme-default.css";
import FlowNode from "./FlowNode.vue";
import WindowHeader from "../../components/WindowHeader.vue";
import { useWorkflowStore } from "../../stores/workflow";
import {
  GUARD_LABELS,
  NODE_LABELS,
  TRIGGER_LABELS,
  WORKFLOW_TEMPLATES,
  emptyWorkflow,
  type WorkflowDef,
} from "../../lib/workflow";

const store = useWorkflowStore();
// any[] keeps Vue Flow's deep generics out of vue-tsc inference (TS2589)
const nodes = ref<any[]>([]);
const edges = ref<any[]>([]);
const selectedNodeId = ref<string | null>(null);
const activeParam = ref("");
const refMenu = ref(false);
const dirty = ref(false);
const errorMsg = ref("");
const copyTarget = ref<{ id: string; action: "copy" | "move" } | null>(null);
const copyCharId = ref("");

const nodeTypes = { workflow: FlowNode as any };

const selectedNode = computed(() =>
  (nodes.value.find((n) => n.id === selectedNodeId.value) as any) ?? null,
);

const sp = computed(() => ((selectedNode.value as any)?.data?.params ?? {}) as Record<string, unknown>);

const outFields: Record<string, string[]> = {
  agent: ["result", "threadId", "status"],
  if: ["matched"],
  wait: ["elapsedSec"],
  bubble: ["text"],
  show_window: ["opened"],
};

const refCandidates = computed(() => {
  if (!selectedNode.value) return [];
  const list: { nodeId: string; field: string }[] = [];
  for (const n of nodes.value) {
    if (n.id === selectedNode.value.id) continue;
    for (const f of outFields[n.data.kind as string] ?? []) {
      list.push({ nodeId: n.id, field: f });
    }
  }
  return list;
});

function defaultParams(kind: string): Record<string, unknown> {
  switch (kind) {
    case "bubble":
      return { text: "", priority: "normal" };
    case "agent":
      return { prompt: "", wait: true };
    case "show_window":
      return { target: "chat" };
    case "wait":
      return { seconds: 5 };
    case "if":
      return { source: "", op: "not_empty", value: "" };
    default:
      return {};
  }
}

function loadDraft(wf: WorkflowDef) {
  nodes.value = wf.nodes.map((n) => ({
    id: n.id,
    type: "workflow",
    position: { x: n.x, y: n.y },
    data: { kind: n.kind, params: { ...n.params } },
  }));
  edges.value = wf.edges.map((e) => ({
    id: e.id,
    source: e.source,
    target: e.target,
    sourceHandle: e.sourceHandle || "out",
  }));
  selectedNodeId.value = null;
  activeParam.value = "";
  refMenu.value = false;
  dirty.value = false;
  errorMsg.value = "";
}

function toDraft(): WorkflowDef {
  const base = store.workflows.find((w) => w.id === store.currentWorkflowId);
  const draft: WorkflowDef = base
    ? JSON.parse(JSON.stringify(base))
    : emptyWorkflow(store.currentCharacterId ?? "");
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

onMounted(async () => {
  await store.init();
  if (store.currentWorkflowId) {
    const wf = store.workflows.find((w) => w.id === store.currentWorkflowId);
    if (wf) loadDraft(wf);
  } else if (store.currentCharacterId) {
    loadDraft(emptyWorkflow(store.currentCharacterId));
  }
});

watch(
  () => [store.currentCharacterId, store.currentWorkflowId] as const,
  () => {
    if (dirty.value) return; // keep the in-progress edit until save/reset
    const wf = store.workflows.find((w) => w.id === store.currentWorkflowId);
    if (wf) loadDraft(wf);
    else if (store.currentCharacterId) loadDraft(emptyWorkflow(store.currentCharacterId));
  },
);

function onConnect(conn: Connection) {
  if (!conn.source || !conn.target) return;
  edges.value.push({
    id: "e-" + Date.now(),
    source: conn.source,
    sourceHandle: conn.sourceHandle ?? "out",
    target: conn.target,
  });
  dirty.value = true;
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
  dirty.value = true;
}

function onNodeDrag() {
  dirty.value = true;
}

function addNode(kind: string) {
  const len = nodes.value.length;
  const n: Node = {
    id: "n" + Date.now(),
    type: "workflow",
    position: { x: 60 + (len % 4) * 230, y: 60 + Math.floor(len / 4) * 150 },
    data: { kind, params: defaultParams(kind) },
  };
  nodes.value.push(n);
  selectedNodeId.value = n.id;
  activeParam.value = "";
  refMenu.value = false;
  dirty.value = true;
}

function removeSelectedNode() {
  if (!selectedNode.value) return;
  const id = selectedNode.value.id;
  nodes.value = nodes.value.filter((n) => n.id !== id);
  edges.value = edges.value.filter((e) => e.source !== id && e.target !== id);
  selectedNodeId.value = null;
  dirty.value = true;
}

function setParam(key: string, value: unknown) {
  const n = selectedNode.value;
  if (!n) return;
  (n.data.params as Record<string, unknown>)[key] = value;
  dirty.value = true;
}

function insertRef(nodeId: string, field: string) {
  if (!selectedNode.value || !activeParam.value) return;
  const cur = String((selectedNode.value.data.params as Record<string, unknown>)[activeParam.value] ?? "");
  setParam(activeParam.value, cur + "{{" + nodeId + "." + field + "}}");
  refMenu.value = false;
}

async function save() {
  try {
    const saved = await store.save(toDraft());
    dirty.value = false;
    errorMsg.value = "";
    await store.selectWorkflow(saved.id);
    const wf = store.workflows.find((w) => w.id === saved.id);
    if (wf) loadDraft(wf);
  } catch (e) {
    errorMsg.value = String(e);
  }
}

async function runCurrent() {
  if (!store.currentWorkflowId) return;
  try {
    await store.run(store.currentWorkflowId);
  } catch (e) {
    errorMsg.value = String(e);
  }
}

async function applyTemplate(t: (typeof WORKFLOW_TEMPLATES)[number]) {
  if (!store.currentCharacterId) return;
  const spec = t.build();
  const wf: WorkflowDef = {
    id: "",
    characterId: store.currentCharacterId,
    name: spec.name,
    trigger: spec.trigger,
    scheduleType: spec.scheduleType ?? null,
    intervalMinutes: spec.intervalMinutes ?? null,
    dailyTime: spec.dailyTime ?? null,
    guard: spec.guard,
    nodes: spec.nodes,
    edges: spec.edges,
    enabled: true,
    nextRunAt: null,
  };
  try {
    const saved = await store.save(wf);
    await store.selectWorkflow(saved.id);
    const savedWf = store.workflows.find((w) => w.id === saved.id);
    if (savedWf) loadDraft(savedWf);
  } catch (e) {
    errorMsg.value = String(e);
  }
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

function newDraft() {
  if (!store.currentCharacterId) return;
  loadDraft(emptyWorkflow(store.currentCharacterId));
  void store.selectWorkflow(null);
}

function fmtTime(ts: number): string {
  if (!ts) return "—";
  const d = new Date(ts * 1000);
  return d.toLocaleTimeString("zh-CN", { hour12: false });
}
</script>

<template>
  <div class="wf-window">
    <WindowHeader title="工作流" collapsible />
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
      <button v-for="t in WORKFLOW_TEMPLATES" :key="t.key" class="btn" :title="t.desc" @click="applyTemplate(t)">
        {{ t.label }}
      </button>
      <button class="btn accent" @click="save">保存</button>
      <button class="btn" :disabled="!store.currentWorkflowId" @click="runCurrent">运行</button>
      <span v-if="dirty" class="dirty">未保存</span>
      <span v-if="errorMsg" class="err">{{ errorMsg }}</span>
    </div>

    <div class="wf-body">
      <aside class="wf-side">
        <div class="side-head">
          <span>工作流</span>
          <button class="btn" @click="newDraft">新建</button>
        </div>
        <div v-if="!store.workflows.length" class="muted">还没有工作流，用上方模板或「新建」开始</div>
        <div
          v-for="w in store.workflows"
          :key="w.id"
          class="wf-item"
          :class="{ on: w.id === store.currentWorkflowId }"
          @click="store.selectWorkflow(w.id)"
        >
          <div class="wf-item-main">
            <span class="wf-name">{{ w.name }}</span>
            <span class="wf-meta">{{ TRIGGER_LABELS[w.trigger] }} · {{ GUARD_LABELS[w.guard] ?? w.guard }}</span>
          </div>
          <div class="wf-item-ops">
            <button class="ghost" :class="{ off: !w.enabled }" @click.stop="toggleEnabled(w)">{{ w.enabled ? "开" : "关" }}</button>
            <button class="ghost" @click.stop="void store.run(w.id)">▶</button>
            <button class="ghost" @click.stop="void store.cancel(w.id)">停</button>
            <button class="ghost" @click.stop="startCopy(w, 'copy')">复制</button>
            <button class="ghost" @click.stop="startCopy(w, 'move')">迁移</button>
            <button class="ghost danger" @click.stop="void store.remove(w.id)">删</button>
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
        <div class="wf-palette">
          <button v-for="k in ['bubble', 'agent', 'show_window', 'wait', 'if']" :key="k" class="btn" @click="addNode(k)">
            +{{ NODE_LABELS[k] }}
          </button>
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
          @node-drag-stop="onNodeDrag"
        >
          <Background :gap="24" />
        </VueFlow>
      </div>

      <aside class="wf-inspector">
        <template v-if="selectedNode">
          <div class="insp-head">
            <span>{{ NODE_LABELS[String(selectedNode.data.kind)] }}</span>
            <button class="ghost danger" @click="removeSelectedNode">删除节点</button>
          </div>
          <div class="insp-body">
            <template v-if="selectedNode.data.kind === 'bubble'">
              <label>气泡文本</label>
              <textarea
                class="ta"
                :value="String(sp.text ?? '')"
                @focus="activeParam = 'text'; refMenu = true"
                @input="setParam('text', ($event.target as HTMLTextAreaElement).value)"
              />
              <label>优先级</label>
              <select class="sel" :value="String(sp.priority ?? 'normal')" @change="setParam('priority', ($event.target as HTMLSelectElement).value)">
                <option value="normal">normal</option>
                <option value="high">high</option>
                <option value="critical">critical</option>
              </select>
            </template>
            <template v-else-if="selectedNode.data.kind === 'agent'">
              <label>提示词（自动注入角色 agents.md 人格）</label>
              <textarea
                class="ta"
                :value="String(sp.prompt ?? '')"
                @focus="activeParam = 'prompt'; refMenu = true"
                @input="setParam('prompt', ($event.target as HTMLTextAreaElement).value)"
              />
              <label class="check">
                <input type="checkbox" :checked="sp.wait !== false" @change="setParam('wait', ($event.target as HTMLInputElement).checked)" />
                等待结果（取消则发完即走、无输出）
              </label>
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
              <input
                type="number"
                class="ta"
                min="1"
                max="3600"
                :value="Number(sp.seconds ?? 5)"
                @input="setParam('seconds', Number(($event.target as HTMLInputElement).value) || 1)"
              />
            </template>
            <template v-else-if="selectedNode.data.kind === 'if'">
              <label>判断来源（可引用前序输出）</label>
              <input
                class="ta"
                :value="String(sp.source ?? '')"
                @focus="activeParam = 'source'; refMenu = true"
                @input="setParam('source', ($event.target as HTMLInputElement).value)"
              />
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
              <div v-if="!refCandidates.length" class="ref-item muted">没有可引用的前序节点输出</div>
            </div>
          </div>
        </template>
        <template v-else>
          <div class="insp-empty">选中节点编辑参数</div>
        </template>
      </aside>
    </div>

    <div class="wf-runs">
      <div class="runs-head">
        <span>运行记录（{{ store.currentWorkflowId ? store.runs.length : 0 }}）</span>
      </div>
      <div v-if="!store.runs.length" class="muted">尚无运行记录（可用 focus-cli workflow run 触发）</div>
      <div v-for="r in store.runs.slice(0, 8)" :key="r.id" class="run-item">
        <span class="run-status" :class="r.status">{{ r.status }}</span>
        <span class="run-by">{{ r.triggeredBy }}</span>
        <span class="run-time">{{ fmtTime(r.startedAt) }}</span>
        <span v-if="r.error" class="run-err" :title="r.error">{{ r.error }}</span>
      </div>
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
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--glass-border);
  flex-wrap: wrap;
}
.label { color: var(--text-low); }
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
  font-size: 11px;
  padding: 3px 10px;
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
  padding: 1px 6px;
  cursor: pointer;
}
.ghost:hover { color: var(--text-hi); border-color: var(--accent); }
.ghost.danger:hover { color: #ff7b72; border-color: #ff7b72; }
.ghost.off { opacity: 0.5; }
.dirty { color: #e8c766; font-size: 11px; }
.err { color: #ff7b72; font-size: 11px; }
.muted { color: var(--text-low); font-size: 11px; }
.wf-body {
  flex: 1;
  display: flex;
  min-height: 0;
}
.wf-side {
  width: 230px;
  border-right: 1px solid var(--glass-border);
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 8px;
  overflow-y: auto;
  background: rgba(10, 18, 14, 0.5);
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
.wf-item-main { display: flex; flex-direction: column; gap: 2px; }
.wf-name { font-weight: 600; }
.wf-meta { color: var(--text-low); font-size: 10px; }
.wf-item-ops { display: flex; gap: 4px; margin-top: 5px; flex-wrap: wrap; }
.copy-box { display: flex; gap: 6px; align-items: center; padding: 6px 0; flex-wrap: wrap; }
.wf-canvas { flex: 1; min-width: 0; position: relative; }
.wf-palette {
  position: absolute;
  top: 8px;
  left: 8px;
  z-index: 10;
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  max-width: 420px;
}
.wf-inspector {
  width: 250px;
  border-left: 1px solid var(--glass-border);
  display: flex;
  flex-direction: column;
  background: rgba(10, 18, 14, 0.5);
  overflow-y: auto;
}
.insp-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px;
  border-bottom: 1px solid var(--glass-border);
  font-weight: 700;
  color: var(--accent-bright);
}
.insp-body { padding: 10px; display: flex; flex-direction: column; gap: 6px; }
.insp-body label { color: var(--text-low); font-size: 11px; }
.insp-body label.check { display: flex; align-items: center; gap: 6px; color: var(--text-mid); cursor: pointer; }
.ta {
  width: 100%;
  border: 1px solid var(--glass-border);
  border-radius: var(--r-sm);
  padding: 5px 8px;
  font-size: 12px;
  background: #101a15;
  color: var(--text-hi);
  box-sizing: border-box;
  font-family: inherit;
}
textarea.ta { min-height: 64px; resize: vertical; }
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
.wf-runs {
  border-top: 1px solid var(--glass-border);
  padding: 6px 12px;
  max-height: 120px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.runs-head { display: flex; justify-content: space-between; font-size: 11px; color: var(--text-low); }
.run-item { display: flex; gap: 8px; align-items: center; font-size: 11px; }
.run-status { border-radius: var(--r-pill); padding: 1px 8px; background: #183624; color: var(--accent-bright); }
.run-status.failed, .run-status.error { background: #4a1d1d; color: #ffb4ae; }
.run-status.skipped, .run-status.cancelled { background: #3a3318; color: #e8c766; }
.run-by { color: var(--text-mid); }
.run-time { color: var(--text-low); }
.run-err { color: #ff7b72; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 320px; }
</style>