<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useGridDrag } from "../../composables/useGridDrag";
import AppIcon from "../AppIcon.vue";
import FocusButton from "./FocusButton.vue";

defineProps<{ title: string; collapsible?: boolean }>();

const win = getCurrentWebviewWindow();
const label = win.label;
const { onPointerDown, onPointerMove, onPointerUp } = useGridDrag(label);
const pinned = ref(true);

// v1.10 (#31): 150ms lockout on header buttons so rapid clicking cannot flood
// the window manager with redundant toggles.
let lastAction = 0;
function throttled(): boolean {
  const now = Date.now();
  if (now - lastAction < 150) return true;
  lastAction = now;
  return false;
}

async function togglePin() {
  if (throttled()) return;
  pinned.value = !pinned.value;
  await invoke("set_topmost", { label, topmost: pinned.value });
}

async function collapseWin() {
  if (throttled()) return;
  await invoke("collapse", { label });
}
</script>

<template>
  <div class="win-header" @pointerdown="onPointerDown" @pointermove="onPointerMove" @pointerup="onPointerUp">
    <span class="title">{{ title }}</span>
    <div class="actions" @pointerdown.stop>
      <FocusButton variant="ghost" size="icon" :title="pinned ? '取消置顶' : '置顶'" @click="togglePin">
        <AppIcon :name="pinned ? 'pin' : 'pin-off'" />
      </FocusButton>
      <FocusButton v-if="collapsible" variant="ghost" size="icon" title="折叠为 logo" @click="collapseWin">
        <AppIcon name="collapse" />
      </FocusButton>
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
  background: transparent;
  border-bottom: 1px solid var(--glass-border);
  flex-shrink: 0;
}
.title {
  font-size: var(--fs-md);
  font-weight: 600;
  color: var(--text-mid);
}
.actions { display: flex; gap: 4px; }
</style>
