<script setup lang="ts">
import { computed, onMounted } from "vue";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import DesktopView from "./views/desktop/DesktopView.vue";
import PetView from "./views/pet/PetView.vue";
import PanelView from "./views/panel/PanelView.vue";
import MusicView from "./views/music/MusicView.vue";
import { useUiStore } from "./stores/ui";
import { useAgentStore } from "./stores/agent";

const label = getCurrentWebviewWindow().label;

const view = computed(() => {
  switch (label) {
    case "desktop":
      return DesktopView;
    case "pet":
      return PetView;
    case "panel":
      return PanelView;
    case "music":
      return MusicView;
    default:
      return DesktopView;
  }
});

onMounted(() => {
  // Transparent windows: make the webview background transparent.
  if (label === "pet" || label === "music") {
    document.documentElement.classList.add("transparent-window");
    document.body.classList.add("transparent-window");
  }
  // Each window subscribes to the shared Rust event bus (independent Pinia state).
  void useUiStore().init();
  void useAgentStore().init();
});
</script>

<template>
  <component :is="view" />
</template>