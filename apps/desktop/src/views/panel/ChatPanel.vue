<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { useAgentStore } from "../../stores/agent";

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
  <div class="chat-panel">
    <div ref="listRef" class="message-list">
      <div v-if="agent.messages.length === 0" class="empty">等待 Mock Agent 事件…</div>
      <div v-for="(m, i) in agent.messages" :key="i" class="msg" :class="m.role">
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
      <input v-model="input" placeholder="输入消息（Spike 仅本地回显）" />
      <button type="submit">发送</button>
    </form>
  </div>
</template>

<style scoped>
.chat-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
}
.message-list {
  flex: 1;
  overflow-y: auto;
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.empty {
  color: #99a;
  font-size: 12px;
  text-align: center;
  margin-top: 40px;
}
.msg {
  max-width: 85%;
  padding: 6px 10px;
  border-radius: 10px;
  font-size: 13px;
  line-height: 1.5;
}
.msg.agent {
  align-self: flex-start;
  background: #fff;
  border: 1px solid #e3e6f0;
}
.msg.user {
  align-self: flex-end;
  background: #4f7cff;
  color: #fff;
}
.who {
  display: block;
  font-size: 11px;
  opacity: 0.7;
  margin-bottom: 2px;
}
.tool-strip {
  display: flex;
  gap: 6px;
  padding: 6px 12px;
  border-top: 1px solid #e3e6f0;
  flex-wrap: wrap;
}
.tool-chip {
  font-size: 11px;
  border-radius: 8px;
  padding: 2px 8px;
  background: #eef1f7;
  color: #445;
}
.tool-chip.completed {
  background: #dff3ec;
  color: #13795f;
}
.tool-chip.muted {
  color: #99a;
}
.composer {
  display: flex;
  gap: 8px;
  padding: 8px 12px;
  border-top: 1px solid #e3e6f0;
}
.composer input {
  flex: 1;
  border: 1px solid #d5d9e8;
  border-radius: 8px;
  padding: 6px 10px;
  font-size: 13px;
}
.composer button {
  border: none;
  border-radius: 8px;
  background: #4f7cff;
  color: #fff;
  padding: 0 14px;
  cursor: pointer;
  font-size: 13px;
}
</style>