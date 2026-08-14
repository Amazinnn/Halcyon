<script setup lang="ts">
import { computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { viewForLabel, isTransparentLabel } from "./lib/view-registry";
import { useUiStore } from "./stores/ui";
import { useSettingsStore } from "./stores/settings";
import { useAgentStore } from "./stores/agent";
import { playChime } from "./lib/sound";

const label = getCurrentWebviewWindow().label;

const view = computed(() => viewForLabel(label));

// Thin agent windows (extensibility plan C3): light windows subscribe to the
// minimum event set instead of initializing the full Agent store.
const THIN_AGENT_LABELS = new Set(["topbar", "pet-bubble", "grid-overlay"]);

onMounted(() => {
  if (isTransparentLabel(label)) {
    document.documentElement.classList.add("transparent-window");
    document.body.classList.add("transparent-window");
  }
  void useUiStore().init();
  void useAgentStore().init({ thin: THIN_AGENT_LABELS.has(label) });
  window.addEventListener("pointerdown", () => {
    void invoke("drag_diagnostic_browser_event", {
      label: "pet",
      stage: `browser:post-release-first-click:${label}`,
      sequence: null,
    }).catch(() => undefined);
  }, { capture: true, once: false });
  void listen("supervision:alert", () => {
    if (useSettingsStore().soundEnabled) playChime();
  });
});
</script>

<template>
  <component :is="view" />
</template>
