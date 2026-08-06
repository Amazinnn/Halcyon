<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
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

// 12x8 grid lines: brightness fades with distance from the dragged box
// (near = bright lime ~1.0, far = dim ~0.06); requirement: brighter closer to the box being moved
const GRID_COLS = 12;
const GRID_ROWS = 8;

/** Linear falloff: distance d (cells) from the dragged box center -> 1.0 at d=0, ~0.06 at d>=8. */
function lineOpacity(d: number): number {
  return 0.06 + 0.94 * Math.max(0, 1 - d / 8);
}

const cells = computed(() => {
  const t = target.value;
  const out: number[] = [];
  if (!t) {
    for (let i = 0; i < GRID_COLS * GRID_ROWS; i++) out.push(0.07);
    return out;
  }
  const cc = t.rect.col + (t.rect.cols - 1) / 2;
  const cr = t.rect.row + (t.rect.rows - 1) / 2;
  for (let r = 0; r < GRID_ROWS; r++) {
    for (let c = 0; c < GRID_COLS; c++) {
      out.push(lineOpacity(Math.hypot(c - cc, r - cr)));
    }
  }
  return out;
});

function lineStyle(opacity: number) {
  return { borderColor: `rgba(163,230,53,${opacity})` };
}
</script>

<template>
  <div v-if="visible" class="overlay">
    <div class="cells">
      <div
        v-for="(cell, i) in cells"
        :key="i"
        class="line"
        :style="lineStyle(cell)"
      ></div>
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
