<script setup lang="ts">
import { computed } from "vue";
import { useAgentStore } from "../../stores/agent";
import { useUiStore } from "../../stores/ui";
import { BUILTIN_PET_MANIFEST, validatePetManifest } from "../../lib/petPack";

const agent = useAgentStore();
const ui = useUiStore();

// Validate the built-in manifest at startup (design doc §5.4 import rules).
const manifest = validatePetManifest(BUILTIN_PET_MANIFEST);

// §7.1: chat open -> pet keeps animation but no normal bubbles.
const bubbleVisible = computed(() => {
  if (!agent.bubble) return false;
  if (ui.panelMode === "chat") return false;
  if (ui.doNotDisturb || ui.lockActive) return false;
  return Date.now() < agent.bubble.expiresAt;
});

function togglePanel() {
  ui.togglePanel();
}
</script>

<template>
  <div class="pet-window" data-tauri-drag-region>
    <div class="bubble" v-if="bubbleVisible" :class="`prio-${agent.bubble?.priority}`">
      {{ agent.bubble?.text }}
    </div>
    <div class="pet" :class="`anim-${agent.animation}`">
      <span class="face">{{ agent.state === "error" ? "×_×" : "◕‿◕" }}</span>
      <span class="badge" v-if="agent.state === 'waiting_permission'">!</span>
    </div>
    <button class="open-btn" @click.stop="togglePanel">对话</button>
    <div class="pet-name">{{ manifest.name }} · {{ agent.state }}</div>
  </div>
</template>

<style scoped>
.pet-window {
  position: relative;
  width: 100%;
  height: 100%;
  background: transparent;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: flex-end;
  gap: 6px;
  box-sizing: border-box;
}
.pet {
  width: 96px;
  height: 96px;
  border-radius: 50%;
  background: radial-gradient(circle at 35% 30%, #ffd9a0, #f2a65a 70%);
  box-shadow: 0 8px 20px rgba(0, 0, 0, 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  transition: transform 0.2s;
}
.face {
  font-size: 34px;
  line-height: 1;
}
.badge {
  position: absolute;
  top: 6px;
  right: 10px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #e74c3c;
  color: #fff;
  font-size: 12px;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
}
/* placeholder animations driven by manifest animation names */
.anim-thinking {
  animation: bob 0.5s ease-in-out infinite alternate;
}
.anim-editing {
  animation: shake 0.35s ease-in-out infinite alternate;
}
.anim-waiting {
  animation: bob 1.2s ease-in-out infinite;
}
.anim-success {
  animation: pop 0.6s ease-out 1;
}
.anim-error {
  filter: grayscale(0.7);
}
@keyframes bob {
  from {
    transform: translateY(0);
  }
  to {
    transform: translateY(-10px);
  }
}
@keyframes shake {
  from {
    transform: rotate(-6deg);
  }
  to {
    transform: rotate(6deg);
  }
}
@keyframes pop {
  0% {
    transform: scale(0.8);
  }
  60% {
    transform: scale(1.1);
  }
  100% {
    transform: scale(1);
  }
}
.bubble {
  position: absolute;
  top: 2px;
  left: 50%;
  transform: translateX(-50%);
  max-width: 150px;
  background: #fff;
  color: #222;
  border-radius: 10px;
  padding: 6px 10px;
  font-size: 12px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  z-index: 5;
}
.bubble.prio-high,
.bubble.prio-critical {
  border: 2px solid #e74c3c;
}
.open-btn {
  border: none;
  border-radius: 10px;
  padding: 4px 12px;
  font-size: 12px;
  cursor: pointer;
  background: rgba(30, 34, 60, 0.85);
  color: #eef;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
}
.pet-name {
  font-size: 10px;
  color: #666;
  background: rgba(255, 255, 255, 0.75);
  border-radius: 6px;
  padding: 1px 8px;
}
</style>