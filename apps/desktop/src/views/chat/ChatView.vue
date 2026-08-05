<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { useAgentStore } from "../../stores/agent";
import WindowHeader from "../../components/WindowHeader.vue";

const agent = useAgentStore();
const input = ref("");
const listRef = ref<HTMLElement | null>(null);

watch(
  () => agent.messages.length,
  async () => {
    await nextTick();
    listRef.value?.scrollTo({ top: listRef.value.scrollHeight });
  },
);

function send() {
  const text = input.value.trim();
  if (!text) return;
  agent.addUserMessage(text);
  input.value = "";
}
</script>

<template>
  <div class="chat-window">
    <WindowHeader title="对话" collapsible />
    <div ref="listRef" class="msg-list">
      <div v-if="agent.messages.length === 0" class="empty">等待 Agent 事件…</div>
      <div v-for="(m, i) in agent.messages" :key="i" class="msg glass" :class="m.role">
        <span class="who">{{ m.role === "agent" ? "Agent" : "我" }}</span>
        <span class="text">{{ m.text }}</span>
      </div>
    </div>
    <div class="tool-strip">
      <span v-if="agent.tools.length === 0" class="tool-chip muted">无工具调用</span>
      <span v-for="(t, i) in agent.tools.slice(-4)" :key="i" class="tool-chip" :class="t.status">
        {{ t.tool }} {{ t.status === "started" ? "…" : "✓" }}
      </span>
    </div>
    <form class="composer" @submit.prevent="send">
      <input v-model="input" placeholder="输入消息（仅本地回显）" />
      <button type="submit">发送</button>
    </form>
  </div>
</template>

<style scoped>
.chat-window {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: rgba(8, 13, 10, 0.35);
  border: 1px solid var(--glass-border);
  border-radius: var(--r-lg);
  overflow: hidden;
  box-sizing: border-box;
}
.msg-list { flex: 1; overflow-y: auto; padding: 10px 12px; display: flex; flex-direction: column; gap: 8px; }
.empty { color: var(--text-low); font-size: 12px; text-align: center; margin-top: 40px; }
.msg { max-width: 85%; padding: 6px 10px; font-size: 13px; line-height: 1.5; }
.msg.user { align-self: flex-end; background: rgba(163, 230, 53, 0.16); border-color: rgba(163, 230, 53, 0.35); }
.who { display: block; font-size: 11px; opacity: 0.7; margin-bottom: 2px; }
.tool-strip { display: flex; gap: 6px; padding: 6px 12px; flex-wrap: wrap; border-top: 1px solid var(--glass-border); }
.tool-chip { font-size: 11px; border-radius: var(--r-pill); padding: 2px 8px; background: var(--glass); color: var(--text-mid); }
.tool-chip.completed { background: rgba(74, 222, 128, 0.16); color: var(--accent-bright); }
.tool-chip.muted { color: var(--text-low); }
.composer { display: flex; gap: 8px; padding: 8px 12px; border-top: 1px solid var(--glass-border); }
.composer input {
  flex: 1; border: 1px solid var(--glass-border); border-radius: var(--r-sm);
  padding: 6px 10px; font-size: 13px; background: rgba(10, 16, 13, 0.5); color: var(--text-hi);
}
.composer input:focus { outline: none; border-color: var(--accent); }
.composer button {
  border: none; border-radius: var(--r-sm); background: var(--accent); color: #0a110e;
  padding: 0 14px; cursor: pointer; font-size: 13px; font-weight: 600;
}
</style>