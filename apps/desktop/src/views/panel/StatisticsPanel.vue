<script setup lang="ts">
import { onMounted } from "vue";
import Chart from "chart.js/auto";
import Heatmap from "../../components/Heatmap.vue";
import { genHeatmap30, gen24h, genGenre } from "../../lib/fakeStats";

const heatmap = genHeatmap30(42, 30);
const hours = gen24h(7);
const genres = genGenre(11);

onMounted(() => {
  const tc = document.getElementById("time-chart") as HTMLCanvasElement | null;
  if (tc) {
    new Chart(tc, {
      type: "bar",
      data: {
        labels: Array.from({ length: 24 }, (_, h) => `${h}时`),
        datasets: [
          { label: "专注分钟", data: hours, backgroundColor: "#4f7cff", borderRadius: 3 },
        ],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        scales: { y: { beginAtZero: true, max: 60 } },
        plugins: { legend: { display: false } },
      },
    });
  }
  const gc = document.getElementById("genre-chart") as HTMLCanvasElement | null;
  if (gc) {
    new Chart(gc, {
      type: "doughnut",
      data: {
        labels: genres.map((g) => g.genre),
        datasets: [
          {
            data: genres.map((g) => g.minutes),
            backgroundColor: ["#4f7cff", "#22c1a4", "#f5a623", "#9b59b6", "#e74c3c"],
          },
        ],
      },
      options: { responsive: true, maintainAspectRatio: false },
    });
  }
});
</script>

<template>
  <div class="stats-panel">
    <h3>本月专注热力图（假数据）</h3>
    <Heatmap :days="heatmap" />
    <h3>今日 24 小时分布（假数据）</h3>
    <div class="chart-box"><canvas id="time-chart"></canvas></div>
    <h3>音乐类型分布（假数据）</h3>
    <div class="chart-box"><canvas id="genre-chart"></canvas></div>
    <p class="summary">专注 3h42m · 分心 1h08m · 空闲 2h10m（Spike 占位）</p>
  </div>
</template>

<style scoped>
.stats-panel {
  padding: 12px 16px;
  overflow-y: auto;
  height: 100%;
}
h3 {
  font-size: 13px;
  margin: 14px 0 6px;
  color: #333c56;
}
.chart-box {
  position: relative;
  height: 130px;
}
.summary {
  font-size: 12px;
  color: #6b7596;
  margin-top: 16px;
}
</style>