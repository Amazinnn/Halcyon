<script setup lang="ts">
import type { HeatmapDay } from "../lib/dashboard";

defineProps<{ days: HeatmapDay[] }>();

function level(minutes: number): number {
  if (minutes <= 0) return 0;
  if (minutes < 30) return 1;
  if (minutes < 90) return 2;
  if (minutes < 180) return 3;
  return 4;
}
</script>

<template>
  <div class="heatmap">
    <div
      v-for="(d, i) in days"
      :key="i"
      class="cell"
      :class="`lvl-${level(d.minutes)}`"
      :title="`${d.date}: ${d.minutes} 分钟`"
    ></div>
  </div>
</template>

<style scoped>
.heatmap {
  display: grid;
  grid-template-columns: repeat(15, 1fr);
  gap: 3px;
  padding: 4px 0;
}
.cell { aspect-ratio: 1; border-radius: 2px; background: rgba(163, 230, 53, 0.08); }
.cell.lvl-1 { background: rgba(163, 230, 53, 0.28); }
.cell.lvl-2 { background: rgba(163, 230, 53, 0.5); }
.cell.lvl-3 { background: rgba(163, 230, 53, 0.72); }
.cell.lvl-4 { background: #a3e635; }
</style>