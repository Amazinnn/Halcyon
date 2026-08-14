<script setup lang="ts">
/**
 * Overview panel (extensibility plan C4): the declarative panel recipe in
 * action — a window registry entry + view registry entry + capability list
 * entry, assembled from read-only queries, event subscriptions, and kit
 * components. No new Rust commands were needed.
 */
import { onBeforeUnmount, onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import FocusWindowFrame from "../../components/focus/FocusWindowFrame.vue";
import FocusCard from "../../components/focus/FocusCard.vue";

const todaySec = ref(0);
const todayRounds = ref(0);
const runs = ref<{ id: string; workflowName: string; status: string; triggeredBy: string; startedAt: string }[]>([]);
let unlisten: UnlistenFn[] = [];

function fmtShort(totalSec: number) {
  const s = Math.max(0, Math.floor(totalSec));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return h > 0 ? `${h}h${m}m` : `${m} 分钟`;
}

async function loadToday() {
  try {
    const [total, rounds] = await invoke<[number, number]>("get_today_focus_summary");
    todaySec.value = total;
    todayRounds.value = rounds;
  } catch {
    /* panel is read-only; keep last values */
  }
}

async function loadRuns() {
  try {
    const rows = await invoke<{ id: string; workflowName: string; status: string; triggeredBy: string; startedAt: string }[]>("workflow_runs_recent", { limit: 5 });
    runs.value = rows;
  } catch {
    /* read-only */
  }
}

onMounted(async () => {
  await Promise.all([loadToday(), loadRuns()]);
  unlisten.push(await listen("workflow:changed", () => void loadRuns()));
  unlisten.push(await listen("stats:changed", () => void loadToday()));
});

onBeforeUnmount(() => {
  for (const u of unlisten) u();
});
</script>

<template>
  <div class="overview-window">
    <FocusWindowFrame title="概览" collapsible />
    <div class="ov-body">
      <FocusCard title="今日专注">
        <div class="ov-row">
          <span class="ov-big num">{{ fmtShort(todaySec) }}</span>
          <span class="ov-muted">完成 {{ todayRounds }} 轮</span>
        </div>
      </FocusCard>
      <FocusCard title="最近运行">
        <div v-if="!runs.length" class="ov-muted">暂无运行记录</div>
        <div v-else class="ov-runs">
          <div v-for="r in runs" :key="r.id" class="ov-run">
            <span class="ov-run-name" :title="r.workflowName">{{ r.workflowName }}</span>
            <span class="run-status" :class="r.status">{{ r.status }}</span>
          </div>
        </div>
      </FocusCard>
    </div>
  </div>
</template>

<style scoped>
.overview-window { display: flex; flex-direction: column; height: 100%; }
.ov-body { flex: 1; overflow-y: auto; padding: 10px 12px; display: flex; flex-direction: column; gap: 10px; }
.ov-row { display: flex; align-items: baseline; gap: 8px; }
.ov-big { font-size: 22px; color: var(--accent-bright); font-weight: 600; }
.ov-muted { color: var(--text-low); font-size: 11px; }
.ov-runs { display: flex; flex-direction: column; gap: 4px; }
.ov-run { display: flex; align-items: center; justify-content: space-between; gap: 6px; }
.ov-run-name { font-size: 11px; color: var(--text-hi); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
.run-status { font-size: 10px; padding: 1px 6px; border-radius: var(--r-pill); flex-shrink: 0; }
.run-status.success { color: #2ecc71; background: rgba(46, 204, 113, 0.12); }
.run-status.failed { color: #ff5555; background: rgba(255, 85, 85, 0.12); }
.run-status.cancelled { color: #e8c766; background: rgba(232, 199, 102, 0.12); }
.run-status.skipped, .run-status.running { color: var(--text-low); background: rgba(255, 255, 255, 0.06); }
</style>
