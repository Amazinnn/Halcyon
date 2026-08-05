import { defineStore } from "pinia";
import { emit, listen } from "@tauri-apps/api/event";

// UiState per design doc §8: single UiCoordinator manages panel mode + flags.
export type PanelMode =
  | "closed"
  | "chat"
  | "statistics"
  | "task"
  | "permission"
  | "diff"
  | "collapsed";

export const useUiStore = defineStore("ui", {
  state: () => ({
    panelMode: "closed" as PanelMode,
    petVisible: true,
    speechBubbleVisible: false,
    doNotDisturb: false,
    lockActive: false,
  }),
  getters: {
    chatOpen: (s) => s.panelMode === "chat",
  },
  actions: {
    async init() {
      await listen<{ mode: PanelMode }>("panel:mode_changed", (e) => {
        this.panelMode = e.payload.mode;
      });
    },
    setPanelMode(mode: PanelMode) {
      this.panelMode = mode;
      void emit("ui:panel_mode_changed", { mode });
    },
    setSpeechBubble(visible: boolean) {
      this.speechBubbleVisible = visible;
    },
    togglePanel() {
      // Pet click: Rust toggles the panel window and its visibility.
      void emit("ui:toggle_panel", {});
    },
  },
});