<script setup lang="ts">
import { onMounted, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import AppIcon from "../../components/AppIcon.vue";
import type { GridMetrics } from "../../lib/grid";

const collapsed = ref<string[]>([]);
const win = getCurrentWebviewWindow();

let dragging = false;
let scale = 1;
let startScreen = { x: 0, y: 0 };
let startPos = { x: 0, y: 0 };

const meta: Record<string, { label: string; icon: string }> = {
  chat: { label: "对话", icon: "chat" },
  stats: { label: "统计", icon: "stats" },
  music: { label: "音乐", icon: "music" },
};

onMounted(() => {
  void listen("logos:update", (e) => {
    collapsed.value = (e.payload as { collapsed: string[] }).collapsed ?? [];
  });
});

async function onPointerDown(e: PointerEvent) {
  if ((e.target as HTMLElement).closest("button")) return;
  dragging = true;
  startScreen = { x: e.screenX, y: e.screenY };
  scale = await win.scaleFactor();
  const p = await win.outerPosition();
  startPos = { x: p.x, y: p.y };
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
}

async function onPointerMove(e: PointerEvent) {
  if (!dragging) return;
  const dx = (e.screenX - startScreen.x) * scale;
  const dy = (e.screenY - startScreen.y) * scale;
  await win.setPosition(new PhysicalPosition(startPos.x + dx, startPos.y + dy));
}

async function onPointerUp() {
  if (!dragging) return;
  dragging = false;
  const p = await win.outerPosition();
  const s = await win.outerSize();
  scale = await win.scaleFactor();
  const cx = (p.x + s.width / 2) / scale;
  const cy = (p.y + s.height / 2) / scale;
  const m = await invoke<GridMetrics>("get_grid_metrics");
  const d = { top: cy, bottom: m.screenH - cy, left: cx, right: m.screenW - cx };
  const edge = Object.keys(d).reduce((a, b) => (d[a as keyof typeof d] < d[b as keyof typeof d] ? a : b));
  await invoke("dock_logos", { edge });
}

function restore(label: string) {
  void invoke("restore", { label });
}
</script>

<template>
  <div class="logos" @pointerdown="onPointerDown" @pointermove="onPointerMove" @pointerup="onPointerUp">
    <button v-for="l in collapsed" :key="l" class="capsule" @click="restore(l)">
      <AppIcon :name="meta[l]?.icon ?? 'leaf'" />
      <span class="lbl">{{ meta[l]?.label ?? l }}</span>
    </button>
    <div v-if="collapsed.length === 0" class="empty">—</div>
  </div>
</template>

<style scoped>
.logos {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px;
  background: rgba(10, 16, 13, 0.6);
  border: 1px solid var(--glass-border);
  border-radius: var(--r-md);
  box-sizing: border-box;
  cursor: grab;
}
.capsule {
  border: 1px solid var(--glass-border);
  background: var(--glass);
  color: var(--text-hi);
  border-radius: var(--r-pill);
  padding: 5px 12px;
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  font-size: 12px;
}
.capsule:hover { border-color: var(--accent); color: var(--accent-bright); }
.lbl { font-weight: 600; }
.empty { color: var(--text-low); text-align: center; font-size: 11px; padding: 2px; }
</style>