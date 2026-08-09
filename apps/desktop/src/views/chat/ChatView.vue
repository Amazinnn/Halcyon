<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useAgentStore } from "../../stores/agent";
import WindowHeader from "../../components/WindowHeader.vue";

const agent = useAgentStore();
const input = ref("");
const listRef = ref<HTMLElement | null>(null);
const isBusy = computed(() => agent.phase === "connecting" || agent.phase === "streaming");

const phaseText = computed(() => {
  switch (agent.phase) {
    case "connecting":
      return "连接中…";
    case "streaming":
      return "生成中…";
    case "completed":
      return "完成";
    case "error":
      return "错误";
    default:
      return "空闲";
  }
});

watch(
  () => agent.messages.length,
  async () => {
    await nextTick();
    listRef.value?.scrollTo({ top: listRef.value.scrollHeight });
  },
);

onMounted(async () => {
  await agent.init();
});

function send() {
  const text = input.value.trim();
  if (!text) return;
  // v1.12.2: never send with an empty characterId (Rust would report
  // "角色不存在") — prompt instead.
  if (!agent.characterId) {
    agent.pushSystem("请先选择 Agent");
    return;
  }
  input.value = "";
  void agent.send(text);
}

</script>

<template>
  <div class="chat-window">
    <WindowHeader :title="agent.characterName" collapsible />

    <div class="agent-row">
      <select v-if="agent.characters.length" class="agent-select" :value="agent.characterId" :disabled="isBusy" @change="agent.selectCharacter(($event.target as HTMLSelectElement).value)">
        <option v-for="c in agent.characters" :key="c.id" :value="c.id">{{ c.name }}</option>
      </select>
      <button v-else class="ghost" @click="agent.refreshCharacters()">Agent 正在初始化，点击刷新</button>
      <span class="badge">Codex</span>
      <span class="phase" :class="agent.phase">{{ phaseText }}</span>
    </div>

    <div ref="listRef" class="msg-list">
      <div v-if="agent.messages.length === 0" class="empty">输入消息开始与 Agent 对话…</div>
      <div v-for="(m, i) in agent.messages" :key="i" class="msg glass" :class="[m.role, m.kind]">
        <span class="who">
          {{ m.role === "agent" ? (m.kind === "system" ? "系统" : "Agent") : "我" }}
          <span v-if="m.source" class="source">{{ m.source }}</span>
        </span>
        <span class="text">{{ m.text }}</span>
        <span v-if="m.kind === 'delta'" class="cursor">▍</span>
      </div>
    </div>

    <form class="composer" @submit.prevent="send">
      <input v-model="input" placeholder="输入消息…" :disabled="isBusy || !agent.characterId" />
      <button v-if="agent.phase === 'streaming' || agent.phase === 'connecting'" type="button" class="stop" @click="agent.interrupt()">
        停止
      </button>
      <button type="submit" :disabled="!input.trim() || isBusy || !agent.characterId">发送</button>
    </form>
  </div>
</template>

<style scoped>
.chat-window {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: transparent;
  border: 1px solid var(--glass-border);
  border-radius: var(--r-lg);
  overflow: hidden;
  box-sizing: border-box;
}
.agent-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--glass-border);
  flex-wrap: wrap;
}
.agent-select {
  border: 1px solid var(--glass-border);
  background: #101a15;
  color: var(--text-hi);
  border-radius: var(--r-sm);
  font-size: 12px;
  padding: 3px 8px;
  max-width: 160px;
}
.badge {
  font-size: 11px;
  font-weight: 700;
  border-radius: var(--r-pill);
  padding: 2px 10px;
  background: #183624;
  color: var(--accent-bright);
}
.phase {
  font-size: 11px;
  color: var(--text-mid);
}
.phase.streaming,
.phase.connecting {
  color: var(--accent-bright);
}
.phase.error {
  color: #ff7b72;
}
.ghost {
  border: 1px solid var(--glass-border);
  background: transparent;
  color: var(--text-mid);
  border-radius: var(--r-sm);
  font-size: 11px;
  padding: 2px 8px;
  cursor: pointer;
}
.ghost:hover {
  color: var(--text-hi);
  border-color: var(--accent);
}
.msg-list {
  flex: 1;
  overflow-y: auto;
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.empty {
  color: var(--text-low);
  font-size: 12px;
  text-align: center;
  margin-top: 40px;
}
.msg {
  max-width: 85%;
  padding: 6px 10px;
  font-size: 13px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-word;
}
.msg.user {
  align-self: flex-end;
  background: #263718;
  border-color: rgba(163, 230, 53, 0.35);
}
.msg.system {
  align-self: center;
  font-size: 11px;
  color: var(--text-low);
  background: transparent;
  border: none;
}
.who {
  display: block;
  font-size: 11px;
  opacity: 0.7;
  margin-bottom: 2px;
}
.source {
  margin-left: 6px;
  color: var(--accent-bright);
  opacity: 0.8;
}
.cursor {
  color: var(--accent-bright);
  animation: blink 1s steps(1) infinite;
}
@keyframes blink {
  50% {
    opacity: 0;
  }
}
.composer {
  display: flex;
  gap: 8px;
  padding: 8px 12px;
  border-top: 1px solid var(--glass-border);
}
.composer input {
  flex: 1;
  border: 1px solid var(--glass-border);
  border-radius: var(--r-sm);
  padding: 6px 10px;
  font-size: 13px;
  background: #101a15;
  color: var(--text-hi);
}
.composer input:focus {
  outline: none;
  border-color: var(--accent);
}
.composer button {
  border: none;
  border-radius: var(--r-sm);
  background: var(--accent);
  color: #0a110e;
  padding: 0 14px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
}
.composer button:disabled {
  opacity: 0.45;
  cursor: default;
}
.composer button.stop {
  background: #4a1d1d;
  color: #ffb4ae;
}
</style>
