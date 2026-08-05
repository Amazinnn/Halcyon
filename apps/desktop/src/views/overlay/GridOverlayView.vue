<script setup lang="ts">
import { onMounted, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import type { GridRect } from "../../lib/grid";
import { cellStyle } from "../../lib/grid";

const visible = ref(false);
const occupied = ref<GridRect[]>([]);
const target = ref<{ rect: GridRect; conflict: boolean } | null>(null);

onMounted(() => {
  void listen("grid:preview", (e) => {
    const p = e.payload as {
      visible: boolean;
      rect?: GridRect;
      occupiedCells?: GridRect[];
      conflict?: boolean;
    };
    visible.value = !!p.visible;
    occupied.value = p.occupiedCells ?? [];
    target.value = p.rect ? { rect: p.rect, conflict: !!p.conflict } : null;
  });
});
</script>

<template>
  <div v-if="visible" class="overlay">
    <div class="cells">
      <div v-for="i in 96" :key="i" class="line"></div>
      <div v-for="(r, i) in occupied" :key="'o' + i" class="occ" :style="cellStyle(r)"></div>
      <div v-if="target" class="tgt" :class="{ conflict: target.conflict }" :style="cellStyle(target.rect)"></div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  pointer-events: none;
}
.cells {
  display: grid;
  grid-template-columns: repeat(12, 1fr);
  grid-template-rows: repeat(8, 1fr);
  width: 100%;
  height: 100%;
}
.line { border: 1px solid rgba(163, 230, 53, 0.07); }
.occ { background: rgba(248, 113, 113, 0.16); border: 1px solid rgba(248, 113, 113, 0.45); }
.tgt { background: rgba(163, 230, 53, 0.2); border: 2px solid var(--accent); }
.tgt.conflict { background: rgba(248, 113, 113, 0.28); border-color: var(--err); }
</style>