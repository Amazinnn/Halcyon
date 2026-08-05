<script setup lang="ts">
import type { HeatmapDay } from "../lib/fakeStats";

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
  grid-template-columns: repeat(15, 14px);
  gap: 4px;
  padding: 8px 0;
}
.cell {
  width: 14px;
  height: 14px;
  border-radius: 3px;
  background: #eef1f7;
}
.cell.lvl-0 {
  background: #eef1f7;
}
.cell.lvl-1 {
  background: #c6d6ff;
}
.cell.lvl-2 {
  background: #8fb0ff;
}
.cell.lvl-3 {
  background: #4f7cff;
}
.cell.lvl-4 {
  background: #2b52c9;
}
</style>