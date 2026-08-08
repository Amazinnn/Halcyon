<script setup lang="ts">
import { computed } from "vue";
import { Handle, Position } from "@vue-flow/core";
import { NODE_DESC, NODE_LABELS, NODE_OUT_FIELDS } from "../../lib/workflow";

const props = defineProps<{
  id: string;
  data: { kind: string; params: Record<string, unknown>; status?: string };
}>();

const kind = computed(() => props.data.kind);
const p = computed(() => props.data.params ?? {});
const status = computed(() => props.data.status ?? "");

const summary = computed(() => {
  switch (kind.value) {
    case "bubble":
      return String(p.value.text ?? "");
    case "agent":
      return (
        (p.value.wait === false ? "[不等待] " : "[等待结果] ") +
        String(p.value.prompt ?? "")
      );
    case "show_window":
      return "打开 " + String(p.value.target ?? "chat");
    case "wait":
      return "等待 " + String(p.value.seconds ?? 5) + " 秒";
    case "branch": {
      const opts = Array.isArray(p.value.options) ? p.value.options : [];
      return (
        String(p.value.source ?? "") +
        " → " +
        opts.map((_, i) => `选项${i + 1}`).join(" / ")
      );
    }
    case "focus":
      return "专注 " + String(p.value.seconds ?? 1500) + " 秒";
    case "idle":
      return "空闲 " + String(p.value.seconds ?? 300) + " 秒";
    case "ring":
      return "响铃 " + String(p.value.seconds ?? 3) + " 秒";
    case "if":
      return (
        String(p.value.source ?? "") +
        " " +
        String(p.value.op ?? "not_empty") +
        (p.value.value ? " " + p.value.value : "")
      );
    default:
      return "";
  }
});

/** {{nodeId.field}} references embedded in the node's text params. */
const inputRefs = computed(() => {
  const textParams = ["prompt", "text", "source"];
  const found = new Set<string>();
  for (const k of textParams) {
    const v = String(p.value[k] ?? "");
    const re = /\{\{\s*([\w.-]+)\s*\}\}/g;
    let m: RegExpExecArray | null;
    while ((m = re.exec(v))) found.add(m[1]);
  }
  return [...found];
});

const outFields = computed(() => NODE_OUT_FIELDS[kind.value] ?? []);

const branchOptions = computed(() => {
  if (kind.value !== "branch") return [];
  const opts = Array.isArray(p.value.options) ? (p.value.options as unknown[]) : [];
  return opts.length >= 2 ? opts : [];
});
</script>

<template>
  <div class="wf-node" :class="[kind, status]">
    <Handle type="target" :position="Position.Left" />
    <div class="wf-node-head">
      <span class="wf-node-title">{{ NODE_LABELS[kind] ?? kind }}</span>
      <span class="wf-node-desc">{{ NODE_DESC[kind] ?? "" }}</span>
    </div>
    <div class="wf-node-body">{{ summary || "（未配置）" }}</div>
    <div v-if="inputRefs.length" class="wf-node-badges">
      <span v-for="r in inputRefs" :key="r" class="badge ref">{{ r }}</span>
    </div>
    <div v-if="outFields.length" class="wf-node-badges">
      <span v-for="f in outFields" :key="f" class="badge out">{{ f }}</span>
    </div>
    <template v-if="kind === 'branch'">
      <Handle
        v-for="(_, i) in branchOptions"
        :key="'option' + (i + 1)"
        type="source"
        :position="Position.Right"
        :id="'option' + (i + 1)"
        :style="{ top: ((i + 1) / (branchOptions.length + 1)) * 100 + '%' }"
      />
      <span
        v-for="(_, i) in branchOptions"
        :key="'l' + i"
        class="h-label"
        :style="{ top: ((i + 1) / (branchOptions.length + 1)) * 100 + '%' }"
      >选项{{ i + 1 }}</span>
    </template>
    <template v-else-if="kind === 'if'">
      <Handle type="source" :position="Position.Right" id="true" :style="{ top: '32%' }" />
      <Handle type="source" :position="Position.Right" id="false" :style="{ top: '72%' }" />
      <span class="h-label t">是</span>
      <span class="h-label f">否</span>
    </template>
    <Handle v-else type="source" :position="Position.Right" id="out" />
  </div>
</template>

<style scoped>
.wf-node {
  min-width: 178px;
  max-width: 250px;
  background: rgba(16, 26, 21, 0.94);
  border: 1px solid var(--glass-border);
  border-radius: var(--r-md, 10px);
  color: var(--text-hi);
  font-size: 12px;
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.35);
  transition: border-color 0.15s;
}
.wf-node.agent { border-color: rgba(163, 230, 53, 0.6); }
.wf-node.branch { border-color: rgba(232, 199, 102, 0.6); }
.wf-node.wait { border-color: rgba(120, 180, 255, 0.5); }
.wf-node.focus { border-color: rgba(163, 230, 53, 0.5); }
.wf-node.idle { border-color: rgba(120, 180, 255, 0.5); }
.wf-node.ring { border-color: rgba(255, 160, 120, 0.55); }
.wf-node.if { border-color: rgba(232, 199, 102, 0.6); }
.wf-node.running { border-color: #4f9dff; box-shadow: 0 0 0 2px rgba(79, 157, 255, 0.25); }
.wf-node.success { border-color: #2ecc71; }
.wf-node.failed { border-color: #ff5555; }
.wf-node.skipped { border-color: var(--glass-border); opacity: 0.6; }
.wf-node-head {
  padding: 5px 10px;
  display: flex;
  flex-direction: column;
  gap: 1px;
  border-bottom: 1px solid var(--glass-border);
}
.wf-node-title { font-weight: 700; font-size: 11px; color: var(--accent-bright); }
.wf-node-desc { font-size: 10px; color: var(--text-low); }
.wf-node-body {
  padding: 6px 10px;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 84px;
  overflow: hidden;
  font-size: 11px;
}
.wf-node-badges {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 0 10px 6px;
}
.badge {
  font-size: 9px;
  border-radius: 4px;
  padding: 1px 5px;
  font-family: ui-monospace, Consolas, monospace;
}
.badge.ref { background: rgba(120, 180, 255, 0.14); color: #9cc7ff; }
.badge.out { background: rgba(163, 230, 53, 0.12); color: var(--accent-bright); }
.h-label {
  position: absolute;
  right: -40px;
  font-size: 9px;
  color: var(--text-low);
  white-space: nowrap;
}
.h-label.t { top: 22%; }
.h-label.f { top: 62%; }
</style>