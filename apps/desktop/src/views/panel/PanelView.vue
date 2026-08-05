<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useUiStore } from "../../stores/ui";
import { useAgentStore } from "../../stores/agent";

const ui = useUiStore();
const agent = useAgentStore();
const route = useRoute();
const router = useRouter();

const mode = computed(() => (route.path.includes("statistics") ? "statistics" : "chat"));

function setMode(m: "chat" | "statistics") {
  ui.setPanelMode(m);
  void router.push(m === "chat" ? "/panel/chat" : "/panel/statistics");
}
</script>

<template>
  <div class="panel-window">
    <header class="panel-header">
      <span class="agent-name">Agent：{{ agent.agentId }}</span>
      <nav class="tabs">
        <button :class="{ active: mode === 'chat' }" @click="setMode('chat')">对话</button>
        <button :class="{ active: mode === 'statistics' }" @click="setMode('statistics')">统计</button>
      </nav>
    </header>
    <main class="panel-body">
      <router-view />
    </main>
  </div>
</template>

<style scoped>
.panel-window {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: #f7f8fc;
  color: #1c2233;
  box-sizing: border-box;
}
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px;
  background: #232b44;
  color: #eef;
}
.agent-name {
  font-size: 13px;
  font-weight: 600;
}
.tabs {
  display: flex;
  gap: 6px;
}
.tabs button {
  border: none;
  border-radius: 8px;
  padding: 4px 12px;
  cursor: pointer;
  background: rgba(255, 255, 255, 0.1);
  color: #cdd6ee;
  font-size: 12px;
}
.tabs button.active {
  background: #4f7cff;
  color: #fff;
}
.panel-body {
  flex: 1;
  overflow: hidden;
  display: flex;
}
.panel-body > * {
  flex: 1;
  min-width: 0;
}
</style>