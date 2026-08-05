<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { useSettingsStore } from "../../stores/settings";
import { useAgentStore } from "../../stores/agent";
import type { FocusState } from "../../stores/ui";

/**
 * Always-on-top status capsule (v1.4.1): a tiny transparent window that floats
 * above every app (including fullscreen windows). Purely informational - the
 * window is click-through (set_ignore_cursor_events in Rust) so it never
 * blocks clicks on apps underneath. Mirrors the live focus timer via the
 * `focus:tick` event and the supervision state via `supervision:status`.
 */
const settings = useSettingsStore();
const agent = useAgentStore();

const state = ref<FocusState>("idle");
const focusRemaining = ref(0);
const restRemaining = ref(0);
const paused = ref(false);
const phaseDone = ref(false);
const supStatus = ref<"ok" | "drift" | "paused">("ok");

function fmt(totalSec: number) {
  const s = Math.max(0, Math.floor(totalSec));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  return h > 0
    ? `${h}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`
    : `${m}:${String(sec).padStart(2, "0")}`;
}

const modeText = computed(() => {
  if (state.value === "focus") {
    const suffix = paused.value ? " · 已暂停" : "";
    return `专注中 · ${fmt(focusRemaining.value)}${suffix}`;
  }
  if (state.value === "rest") {
    return phaseDone.value ? "休息结束" : `休息中 · ${fmt(restRemaining.value)}`;
  }
  return "未开始";
});
const supText = computed(() =>
  supStatus.value === "drift" ? "走神中" : supStatus.value === "paused" ? "监督暂停" : "",
);

onMounted(async () => {
  await settings.load();
  void agent.init();
  await listen<{
    state: FocusState;
    focusRemainingSec: number;
    restRemainingSec: number;
    paused: boolean;
    phaseDone: boolean;
  }>("focus:tick", (e) => {
    state.value = e.payload.state;
    focusRemaining.value = e.payload.focusRemainingSec;
    restRemaining.value = e.payload.restRemainingSec;
    paused.value = e.payload.paused;
    phaseDone.value = e.payload.phaseDone;
  });
  await listen<{ status: string }>("supervision:status", (e) => {
    const st = e.payload.status;
    if (st === "drift" || st === "paused" || st === "ok") supStatus.value = st;
  });
});
</script>

<template>
  <div class="capsule">
    <span class="task" :title="settings.currentTask?.name ?? ''">
      {{ settings.currentTask?.name ?? "未设置任务" }}
    </span>
    <span class="agent-status">
      Agent
      <span class="dot" :class="`st-${agent.state}`"></span>
    </span>
    <span class="mode-chip num" :class="state">{{ modeText }}</span>
    <span v-if="supText" class="sup-status" :class="supStatus">{{ supText }}</span>
  </div>
</template>

<style scoped>
.capsule {
  height: 100%;
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 18px;
  border-radius: var(--r-pill);
  background: var(--glass);
  border: 1px solid var(--glass-border);
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.3);
  font-size: 12px;
  color: var(--text-hi);
  overflow: hidden;
}
.task {
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 160px;
}
.agent-status { display: flex; align-items: center; gap: 6px; color: var(--text-mid); white-space: nowrap; }
.dot { width: 8px; height: 8px; border-radius: 50%; background: var(--text-low); flex-shrink: 0; }
.dot.st-thinking, .dot.st-reading, .dot.st-searching { background: var(--accent); }
.dot.st-editing, .dot.st-running, .dot.st-testing { background: var(--accent-bright); }
.dot.st-waiting_permission { background: var(--warn); }
.dot.st-success { background: var(--accent); }
.dot.st-error { background: var(--err); }
.mode-chip {
  color: var(--text-mid);
  border: 1px solid var(--glass-border);
  border-radius: var(--r-pill);
  padding: 2px 10px;
  white-space: nowrap;
}
.mode-chip.focus { color: var(--accent-bright); border-color: rgba(163, 230, 53, 0.4); }
.mode-chip.rest { color: var(--warn); border-color: rgba(251, 191, 36, 0.4); }
.sup-status {
  border-radius: var(--r-pill);
  padding: 2px 10px;
  border: 1px solid;
  white-space: nowrap;
}
.sup-status.drift { color: var(--warn); border-color: rgba(251, 191, 36, 0.5); }
.sup-status.paused { color: var(--text-mid); border-color: var(--glass-border); }
</style>
