<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useAgentStore } from "../../stores/agent";
import {
  hasInlineComposerBody,
  insertSkillToken,
  serializeInlineComposer,
  type InlineComposerPart,
} from "../../lib/inline-composer";
import WindowHeader from "../../components/WindowHeader.vue";
import { useSettingsStore } from "../../stores/settings";

const agent = useAgentStore();
const settings = useSettingsStore();
const skillPicker = ref("");
const listRef = ref<HTMLElement | null>(null);
const editorRef = ref<HTMLElement | null>(null);
const composerText = ref("");
const hasComposerBody = ref(false);
let savedComposerRange: Range | null = null;
const isBusy = computed(() => agent.phase === "connecting" || agent.phase === "streaming");

const phaseText = computed(() => {
  if (agent.phase === "connecting" || agent.phase === "streaming") {
    if (agent.phase === "connecting") return "连接中…";
    if (agent.provider === "claude" && settings.chatStreamingEnabled && !agent.publicTextDeltaSeen) return "等待 Claude 公开输出…";
    return "生成中…";
  }
  switch (agent.phase) {
    case "com" + "pleted":
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
  document.addEventListener("selectionchange", rememberComposerSelection);
  await agent.init();
  await settings.load();
});

onBeforeUnmount(() => {
  document.removeEventListener("selectionchange", rememberComposerSelection);
});

function send() {
  const text = composerText.value.trim();
  if (!text || !hasComposerBody.value) return;
  // v1.12.2: never send with an empty characterId (Rust would report
  // "角色不存在") — prompt instead.
  if (!agent.characterId) {
    agent.errorMessage = "请选择 Agent 后再发送消息。";
    agent.pushSystem("请先选择 Agent");
    return;
  }
  editorRef.value?.replaceChildren();
  composerText.value = "";
  hasComposerBody.value = false;
  void agent.send(text);
}

function editorParts(): InlineComposerPart[] {
  const editor = editorRef.value;
  if (!editor) return [];
  return Array.from(editor.childNodes).flatMap<InlineComposerPart>((node) => {
    if (node instanceof HTMLElement && node.dataset.skill) {
      return [{ kind: "skill" as const, name: node.dataset.skill }];
    }
    const text = node.textContent ?? "";
    return text ? [{ kind: "text" as const, text }] : [];
  });
}

function composerRange(): Range | null {
  const editor = editorRef.value;
  const selection = window.getSelection();
  const range = selection?.rangeCount ? selection.getRangeAt(0) : null;
  if (!editor || !range || !editor.contains(range.startContainer) || !editor.contains(range.endContainer)) {
    return null;
  }
  return range;
}

function rememberComposerSelection() {
  const range = composerRange();
  if (range) savedComposerRange = range.cloneRange();
}

function syncComposer() {
  const parts = editorParts();
  composerText.value = serializeInlineComposer(parts);
  hasComposerBody.value = hasInlineComposerBody(parts);
  rememberComposerSelection();
}

function restoreComposerSelection(): Range | null {
  const editor = editorRef.value;
  if (!editor) return null;
  editor.focus();
  const range = savedComposerRange?.cloneRange() ?? document.createRange();
  if (!savedComposerRange) {
    range.selectNodeContents(editor);
    range.collapse(false);
  }
  const selection = window.getSelection();
  selection?.removeAllRanges();
  selection?.addRange(range);
  return range;
}

function caretAtComposerStart(range: Range): boolean {
  const editor = editorRef.value;
  if (!editor) return true;
  const before = range.cloneRange();
  before.selectNodeContents(editor);
  before.setEnd(range.startContainer, range.startOffset);
  return before.toString().length === 0;
}

function setCaretAfter(node: Node) {
  const range = document.createRange();
  range.setStartAfter(node);
  range.collapse(true);
  const selection = window.getSelection();
  selection?.removeAllRanges();
  selection?.addRange(range);
  savedComposerRange = range.cloneRange();
}

function adjacentNode(range: Range, direction: "backward" | "forward"): Node | null {
  const editor = editorRef.value;
  if (!editor || !range.collapsed) return null;
  const container = range.startContainer;
  const offset = range.startOffset;
  if (container === editor) {
    return direction === "backward"
      ? editor.childNodes[offset - 1] ?? null
      : editor.childNodes[offset] ?? null;
  }
  if (container.nodeType === Node.TEXT_NODE) {
    const text = container.textContent ?? "";
    if (direction === "backward" && offset > 0) return null;
    if (direction === "forward" && offset < text.length) return null;
  }
  return direction === "backward" ? container.previousSibling : container.nextSibling;
}

function adjacentSkill(range: Range, direction: "backward" | "forward"): HTMLElement | null {
  let node = adjacentNode(range, direction);
  while (node?.nodeType === Node.TEXT_NODE && /^\s*$/.test(node.textContent ?? "")) {
    node = direction === "backward" ? node.previousSibling : node.nextSibling;
  }
  return node instanceof HTMLElement && node.dataset.skill ? node : null;
}

function removeSkillToken(token: HTMLElement) {
  const before = token.previousSibling;
  const after = token.nextSibling;
  if (before?.nodeType === Node.TEXT_NODE && /^\s*$/.test(before.textContent ?? "")) before.remove();
  token.remove();
  if (after?.nodeType === Node.TEXT_NODE && /^\s*$/.test(after.textContent ?? "")) after.remove();
  editorRef.value?.focus();
  syncComposer();
}

function handleComposerKeydown(event: KeyboardEvent) {
  if (event.key === "Enter") {
    event.preventDefault();
    send();
    return;
  }
  if (event.key !== "Backspace" && event.key !== "Delete") return;
  const range = composerRange();
  if (!range) return;
  const token = adjacentSkill(range, event.key === "Backspace" ? "backward" : "forward");
  if (!token) return;
  event.preventDefault();
  removeSkillToken(token);
}

function handleComposerPaste(event: ClipboardEvent) {
  event.preventDefault();
  document.execCommand("insertText", false, event.clipboardData?.getData("text/plain") ?? "");
  syncComposer();
}

function selectSkill(event: Event) {
  const target = event.target as HTMLSelectElement;
  skillPicker.value = "";
  if (!target.value) return;
  const range = restoreComposerSelection();
  if (!range) return;
  range.deleteContents();
  const fragment = document.createDocumentFragment();
  if (!caretAtComposerStart(range)) fragment.append(document.createTextNode("  "));
  const tokenPart = insertSkillToken([], 0, target.value)[0];
  const token = document.createElement("span");
  token.className = "skill-chip";
  token.contentEditable = "false";
  token.dataset.skill = tokenPart.kind === "skill" ? tokenPart.name : target.value;
  token.textContent = `$${token.dataset.skill}`;
  const spacer = document.createTextNode("  ");
  fragment.append(token, spacer);
  range.insertNode(fragment);
  setCaretAfter(spacer);
  syncComposer();
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
      <span class="badge">{{ agent.provider === "claude" ? "Claude" : "Codex" }}</span>
      <span v-if="isBusy" class="phase" :class="agent.phase">{{ phaseText }}</span>
      <button class="ghost" :class="{ on: settings.chatStreamingEnabled }" :disabled="isBusy" @click="settings.setChatStreamingEnabled(!settings.chatStreamingEnabled)">
        流式 {{ settings.chatStreamingEnabled ? "开" : "关" }}
      </button>
    </div>

    <div ref="listRef" class="msg-list">
      <div v-if="agent.messages.length === 0" class="empty">输入消息开始与 Agent 对话…</div>
      <div v-for="(m, i) in agent.messages" :key="i" class="msg glass" :class="[m.role, m.kind]">
        <span class="who">
          {{ m.role === "agent" ? (m.kind === "system" ? "系统" : agent.characterName) : "我" }}
          <span v-if="m.source" class="source">{{ m.source }}</span>
        </span>
        <span v-if="m.thinking" class="thinking" aria-label="思考过程">{{ m.thinking }}</span>
        <span class="text">{{ m.text }}</span>
        <span v-if="m.kind === 'delta'" class="cursor">▍</span>
      </div>
    </div>

    <p v-if="agent.errorMessage" class="error-message" role="alert">{{ agent.errorMessage }}</p>
    <form class="composer" @submit.prevent="send">
      <select
        v-if="agent.skills.length"
        class="skill-select"
        aria-label="Skills"
        :value="skillPicker"
        :disabled="isBusy || !agent.characterId"
        @change="selectSkill"
      >
        <option value="">Skills</option>
        <option v-for="skill in agent.skills" :key="skill" :value="skill">{{ skill }}</option>
      </select>
      <div class="composer-input">
        <div
          ref="editorRef"
          class="composer-editor"
          :contenteditable="!isBusy && !!agent.characterId"
          role="textbox"
          aria-label="输入消息"
          data-placeholder="输入消息…"
          @keydown="handleComposerKeydown"
          @input="syncComposer"
          @paste="handleComposerPaste"
        ></div>
      </div>
      <button v-if="agent.phase === 'streaming'" type="button" class="stop" @click="agent.interrupt()">
        停止
      </button>
      <button type="submit" :disabled="!hasComposerBody || isBusy || !agent.characterId">发送</button>
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
  border-radius: var(--window-host-radius);
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
.thinking {
  display: block;
  margin: 0 0 8px;
  padding: 8px 10px;
  border-left: 2px solid rgba(163, 230, 53, 0.28);
  background: rgba(163, 230, 53, 0.05);
  border-radius: 0 var(--r-sm) var(--r-sm) 0;
  font-size: 11px;
  line-height: 1.5;
  color: var(--text-low);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
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
.skill-select {
  width: 86px;
  flex: 0 0 86px;
  border: 1px solid var(--glass-border);
  border-radius: var(--r-sm);
  background: #101a15;
  color: var(--text-mid);
  font-size: 12px;
  padding: 0 6px;
}
.error-message {
  margin: 0;
  padding: 6px 12px;
  border-top: 1px solid var(--glass-border);
  color: #ffb4ae;
  font-size: 12px;
}
.composer-input {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  align-content: center;
  flex-wrap: wrap;
  gap: 8px;
  border: 1px solid var(--glass-border);
  border-radius: var(--r-sm);
  min-height: 36px;
  padding: 3px 8px;
  background: #101a15;
}
.skill-chip {
  flex: 0 0 auto;
  padding: 0 8px;
  border: 1px solid rgba(190, 222, 163, 0.22);
  border-radius: 4px;
  background: #1a281c;
  color: #d8edcb;
  font-family: "Cascadia Mono", Consolas, "Microsoft YaHei UI", monospace;
  font-size: 14px;
  font-weight: 700;
  line-height: 26px;
  letter-spacing: 0;
  white-space: nowrap;
  user-select: none;
}
.composer-editor {
  min-width: 120px;
  flex: 1;
  outline: none;
  color: var(--text-hi);
  font-size: 13px;
  line-height: 30px;
  white-space: pre-wrap;
  word-break: break-word;
}
.composer-editor:empty::before {
  content: attr(data-placeholder);
  color: var(--text-low);
  pointer-events: none;
}
.composer-editor[contenteditable="false"] {
  opacity: 0.55;
  pointer-events: none;
}
.composer-input:focus-within {
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
