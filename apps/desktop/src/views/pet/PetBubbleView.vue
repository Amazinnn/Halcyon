<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useAgentStore } from "../../stores/agent";
import {
  BubbleVisibilityRequest,
  bubbleDisplayDurationMs,
  bubbleShouldBeVisible,
  nextBubblePage,
  paginateBubbleTextMeasured,
} from "../../lib/pet-bubble";

const agent = useAgentStore();
const page = ref(0);
const fading = ref(false);
const appearing = ref(false);
let rotation: ReturnType<typeof setInterval> | null = null;
let transitionTimer: ReturnType<typeof setTimeout> | null = null;
let expiryTimer: ReturnType<typeof setTimeout> | null = null;
let unlistenDragStart: (() => void) | null = null;
let unlistenDragEnd: (() => void) | null = null;
const now = ref(Date.now());
const expiresAt = ref(0);
const dragging = ref(false);
const accent = ref("#8aa68d");
const anchor = ref({ x: 0.5, y: 0.05 });
const direction = ref("above");
const visibilityRequests = new BubbleVisibilityRequest();

interface BubblePlacement {
  anchorX: number;
  anchorY: number;
  accent: string;
}

function measureBubbleText(text: string): number {
  const context = document.createElement("canvas").getContext("2d");
  if (!context) return text.length * 14;
  context.font = '600 14px "Segoe UI Variable", "Microsoft YaHei UI", sans-serif';
  return context.measureText(text).width;
}

const pages = computed(() => paginateBubbleTextMeasured(agent.bubble?.text ?? "", measureBubbleText, 214));
const visible = computed(() => bubbleShouldBeVisible({
  hasMessage: Boolean(agent.bubble),
  dragging: dragging.value,
  now: now.value,
  expiresAt: expiresAt.value,
}));
const currentLines = computed(() => pages.value[page.value] ?? []);

async function syncVisibility() {
  const generation = visibilityRequests.issue();
  const bubbleId = agent.bubble?.id ?? null;
  const characterId = agent.characterId;
  if (!visible.value) {
    await invoke("pet_bubble_hide");
    return;
  }
  const placement = await invoke<BubblePlacement | null>("pet_bubble_placement");
  if (!visibilityRequests.isCurrent(generation) || !visible.value || agent.bubble?.id !== bubbleId || agent.characterId !== characterId) return;
  if (!placement) {
    await invoke("pet_bubble_hide");
    return;
  }
  accent.value = placement.accent;
  anchor.value = { x: placement.anchorX, y: placement.anchorY };
  const nextDirection = await invoke<string | null>("pet_bubble_show", { anchorX: anchor.value.x, anchorY: anchor.value.y });
  if (!visibilityRequests.isCurrent(generation) || !visible.value || agent.bubble?.id !== bubbleId || agent.characterId !== characterId) {
    if (!visible.value) {
      await invoke("pet_bubble_hide");
    }
    return;
  }
  if (!nextDirection) return;
  direction.value = nextDirection;
  appearing.value = true;
  requestAnimationFrame(() => { appearing.value = false; });
}

function restartRotation() {
  if (rotation) clearInterval(rotation);
  if (transitionTimer) clearTimeout(transitionTimer);
  page.value = 0;
  fading.value = false;
  if (pages.value.length <= 1) return;
  rotation = setInterval(() => {
    fading.value = true;
    transitionTimer = setTimeout(() => {
      page.value = nextBubblePage(page.value, pages.value.length);
      fading.value = false;
      transitionTimer = null;
    }, 180);
  }, 3000);
}

function restartExpiry() {
  if (expiryTimer) clearTimeout(expiryTimer);
  if (!agent.bubble) return;
  expiresAt.value = Date.now() + bubbleDisplayDurationMs(pages.value);
  expiryTimer = setTimeout(() => {
    now.value = Date.now();
    void syncVisibility();
  }, Math.max(0, expiresAt.value - Date.now()) + 1);
}

watch(() => agent.bubble?.id, () => {
  now.value = Date.now();
  restartRotation();
  restartExpiry();
  void syncVisibility();
});
watch(() => agent.characterId, () => void syncVisibility());

onBeforeUnmount(() => {
  visibilityRequests.invalidate();
  if (rotation) clearInterval(rotation);
  if (transitionTimer) clearTimeout(transitionTimer);
  if (expiryTimer) clearTimeout(expiryTimer);
  unlistenDragStart?.();
  unlistenDragEnd?.();
});

onMounted(async () => {
  restartRotation();
  restartExpiry();
  void syncVisibility();
  unlistenDragStart = await listen("pet:drag-started", async () => {
    dragging.value = true;
    visibilityRequests.invalidate();
    await invoke("pet_bubble_hide");
  });
  unlistenDragEnd = await listen("pet:drag-ended", () => {
    dragging.value = false;
    void syncVisibility();
  });
});
</script>

<template>
  <main class="pet-bubble" :class="[direction, { fading, appearing }]" :style="{ '--pet-accent': accent }">
    <p v-for="line in currentLines" :key="line">{{ line }}</p>
  </main>
</template>

<style scoped>
.pet-bubble {
  width: 100%; min-height: 100%; box-sizing: border-box;
  padding: 11px 15px 12px;
  border: 1px solid color-mix(in srgb, var(--pet-accent, #8aa68d) 40%, white);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.77);
  box-shadow: 0 8px 24px rgba(18, 32, 24, 0.17);
  backdrop-filter: blur(18px) saturate(118%);
  color: #183226;
  opacity: 1;
  transition: opacity 180ms ease;
  overflow: hidden;
}
.pet-bubble::after {
  content: ""; position: absolute; left: 50%; bottom: -7px; width: 13px; height: 13px;
  background: rgba(255, 255, 255, 0.77); border-right: 1px solid color-mix(in srgb, var(--pet-accent, #8aa68d) 40%, white);
  border-bottom: 1px solid color-mix(in srgb, var(--pet-accent, #8aa68d) 40%, white); transform: translateX(-50%) rotate(45deg);
}
.pet-bubble.below::after { top: -7px; bottom: auto; transform: translateX(-50%) rotate(225deg); }
.pet-bubble.left::after { left: auto; right: -7px; top: 50%; bottom: auto; transform: translateY(-50%) rotate(-45deg); }
.pet-bubble.right::after { left: -7px; top: 50%; bottom: auto; transform: translateY(-50%) rotate(135deg); }
.pet-bubble.above-left::after { left: 24%; }
.pet-bubble.above-right::after { left: 76%; }
.pet-bubble.fading, .pet-bubble.appearing { opacity: 0; }
p { position: relative; z-index: 1; margin: 0; min-height: 1.4em; font: 600 14px/1.4 "Segoe UI Variable", "Microsoft YaHei UI", sans-serif; letter-spacing: 0; white-space: pre-wrap; overflow-wrap: anywhere; }
</style>
