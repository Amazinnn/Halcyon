<script setup lang="ts">
import { computed, onMounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import DesktopView from "./views/desktop/DesktopView.vue";
import ChatView from "./views/chat/ChatView.vue";
import StatsView from "./views/stats/StatsView.vue";
import MusicView from "./views/music/MusicView.vue";
import WorkflowView from "./views/workflow/WorkflowView.vue";
import PetView from "./views/pet/PetView.vue";
import TopbarView from "./views/topbar/TopbarView.vue";
import GridOverlayView from "./views/overlay/GridOverlayView.vue";
import { useUiStore } from "./stores/ui";
import { useSettingsStore } from "./stores/settings";
import { useAgentStore } from "./stores/agent";
import { playChime } from "./lib/sound";

const label = getCurrentWebviewWindow().label;

const view = computed(() => {
  switch (label) {
    case "desktop": return DesktopView;
    case "chat": return ChatView;
    case "stats": return StatsView;
    case "music": return MusicView;
    case "workflow": return WorkflowView;
    case "pet": return PetView;
    case "topbar": return TopbarView;
    case "grid-overlay": return GridOverlayView;
    default: return DesktopView;
  }
});

onMounted(() => {
  if (["pet", "music", "topbar", "chat", "stats", "workflow", "grid-overlay"].includes(label)) {
    document.documentElement.classList.add("transparent-window");
    document.body.classList.add("transparent-window");
  }
  void useUiStore().init();
  void useAgentStore().init();
  void listen("supervision:alert", (e) => {
    const p = (e.payload ?? {}) as { text?: string };
    useAgentStore().showBubble(p.text ?? "注意保持专注", "high");
    if (useSettingsStore().soundEnabled) playChime();
  });
});
</script>

<template>
  <component :is="view" />
</template>
