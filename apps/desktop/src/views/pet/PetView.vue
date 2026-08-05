<script setup lang="ts">
import { computed } from "vue";
import { emit } from "@tauri-apps/api/event";
import { useAgentStore } from "../../stores/agent";
import { useUiStore } from "../../stores/ui";
import { useGridDrag } from "../../composables/useGridDrag";

const agent = useAgentStore();
const ui = useUiStore();
const { onPointerDown, onPointerMove, onPointerUp } = useGridDrag("pet");

const bubbleVisible = computed(() => {
  if (!agent.bubble) return false;
  if (ui.chatOpen) return false;
  return Date.now() < agent.bubble.expiresAt;
});

function toggleChat() {
  void emit("ui:toggle_chat", {});
}
</script>

<template>
  <div class="pet-window" @pointerdown="onPointerDown" @pointermove="onPointerMove" @pointerup="onPointerUp">
    <div v-if="bubbleVisible" class="bubble" :class="`prio-${agent.bubble?.priority}`" data-no-drag>
      {{ agent.bubble?.text }}
    </div>
    <div class="sprout" :class="`anim-${agent.animation}`" data-no-drag>
      <svg viewBox="0 0 64 64" width="72" height="72">
        <path d="M32 58 C32 42 32 30 32 22" stroke="#a3e635" stroke-width="3" fill="none" stroke-linecap="round" />
        <path d="M32 34 C20 30 15 20 19 11 C28 11 34 21 32 34Z" fill="#4ade80" />
        <path d="M32 26 C44 22 49 14 45 6 C37 8 31 16 32 26Z" fill="#a3e635" />
        <path d="M30 46 C22 44 18 38 20 32 C26 33 30 38 30 46Z" fill="#16a34a" />
      </svg>
      <span class="halo" :class="`st-${agent.state}`"></span>
    </div>
    <button class="open-btn" @click.stop="toggleChat">对话</button>
    <div class="pet-name">{{ agent.state }}</div>
  </div>
</template>

<style scoped>
.pet-window {
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
  box-sizing: border-box;
  cursor: grab;
}
.sprout { position: relative; display: flex; align-items: center; justify-content: center; }
.halo {
  position: absolute; inset: -6px;
  border-radius: 50%;
  border: 2px solid rgba(163, 230, 53, 0.45);
  filter: blur(1px);
}
.halo.st-waiting_permission { border-color: var(--warn); }
.halo.st-error { border-color: var(--err); }
.anim-thinking { animation: sway 0.6s ease-in-out infinite alternate; }
.anim-editing { animation: shake 0.3s ease-in-out infinite alternate; }
.anim-waiting { animation: pulse 1.2s ease-in-out infinite; }
.anim-success { animation: bloom 0.6s ease-out 1; }
.anim-error { transform: rotate(-8deg); filter: grayscale(0.5); }
@keyframes sway { from { transform: rotate(-5deg); } to { transform: rotate(5deg); } }
@keyframes shake { from { transform: translateX(-3px); } to { transform: translateX(3px); } }
@keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.55; } }
@keyframes bloom { 0% { transform: scale(0.8); } 60% { transform: scale(1.12); } 100% { transform: scale(1); } }
.bubble {
  position: absolute; top: 2px; left: 50%; transform: translateX(-50%);
  max-width: 150px; background: rgba(238, 247, 230, 0.95); color: #12211a;
  border-radius: 10px; padding: 5px 10px; font-size: 12px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis; z-index: 5;
}
.bubble.prio-high, .bubble.prio-critical { border: 2px solid var(--warn); }
.open-btn {
  border: none; border-radius: var(--r-pill);
  padding: 4px 14px; font-size: 12px; cursor: pointer;
  background: var(--glass-strong); color: var(--accent-bright);
  border: 1px solid var(--glass-border);
}
.pet-name { font-size: 10px; color: var(--text-low); }
</style>