import { defineStore } from "pinia";
import { listen } from "@tauri-apps/api/event";

// Slim UI coordinator (v1.2): chat visibility drives the pet bubble mute rule.
export const useUiStore = defineStore("ui", {
  state: () => ({
    chatOpen: true,
    doNotDisturb: false,
    lockActive: false,
  }),
  actions: {
    async init() {
      await listen("window:visibility", (e) => {
        const p = e.payload as { label: string; visible: boolean };
        if (p.label === "chat") this.chatOpen = p.visible;
      });
    },
  },
});