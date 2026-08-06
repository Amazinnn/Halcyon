<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import type { GridRect } from "../../lib/grid";
import { cellStyle } from "../../lib/grid";

const visible = ref(false);
const occupied = ref<GridRect[]>([]);
const target = ref<{ rect: GridRect; conflict: boolean } | null>(null);
// Actual floating rect of the dragged window, in continuous grid units.
// Brightness at every point of a grid line is computed from the distance to
// this rect, so the glow follows the real position (never the snapped cell).
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
const GRID_FALL_OFF = 1.5; // bright -> fully transparent over this many cells
const GRID_STOP_STEP = 0.25; // gradient stop resolution along each line (cells)

// Brightness of a point (px, py) in grid units: linear falloff with the
// Chebyshev edge distance to the floating rect; beyond GRID_FALL_OFF -> 0.
function pointBrightness(
  f: { x: number; y: number; w: number; h: number },
  px: number,
  py: number,
): number {
  const dx = Math.max(0, f.x - px, px - (f.x + f.w));
  const dy = Math.max(0, f.y - py, py - (f.y + f.h));
  return Math.max(0, 1 - Math.max(dx, dy) / GRID_FALL_OFF);
}

function alphaStop(alpha: number): string {
  return `rgba(163,230,53,${alpha.toFixed(3)})`;
}

// Vertical line at grid x=i: brightness varies along y (0..GRID_ROWS).
function vGradient(f: { x: number; y: number; w: number; h: number }, i: number): string {
  const stops: string[] = [];
  for (let y = 0; y <= GRID_ROWS; y += GRID_STOP_STEP) {
    const a = pointBrightness(f, i, y);
    stops.push(`${alphaStop(a)} ${((y / GRID_ROWS) * 100).toFixed(2)}%`);
  }
  return `linear-gradient(to bottom, ${stops.join(", ")})`;
}

// Horizontal line at grid y=j: brightness varies along x (0..GRID_COLS).
function hGradient(f: { x: number; y: number; w: number; h: number }, j: number): string {
  const stops: string[] = [];
  for (let x = 0; x <= GRID_COLS; x += GRID_STOP_STEP) {
    const a = pointBrightness(f, x, j);
    stops.push(`${alphaStop(a)} ${((x / GRID_COLS) * 100).toFixed(2)}%`);
  }
  return `linear-gradient(to right, ${stops.join(", ")})`;
}

// 13 vertical + 9 horizontal full-length lines. Each line is ONE element with
// a gradient along its own length, so brightness changes continuously with
// the distance from every point to the dragged box - no per-cell steps.
const vLines = computed(() => {
  const f = floatRect.value;
  const out: { pos: string; grad: string }[] = [];
  for (let i = 0; i <= GRID_COLS; i++) {
    out.push({ pos: `${((i / GRID_COLS) * 100).toFixed(4)}%`, grad: f ? vGradient(f, i) : "transparent" });
  }
  return out;
});
const hLines = computed(() => {
  const f = floatRect.value;
  const out: { pos: string; grad: string }[] = [];
  for (let j = 0; j <= GRID_ROWS; j++) {
    out.push({ pos: `${((j / GRID_ROWS) * 100).toFixed(4)}%`, grad: f ? hGradient(f, j) : "transparent" });
  }
  return out;
});
</script>

<template>
  <div v-if="visible" class="overlay">
    <div class="grid-lines">
      <div
        v-for="(l, i) in vLines"
        :key="'v' + i"
        class="vline"
        :style="{ left: l.pos, background: l.grad }"
      ></div>
      <div
        v-for="(l, j) in hLines"
        :key="'h' + j"
        class="hline"
        :style="{ top: l.pos, background: l.grad }"
      ></div>
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
.grid-lines {
  position: absolute;
  inset: 0;
}
.vline {
  position: absolute;
  top: 0;
  bottom: 0;
  width: 1px;
}
.hline {
  position: absolute;
  left: 0;
  right: 0;
  height: 1px;
}
.grid-marks {
  position: absolute;
  inset: 0;
  display: grid;
  grid-template-columns: repeat(12, 1fr);
  grid-template-rows: repeat(8, 1fr);
}
.occ { background: rgba(248, 113, 113, 0.16); border: 1px solid rgba(248, 113, 113, 0.45); }
.tgt { background: rgba(163, 230, 53, 0.2); border: 2px solid var(--accent); }
.tgt.conflict { background: rgba(248, 113, 113, 0.28); border-color: var(--err); }
</style>
