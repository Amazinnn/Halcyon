<script setup lang="ts">
import { onMounted, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import AppIcon from "../../components/AppIcon.vue";
import { useGridDrag } from "../../composables/useGridDrag";

const collapsed = ref<string[]>([]);
const { onPointerDown, onPointerMove, onPointerUp } = useGridDrag("logos");

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
  background: transparent;
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