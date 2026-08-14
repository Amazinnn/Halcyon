<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useAgentStore } from "../../stores/agent";
import type { FocusState } from "../../stores/ui";

/**
 * Always-on-top status capsule (v1.4.1): a tiny transparent window that floats
 * above every app (including fullscreen windows). Purely informational - the
 * window is click-through (set_ignore_cursor_events in Rust) so it never
 * blocks clicks on apps underneath. Mirrors the live focus timer via the
 * `focus:tick` event and the supervision state via `supervision:status`.
 */
const agent = useAgentStore();

const state = ref<FocusState>("idle");
const focusRemaining = ref(0);
const restRemaining = ref(0);
const paused = ref(false);
const phaseDone = ref(false);
const supStatus = ref<"ok" | "drift">("ok");
const acrylicEnabled = ref(true);
const glassOpacity = ref(22);

function applyGlassOpacity(opacity: number) {
  glassOpacity.value = opacity;
  // Requirement #123: one global glass opacity for every window's glass
  // layer. The pill's historical alpha (0.84) is scaled by opacity/22.
  const factor = opacity / 22;
  const alpha = Math.min(1, Math.max(0.04, 0.84 * factor));
  document.documentElement.style.setProperty("--glass-opacity", String(alpha));
}

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
  supStatus.value === "drift" ? "走神中" : "",
);

onMounted(async () => {
  void agent.init();
  const bootstrap = await invoke<{ acrylicEnabled?: boolean; acrylicOpacity?: number }>("get_bootstrap");
  acrylicEnabled.value = bootstrap.acrylicEnabled ?? true;
  applyGlassOpacity(bootstrap.acrylicOpacity ?? 22);
  await listen<{ enabled: boolean; opacity?: number }>("settings:acrylic-changed", (e) => {
    acrylicEnabled.value = e.payload.enabled;
    if (typeof e.payload.opacity === "number") applyGlassOpacity(e.payload.opacity);
  });
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
    if (st === "drift" || st === "ok") supStatus.value = st;
  });
});
</script>

<template>
  <div class="capsule" :class="{ 'glass-disabled': !acrylicEnabled }">
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
  /* Requirement #121: the transparent host is larger than the pill so the
     WebView-owned shadow renders fully; these insets keep the pill at the
     same visible geometry (500x44) inside the 540x84 window. */
  position: absolute;
  left: 20px;
  right: 20px;
  top: 14px;
  bottom: 26px;
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 18px;
  border-radius: var(--r-pill);
  background:
    linear-gradient(135deg, rgba(255,255,255,0.13), rgba(255,255,255,0.035)),
    rgb(12 24 17 / var(--glass-opacity, 0.84));
  border: 1px solid var(--glass-border);
  box-shadow: inset 0 1px 0 rgba(255,255,255,0.12), 0 6px 18px rgba(0, 0, 0, 0.3);
  font-size: 12px;
  color: var(--text-hi);
  overflow: hidden;
}
.capsule.glass-disabled { background: rgba(12, 24, 17, 0.96); box-shadow: inset 0 1px 0 rgba(255,255,255,0.08), 0 4px 12px rgba(0,0,0,.24); }
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
</style>
