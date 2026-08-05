<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useGridDrag } from "../composables/useGridDrag";
import AppIcon from "./AppIcon.vue";

defineProps<{ title: string; collapsible?: boolean }>();

const win = getCurrentWebviewWindow();
const label = win.label;
const { onPointerDown, onPointerMove, onPointerUp } = useGridDrag(label);
const pinned = ref(true);

async function togglePin() {
  pinned.value = !pinned.value;
  await invoke("set_topmost", { label, topmost: pinned.value });
}

async function collapseWin() {
  await invoke("collapse", { label });
}
</script>

<template>
  <div class="win-header" @pointerdown="onPointerDown" @pointermove="onPointerMove" @pointerup="onPointerUp">
    <span class="title">{{ title }}</span>
    <div class="actions" @pointerdown.stop>
      <button class="ghost" :title="pinned ? '取消置顶' : '置顶'" @click="togglePin">
        <AppIcon :name="pinned ? 'pin' : 'pin-off'" />
      </button>
      <button v-if="collapsible" class="ghost" title="折叠为 logo" @click="collapseWin">
        <AppIcon name="collapse" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.win-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 10px;
  cursor: grab;
  background: rgba(10, 16, 13, 0.4);
  border-bottom: 1px solid var(--glass-border);
  flex-shrink: 0;
}
.title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-mid);
}
.actions { display: flex; gap: 4px; }
.ghost {
  border: none;
  background: transparent;
  color: var(--text-mid);
  border-radius: var(--r-sm);
  padding: 3px;
  cursor: pointer;
  display: inline-flex;
}
.ghost:hover { color: var(--accent); background: var(--accent-wash); }
</style>