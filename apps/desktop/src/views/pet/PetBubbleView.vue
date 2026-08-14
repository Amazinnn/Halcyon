<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  acceptBubbleDelivery,
  type BubbleDelivery,
  BubbleVisibilityRequest,
  bubbleDisplayDurationMs,
  bubbleShouldBeVisible,
  nextBubblePage,
  paginateBubbleTextMeasured,
} from "../../lib/pet-bubble";

type LocalBubble = BubbleDelivery & { id: number };

const currentAgentId = ref("");
const bubble = ref<LocalBubble | null>(null);
const seenDeliveryIds = ref<string[]>([]);
let bubbleSequence = 0;
const page = ref(0);
const fading = ref(false);
const appearing = ref(false);
let rotation: ReturnType<typeof setInterval> | null = null;
let transitionTimer: ReturnType<typeof setTimeout> | null = null;
let expiryTimer: ReturnType<typeof setTimeout> | null = null;
let unlistenDragStart: (() => void) | null = null;
let unlistenDragEnd: (() => void) | null = null;
let unlistenBubbleDelivery: (() => void) | null = null;
let unlistenPetChanged: (() => void) | null = null;
const now = ref(Date.now());
const expiresAt = ref(0);
const dragging = ref(false);
const direction = ref("above");
const visibilityRequests = new BubbleVisibilityRequest();
let hostGeneration = 0;

function measureBubbleText(text: string): number {
  const context = document.createElement("canvas").getContext("2d");
  if (!context) return text.length * 14;
  context.font = '600 14px "Segoe UI Variable", "Microsoft YaHei UI", sans-serif';
  return context.measureText(text).width;
}

const pages = computed(() => paginateBubbleTextMeasured(bubble.value?.text ?? "", measureBubbleText, 214));
const visible = computed(() => bubbleShouldBeVisible({
  hasMessage: Boolean(bubble.value),
  dragging: dragging.value,
  now: now.value,
  expiresAt: expiresAt.value,
}));
const currentLines = computed(() => pages.value[page.value] ?? []);

async function syncVisibility() {
  visibilityRequests.issue();
  if (!visible.value) {
    await invoke("pet_bubble_hide");
  }
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
  if (!bubble.value) return;
  expiresAt.value = Date.now() + bubbleDisplayDurationMs(pages.value);
  expiryTimer = setTimeout(() => {
    now.value = Date.now();
    void syncVisibility();
  }, Math.max(0, expiresAt.value - Date.now()) + 1);
}

async function receiveDelivery(delivery: BubbleDelivery) {
  const accepted = acceptBubbleDelivery(currentAgentId.value, seenDeliveryIds.value, delivery);
  if (!accepted.message) return;
  seenDeliveryIds.value = accepted.seenDeliveryIds;
  bubble.value = { ...accepted.message, id: ++bubbleSequence };
  await nextTick();
  if (!currentAgentId.value || delivery.agentId !== currentAgentId.value) return;
  const shown = await invoke<boolean>("pet_bubble_rendered", {
    characterId: currentAgentId.value,
    generation: hostGeneration,
    deliveryId: delivery.deliveryId,
  });
  if (shown) {
    appearing.value = true;
    requestAnimationFrame(() => { appearing.value = false; });
  }
}

async function registerBubbleHost() {
  if (!currentAgentId.value) return;
  const pending = await invoke<BubbleDelivery | null>("pet_bubble_ready", {
    characterId: currentAgentId.value,
    generation: hostGeneration,
  });
  if (pending) await receiveDelivery(pending);
}

async function refreshCurrentAgent() {
  const bootstrap = await invoke<{ currentAgentId?: string | null }>("get_bootstrap");
  const nextAgentId = bootstrap.currentAgentId ?? "";
  if (nextAgentId !== currentAgentId.value) {
    currentAgentId.value = nextAgentId;
    bubble.value = null;
    seenDeliveryIds.value = [];
  }
  hostGeneration += 1;
  await registerBubbleHost();
}

watch(() => bubble.value?.id, () => {
  now.value = Date.now();
  restartRotation();
  restartExpiry();
  void syncVisibility();
});
watch(currentAgentId, () => void syncVisibility());

onBeforeUnmount(() => {
  visibilityRequests.invalidate();
  if (rotation) clearInterval(rotation);
  if (transitionTimer) clearTimeout(transitionTimer);
  if (expiryTimer) clearTimeout(expiryTimer);
  unlistenDragStart?.();
  unlistenDragEnd?.();
  unlistenBubbleDelivery?.();
  unlistenPetChanged?.();
});

onMounted(async () => {
  // Register before host readiness; the native controller retains the envelope
  // until this window confirms it rendered the matching delivery.
  unlistenBubbleDelivery = await listen<BubbleDelivery>("bubble:deliver", (event) => {
    void receiveDelivery(event.payload);
  });
  unlistenPetChanged = await listen("pet:changed", () => {
    void refreshCurrentAgent();
  });
  await refreshCurrentAgent();
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
  <main class="pet-bubble" :class="[direction, { fading, appearing }]">
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
