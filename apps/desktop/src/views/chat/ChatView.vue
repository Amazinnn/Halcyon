<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useAgentStore } from "../../stores/agent";
import WindowHeader from "../../components/WindowHeader.vue";

const agent = useAgentStore();
const input = ref("");
const workspaceInput = ref("");
const listRef = ref<HTMLElement | null>(null);
const optionsOpen = ref(false);

const QUICK = [
  { label: "计时器状态", text: "请用 focus-cli 查看当前计时器状态" },
  { label: "开始专注", text: "请用 focus-cli 开始专注" },
  { label: "今日统计", text: "请用 focus-cli 查看今日专注统计" },
];

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
  workspaceInput.value = agent.workspaceDir;
});

function send() {
  const text = input.value.trim();
  if (!text) return;
  input.value = "";
  void agent.send(text);
}

function sendQuick(text: string) {
  input.value = "";
  void agent.send(text);
}

function useSkill(name: string) {
  input.value = `使用技能 ${name}：`;
}

async function applyWorkspace() {
  const dir = workspaceInput.value.trim();
  try {
    await agent.setWorkspaceDir(dir);
  } catch (e) {
    agent.pushSystem(`工作区设置失败：${e}`);
  }
}
</script>

<template>
  <div class="chat-window">
    <WindowHeader :title="agent.characterName" collapsible />

    <div class="agent-row">
      <select class="agent-select" :value="agent.characterId" @change="agent.selectCharacter(($event.target as HTMLSelectElement).value)">
        <option v-for="c in agent.characters" :key="c.id" :value="c.id">{{ c.name }}</option>
      </select>
      <span class="badge" :class="agent.provider">
        {{ agent.provider === "mock" && agent.fallback ? "Mock（回退）" : agent.provider === "mock" ? "Mock" : "Codex" }}
      </span>
      <span class="phase" :class="agent.phase">{{ phaseText }}</span>
      <button class="ghost" @click="agent.newThread()">新会话</button>
      <button v-if="agent.threads.some((t) => t.automation)" class="ghost" @click="agent.cleanupAutomationThreads()">清理自动化</button>
      <button class="ghost" @click="optionsOpen = !optionsOpen">选项</button>
    </div>

    <div v-if="optionsOpen" class="options">
      <div class="row wrap">
        <span class="label">技能</span>
        <span v-if="!agent.skills.length" class="muted">未发现 ~/.codex/skills</span>
        <span v-for="s in agent.skills" :key="s" class="chip" @click="useSkill(s)">{{ s }}</span>
      </div>
      <div class="row wrap">
        <span class="label">focus-cli</span>
        <span v-for="q in QUICK" :key="q.label" class="chip" @click="sendQuick(q.text)">{{ q.label }}</span>
      </div>
      <div class="row">
        <span class="label">工作区</span>
        <input v-model="workspaceInput" class="text-input" placeholder="agent 工作目录（默认用户主目录）" />
        <button class="btn" @click="applyWorkspace">应用</button>
      </div>
    </div>

    <!-- M5 (ADR-0022): history sessions are not listed in the UI — the
         Agent reads its own hash via focus-cli (agent session). -->

    <div ref="listRef" class="msg-list">
      <div v-if="agent.messages.length === 0" class="empty">输入消息开始与 Agent 对话…</div>
      <div v-for="(m, i) in agent.messages" :key="i" class="msg glass" :class="[m.role, m.kind]">
        <span class="who">{{ m.role === "agent" ? (m.kind === "system" ? "系统" : "Agent") : "我" }}</span>
        <span class="text">{{ m.text }}</span>
        <span v-if="m.kind === 'delta'" class="cursor">▍</span>
      </div>
    </div>

    <div class="tool-strip">
      <span v-if="agent.tools.length === 0" class="tool-chip muted">无工具调用</span>
      <span v-for="(t, i) in agent.tools.slice(-4)" :key="i" class="tool-chip" :class="t.status" :title="t.summary">
        {{ t.tool }} {{ t.status === "started" ? "…" : "✓" }}
      </span>
    </div>

    <form class="composer" @submit.prevent="send">
      <input v-model="input" placeholder="输入消息…" :disabled="agent.phase === 'connecting'" />
      <button v-if="agent.phase === 'streaming' || agent.phase === 'connecting'" type="button" class="stop" @click="agent.interrupt()">
        停止
      </button>
      <button type="submit" :disabled="!input.trim() || agent.phase === 'connecting'">发送</button>
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
.badge.mock {
  background: #3a3318;
  color: #e8c766;
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
.options {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px 12px;
  border-bottom: 1px solid var(--glass-border);
  background: rgba(16, 26, 21, 0.6);
}
.row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}
.row.wrap {
  flex-wrap: wrap;
}
.row .label {
  color: var(--text-low);
  min-width: 52px;
}
.seg {
  display: flex;
  gap: 4px;
}
.seg button {
  border: 1px solid var(--glass-border);
  background: transparent;
  color: var(--text-mid);
  border-radius: var(--r-sm);
  font-size: 11px;
  padding: 2px 10px;
  cursor: pointer;
}
.seg button.on {
  background: var(--accent);
  color: #0a110e;
  border-color: var(--accent);
}
.text-input {
  flex: 1;
  border: 1px solid var(--glass-border);
  border-radius: var(--r-sm);
  padding: 4px 8px;
  font-size: 12px;
  background: #101a15;
  color: var(--text-hi);
}
.btn {
  border: 1px solid var(--glass-border);
  background: transparent;
  color: var(--text-mid);
  border-radius: var(--r-sm);
  font-size: 11px;
  padding: 3px 10px;
  cursor: pointer;
}
.chip {
  font-size: 11px;
  border-radius: var(--r-pill);
  padding: 2px 8px;
  background: var(--glass);
  color: var(--text-mid);
  cursor: pointer;
  border: 1px solid var(--glass-border);
}
.chip:hover {
  color: var(--accent-bright);
  border-color: var(--accent);
}
.muted {
  color: var(--text-low);
  font-size: 11px;
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
.cursor {
  color: var(--accent-bright);
  animation: blink 1s steps(1) infinite;
}
@keyframes blink {
  50% {
    opacity: 0;
  }
}
.tool-strip {
  display: flex;
  gap: 6px;
  padding: 6px 12px;
  flex-wrap: wrap;
  border-top: 1px solid var(--glass-border);
}
.tool-chip {
  font-size: 11px;
  border-radius: var(--r-pill);
  padding: 2px 8px;
  background: var(--glass);
  color: var(--text-mid);
}
.tool-chip.completed {
  background: #183624;
  color: var(--accent-bright);
}
.tool-chip.muted {
  color: var(--text-low);
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