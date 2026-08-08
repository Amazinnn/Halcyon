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
  // v1.10.2 (#41): smooth line, only "0时"/"24时" ticks, precise nearest hover.
  timeChart = new Chart(tc, {
    type: "line",
    data: {
      labels: Array.from({ length: 24 }, (_, h) => `${h}时`),
      datasets: [
        {
          label: "专注分钟",
          data: d.hours24,
          borderColor: "#a3e635",
          backgroundColor: "rgba(163, 230, 53, 0.14)",
          fill: true,
          tension: 0.4,
          borderWidth: 2,
          pointRadius: 2,
          pointHoverRadius: 5,
          pointBackgroundColor: "#a3e635",
        },
      ],
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      scales: {
        x: {
          grid: { display: false },
          ticks: {
            color: "#5f7363",
            maxRotation: 0,
            callback: (_v: string | number, index: number) =>
              index === 0 ? "0时" : index === 23 ? "24时" : "",
          },
        },
        y: { beginAtZero: true, max: 60, ticks: { color: "#5f7363" } },
      },
      interaction: { mode: "nearest", intersect: false },
      plugins: {
        legend: { display: false },
        tooltip: {
          callbacks: { label: (ctx) => `${ctx.label} · ${ctx.parsed.y} 分钟` },
        },
      },
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
  grid-auto-rows: minmax(84px, auto);
  grid-auto-flow: dense;
  gap: 10px;
  padding: 12px;
}
.card { padding: 10px 12px; min-width: 0; }
.card h4 { margin: 0 0 8px; font-size: 12px; color: var(--text-mid); font-weight: 600; letter-spacing: 0.02em; }
.chart-box { position: relative; height: 72px; }
.big { font-size: 22px; margin: 4px 0; color: var(--accent-bright); line-height: 1.15; }
.summary p { margin: 2px 0; font-size: 12px; }
.sub { font-size: 11px; color: var(--text-low); margin: 2px 0 0; }
.na { font-size: 11px; color: var(--text-low); margin: 2px 0; }
.num { color: var(--text-hi); }
.na { color: var(--text-mid); }
.sub { font-size: 11px; color: var(--text-mid); }
</style>