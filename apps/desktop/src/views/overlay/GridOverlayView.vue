<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import type { GridRect } from "../../lib/grid";
import { cellStyle } from "../../lib/grid";

const visible = ref(false);
const occupied = ref<GridRect[]>([]);
const target = ref<{ rect: GridRect; conflict: boolean } | null>(null);
// Actual floating rect of the dragged window, in continuous grid units.
// This is the brightness center: the glow follows the real position during
// the drag, NOT the snapped placement cell (so it never jumps between cells).
const floatRect = ref<{ x: number; y: number; w: number; h: number } | null>(null);

onMounted(() => {
  void listen("grid:preview", (e) => {
    const p = e.payload as {
      visible: boolean;
      rect?: GridRect;
      floatRect?: { x: number; y: number; w: number; h: number };
      occupiedCells?: GridRect[];
      conflict?: boolean;
    };
    visible.value = !!p.visible;
    occupied.value = p.occupiedCells ?? [];
    target.value = p.rect ? { rect: p.rect, conflict: !!p.conflict } : null;
    floatRect.value = p.floatRect ?? null;
  });
});

const GRID_COLS = 12;
const GRID_ROWS = 8;

// Brightness fades linearly with Chebyshev distance from the box and is fully
// off beyond 2 cells ("only the surrounding two cells are lit; farther lines
// are invisible"). 0 = touching the box -> 1.0, 1 cell out -> 0.5, 2 cells
// out -> 0.
function cellBrightness(f: { x: number; y: number; w: number; h: number }, c: number, r: number): number {
  const dx = Math.max(0, f.x - (c + 1), c - (f.x + f.w));
  const dy = Math.max(0, f.y - (r + 1), r - (f.y + f.h));
  const d = Math.max(dx, dy);
  return Math.max(0, 1 - d / 2);
}

// One grid segment per cell (96). Each segment paints only its right + bottom
// edge (plus the outer frame edges on the first row/column), so every line is
// drawn exactly once with a single uniform color - no more double-drawn
// "short lines" with two different brightnesses on the same edge.
const cells = computed(() => {
  const f = floatRect.value;
  const out: number[] = [];
  for (let r = 0; r < GRID_ROWS; r++) {
    for (let c = 0; c < GRID_COLS; c++) {
      out.push(f ? cellBrightness(f, c, r) : 0.07);
    }
  }
  return out;
});

function lineStyle(opacity: number) {
  return { opacity };
}
</script>

<template>
  <div v-if="visible" class="overlay">
    <div class="grid-lines">
      <div v-for="(o, i) in cells" :key="i" class="line" :style="lineStyle(o)"></div>
    </div>
    <div class="grid-marks">
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
  overflow: hidden;
}
.grid-lines,
.grid-marks {
  position: absolute;
  inset: 0;
  display: grid;
  grid-template-columns: repeat(12, 1fr);
  grid-template-rows: repeat(8, 1fr);
}
/* Each cell paints only its right+bottom edge (single-draw grid lines);
   the first row/column supply the outer frame edges. */
.line {
  border: 0;
  border-right: 1px solid var(--accent);
  border-bottom: 1px solid var(--accent);
  transition: opacity 40ms linear;
}
.line:nth-child(12n + 1) {
  border-left: 1px solid var(--accent);
}
.line:nth-child(-n + 12) {
  border-top: 1px solid var(--accent);
}
.occ { background: rgba(248, 113, 113, 0.16); border: 1px solid rgba(248, 113, 113, 0.45); }
.tgt { background: rgba(163, 230, 53, 0.2); border: 2px solid var(--accent); }
.tgt.conflict { background: rgba(248, 113, 113, 0.28); border-color: var(--err); }
</style>
