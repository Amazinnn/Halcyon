<script setup lang="ts">
import { computed } from "vue";
import { Handle, Position } from "@vue-flow/core";
import { NODE_DESC, NODE_LABELS } from "../../lib/workflow";

const props = defineProps<{
  id: string;
  data: { kind: string; params: Record<string, unknown>; status?: string };
}>();

const kind = computed(() => props.data.kind);
const p = computed(() => props.data.params ?? {});
const status = computed(() => props.data.status ?? "");

const summary = computed(() => {
  switch (kind.value) {
    case "agent":
      return (
        (p.value.wait === false ? "[不等待] " : "[等待结果] ") +
        String(p.value.prompt ?? "") +
        (p.value.wait === false ? "" : "（回复显示在对话框与宠物）")
      );
    case "show_window":
      return "打开 " + String(p.value.target ?? "chat");
    case "wait":
      return "等待 " + String(p.value.seconds ?? 30) + " 秒";
    case "branch": {
      const condition = p.value.condition === "focus_state" ? "专注状态" : "上游 Agent 槽值";
      const opts = Array.isArray(p.value.options) ? (p.value.options as unknown[]) : [];
      const detail =
        p.value.condition === "focus_state"
          ? "当状态为「" + String(p.value.focusState ?? "focus") + "」"
          : opts.map((_, i) => `选项${i + 1}`).join(" / ");
      return condition + " → " + detail;
    }
    case "focus":
      return "专注 " + String(p.value.seconds ?? 1500) + " 秒";
    case "idle":
      return "空闲 " + String(p.value.seconds ?? 300) + " 秒";
    case "ring":
      return "响铃 " + String(p.value.seconds ?? 3) + " 秒";
    default:
      return "";
  }
});

const branchOptions = computed(() => {
  if (kind.value !== "branch" || p.value.condition === "focus_state") return [];
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
    <template v-if="kind === 'branch' && p.condition !== 'focus_state'">
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
    <template v-else-if="kind === 'branch'">
      <Handle type="source" :position="Position.Right" id="true" :style="{ top: '32%' }" />
      <Handle type="source" :position="Position.Right" id="false" :style="{ top: '72%' }" />
      <span class="h-label t">符合</span>
      <span class="h-label f">不符合</span>
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