<script setup lang="ts">
import { computed } from "vue";
import { Handle, Position } from "@vue-flow/core";
import { NODE_LABELS } from "../../lib/workflow";

const props = defineProps<{
  id: string;
  data: { kind: string; params: Record<string, unknown> };
}>();

const kind = computed(() => props.data.kind);
const p = computed(() => props.data.params ?? {});
const summary = computed(() => {
  switch (kind.value) {
    case "bubble":
      return String(p.value.text ?? "");
    case "agent":
      return (p.value.wait === false ? "[发完即走] " : "[等待结果] ") + String(p.value.prompt ?? "");
    case "show_window":
      return "打开 " + String(p.value.target ?? "chat");
    case "wait":
      return "等待 " + String(p.value.seconds ?? 5) + " 秒";
    case "if":
      return (p.value.source ?? "") + " " + String(p.value.op ?? "not_empty") + (p.value.value ? " " + p.value.value : "");
    default:
      return "";
  }
});
</script>

<template>
  <div class="wf-node" :class="kind">
    <Handle type="target" :position="Position.Left" />
    <div class="wf-node-head">{{ NODE_LABELS[kind] ?? kind }}</div>
    <div class="wf-node-body">{{ summary }}</div>
    <template v-if="kind === 'if'">
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
  min-width: 170px;
  max-width: 260px;
  background: rgba(16, 26, 21, 0.92);
  border: 1px solid var(--glass-border);
  border-radius: var(--r-md, 10px);
  color: var(--text-hi);
  font-size: 12px;
  box-shadow: 0 4px 14px rgba(0, 0, 0, 0.35);
}
.wf-node.agent { border-color: rgba(163, 230, 53, 0.6); }
.wf-node.if { border-color: rgba(232, 199, 102, 0.6); }
.wf-node.wait { border-color: rgba(120, 180, 255, 0.5); }
.wf-node-head {
  padding: 5px 10px;
  font-weight: 700;
  font-size: 11px;
  border-bottom: 1px solid var(--glass-border);
  color: var(--accent-bright);
}
.wf-node-body {
  padding: 6px 10px;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 90px;
  overflow: hidden;
}
.h-label {
  position: absolute;
  right: -18px;
  font-size: 10px;
  color: var(--text-low);
}
.h-label.t { top: 22%; }
.h-label.f { top: 62%; }
</style>