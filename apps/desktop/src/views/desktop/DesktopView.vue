<script setup lang="ts">
import { useAgentStore } from "../../stores/agent";

const agent = useAgentStore();
const launcherApps = ["VS Code", "Obsidian", "浏览器", "文件夹"];
const quickPages = ["课程项目", "日记", "任务", "统计"];
</script>

<template>
  <div class="desktop-view">
    <header class="topbar">
      <div class="task">当前任务：实现统计模块</div>
      <div class="focus-timer">专注 00:00:00 · 休息 12:00</div>
      <div class="agent-status">
        Agent: {{ agent.state }}
        <span class="dot" :class="`st-${agent.state}`"></span>
      </div>
    </header>

    <section class="launcher">
      <div v-for="app in launcherApps" :key="app" class="icon" :class="`ic-${app}`">
        <span class="glyph">▢</span>
        <span class="name">{{ app }}</span>
      </div>
    </section>

    <section class="quick">
      <div v-for="page in quickPages" :key="page" class="icon small">
        <span class="name">{{ page }}</span>
      </div>
    </section>

    <footer class="hint">
      Spike 桌面壳层 · 点击桌宠「对话」可切换面板 · Win+D / Alt+Tab / 全屏行为见可行性报告
    </footer>
  </div>
</template>

<style scoped>
.desktop-view {
  height: 100vh;
  background: linear-gradient(160deg, #10131f 0%, #1a2033 60%, #232b44 100%);
  color: #e8ecf7;
  display: flex;
  flex-direction: column;
  padding: 24px 40px;
  box-sizing: border-box;
  overflow: hidden;
}
.topbar {
  display: flex;
  align-items: center;
  gap: 24px;
  padding-bottom: 18px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.08);
}
.task {
  font-size: 18px;
  font-weight: 600;
}
.focus-timer {
  color: #9fb3ff;
  font-variant-numeric: tabular-nums;
}
.agent-status {
  margin-left: auto;
  font-size: 13px;
  color: #aab4d0;
  display: flex;
  align-items: center;
  gap: 6px;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #55607c;
}
.dot.st-thinking,
.dot.st-reading,
.dot.st-searching {
  background: #f5c542;
}
.dot.st-editing,
.dot.st-running,
.dot.st-testing {
  background: #4f7cff;
}
.dot.st-waiting_permission {
  background: #ff9f43;
}
.dot.st-success {
  background: #22c1a4;
}
.dot.st-error {
  background: #e74c3c;
}
.launcher,
.quick {
  display: flex;
  gap: 18px;
  margin-top: 28px;
  flex-wrap: wrap;
}
.icon {
  width: 92px;
  height: 92px;
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.08);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
}
.icon.small {
  width: auto;
  height: 40px;
  padding: 0 16px;
  flex-direction: row;
  font-size: 13px;
}
.glyph {
  font-size: 28px;
}
.name {
  font-size: 12px;
  color: #c6cfe8;
}
.hint {
  margin-top: auto;
  font-size: 12px;
  color: #6b7596;
}
</style>