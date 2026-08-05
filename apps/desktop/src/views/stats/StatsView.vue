<script setup lang="ts">
import { onMounted } from "vue";
import Chart from "chart.js/auto";
import Heatmap from "../../components/Heatmap.vue";
import { genHeatmap30, gen24h, genGenre } from "../../lib/fakeStats";
import WindowHeader from "../../components/WindowHeader.vue";

const heatmap = genHeatmap30(42, 30);
const hours = gen24h(7);
const genres = genGenre(11);
const GREEN = ["#a3e635", "#4ade80", "#22c55e", "#16a34a", "#15803d"];

onMounted(() => {
  const tc = document.getElementById("time-chart") as HTMLCanvasElement | null;
  if (tc) {
    new Chart(tc, {
      type: "bar",
      data: {
        labels: Array.from({ length: 24 }, (_, h) => `${h}时`),
        datasets: [{ label: "专注分钟", data: hours, backgroundColor: "#a3e635", borderRadius: 3 }],
      },
      options: { responsive: true, maintainAspectRatio: false, scales: { y: { beginAtZero: true, max: 60 } }, plugins: { legend: { display: false } } },
    });
  }
  const gc = document.getElementById("genre-chart") as HTMLCanvasElement | null;
  if (gc) {
    new Chart(gc, {
      type: "doughnut",
      data: { labels: genres.map((g) => g.genre), datasets: [{ data: genres.map((g) => g.minutes), backgroundColor: GREEN }] },
      options: { responsive: true, maintainAspectRatio: false, plugins: { legend: { display: false } } },
    });
  }
});
</script>

<template>
  <div class="stats-window">
    <WindowHeader title="统计" collapsible />
    <div class="bento">
      <div class="card glass heatmap" style="grid-column: span 2; grid-row: span 2">
        <h4>本月专注热力图</h4>
        <Heatmap :days="heatmap" />
      </div>
      <div class="card glass h24" style="grid-column: span 2">
        <h4>今日 24 小时分布</h4>
        <div class="chart-box"><canvas id="time-chart"></canvas></div>
      </div>
      <div class="card glass genre">
        <h4>音乐类型</h4>
        <div class="chart-box"><canvas id="genre-chart"></canvas></div>
      </div>
      <div class="card glass summary">
        <h4>汇总</h4>
        <p class="num">专注 3h42m</p>
        <p class="num">分心 1h08m</p>
        <p class="num">空闲 2h10m</p>
      </div>
      <div class="card glass streak" style="grid-column: span 2">
        <h4>连续专注天数</h4>
        <p class="big num">12 天</p>
      </div>
      <div class="card glass today" style="grid-column: span 2">
        <h4>今日专注</h4>
        <p class="big num">2h10m</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.stats-window {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: transparent;
  border: 1px solid var(--glass-border);
  border-radius: var(--r-lg);
  overflow: hidden;
  box-sizing: border-box;
}
.bento {
  flex: 1;
  overflow-y: auto;
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  grid-auto-rows: minmax(86px, auto);
  grid-auto-flow: dense;
  gap: 8px;
  padding: 10px;
}
.card { padding: 8px 10px; min-width: 0; }
.card h4 { margin: 0 0 6px; font-size: 11px; color: var(--text-mid); font-weight: 600; }
.chart-box { position: relative; height: 70px; }
.big { font-size: 20px; margin: 2px 0; color: var(--accent-bright); }
.summary p { margin: 2px 0; font-size: 12px; color: var(--text-mid); }
</style>