<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import Chart from "chart.js/auto";
import Heatmap from "../../components/Heatmap.vue";
import WindowHeader from "../../components/WindowHeader.vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { fmtDuration, type DashboardPayload } from "../../lib/dashboard";

const dashboard = ref<DashboardPayload | null>(null);
let timeChart: Chart | null = null;
let pollTimer: number | undefined;
let unlistenStats: UnlistenFn | undefined;
let unlistenFocus: UnlistenFn | undefined;

function renderTimeChart() {
  const d = dashboard.value;
  const tc = document.getElementById("time-chart") as HTMLCanvasElement | null;
  if (!d || !tc) return;
  if (timeChart) {
    timeChart.data.datasets[0].data = d.hours24;
    timeChart.update();
    return;
  }
  timeChart = new Chart(tc, {
    type: "bar",
    data: {
      labels: Array.from({ length: 24 }, (_, h) => `${h}时`),
      datasets: [{ label: "专注分钟", data: d.hours24, backgroundColor: "#a3e635", borderRadius: 3 }],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      scales: { y: { beginAtZero: true, max: 60 } },
      plugins: { legend: { display: false } },
    },
  });
}

async function refresh() {
  try {
    dashboard.value = await invoke<DashboardPayload>("stats_dashboard");
    renderTimeChart();
  } catch (e) {
    console.error("stats_dashboard failed", e);
  }
}

onMounted(async () => {
  await refresh();
  pollTimer = window.setInterval(() => void refresh(), 30_000);
  unlistenStats = await listen("stats:changed", () => void refresh());
  const win = getCurrentWebviewWindow();
  unlistenFocus = await win.onFocusChanged(({ payload: focused }) => {
    if (focused) void refresh();
  });
});

onUnmounted(() => {
  if (pollTimer !== undefined) window.clearInterval(pollTimer);
  unlistenStats?.();
  unlistenFocus?.();
  timeChart?.destroy();
  timeChart = null;
});
</script>

<template>
  <div class="stats-window">
    <WindowHeader title="统计" collapsible />
    <div class="bento">
      <div class="card glass heatmap" style="grid-column: span 2; grid-row: span 2">
        <h4>本月专注热力图</h4>
        <Heatmap v-if="dashboard" :days="dashboard.heatmap30" />
      </div>
      <div class="card glass h24" style="grid-column: span 2">
        <h4>今日 24 小时分布</h4>
        <div class="chart-box"><canvas id="time-chart"></canvas></div>
      </div>
      <div class="card glass genre">
        <h4>音乐类型</h4>
        <p class="na">暂无数据</p>
      </div>
      <div class="card glass summary">
        <h4>汇总</h4>
        <p class="num">专注 {{ dashboard ? fmtDuration(dashboard.today.totalSec) : "—" }}</p>
        <p class="na">分心 暂无数据</p>
        <p class="na">空闲 暂无数据</p>
      </div>
      <div class="card glass streak" style="grid-column: span 2">
        <h4>连续专注天数</h4>
        <p class="big num">{{ dashboard ? `${dashboard.streakDays} 天` : "—" }}</p>
      </div>
      <div class="card glass today" style="grid-column: span 2">
        <h4>今日专注</h4>
        <p class="big num">{{ dashboard ? fmtDuration(dashboard.today.totalSec) : "—" }}</p>
        <p v-if="dashboard" class="sub">{{ dashboard.today.rounds }} 轮</p>
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
.summary p { margin: 2px 0; font-size: 12px; }
.num { color: var(--text-hi); }
.na { color: var(--text-mid); }
.sub { font-size: 11px; color: var(--text-mid); }
</style>