<script setup lang="ts">
import { computed, onMounted } from "vue";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import DesktopView from "./views/desktop/DesktopView.vue";
import ChatView from "./views/chat/ChatView.vue";
import StatsView from "./views/stats/StatsView.vue";
import MusicView from "./views/music/MusicView.vue";
import PetView from "./views/pet/PetView.vue";
import LogosView from "./views/logos/LogosView.vue";
import GridOverlayView from "./views/overlay/GridOverlayView.vue";
import { useUiStore } from "./stores/ui";
import { useAgentStore } from "./stores/agent";

const label = getCurrentWebviewWindow().label;

const view = computed(() => {
  switch (label) {
    case "desktop": return DesktopView;
    case "chat": return ChatView;
    case "stats": return StatsView;
    case "music": return MusicView;
    case "pet": return PetView;
    case "logos": return LogosView;
    case "grid-overlay": return GridOverlayView;
    default: return DesktopView;
  }
});

onMounted(() => {
  if (["pet", "music", "logos", "chat", "stats", "grid-overlay"].includes(label)) {
    document.documentElement.classList.add("transparent-window");
    document.body.classList.add("transparent-window");
  }
  void useUiStore().init();
  void useAgentStore().init();
});
</script>

<template>
  <component :is="view" />
</template>