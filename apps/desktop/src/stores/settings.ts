import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { DEFAULT_FOCUS_MODE, normalizeFocusMode, type FocusMode } from "../lib/focus-mode";

/** Focus durations and blacklist-only application reminders. */
export const useSettingsStore = defineStore("settings", {
  state: () => ({
    loaded: false,
    focusMinutes: 25,
    restMinutes: 5,
    distractionApps: [] as string[],
    soundEnabled: true,
    showTopbar: "auto" as "auto" | "on" | "off",
    petBgFade: true,
    chatStreamingEnabled: false,
    currentAgentId: null as string | null,
    focusMode: DEFAULT_FOCUS_MODE as FocusMode,
  }),
  actions: {
    async load() {
      const b = await invoke<{
        focusMinutes?: number;
        restMinutes?: number;
        distractionApps?: string[];
        soundEnabled?: boolean;
        showTopbar?: string;
        petBgFade?: boolean;
        chatStreamingEnabled?: boolean;
        currentAgentId?: string | null;
        focusMode?: string;
      }>("get_bootstrap");
      this.focusMinutes = b.focusMinutes ?? 25;
      this.restMinutes = b.restMinutes ?? 5;
      this.distractionApps = b.distractionApps ?? [];
      this.soundEnabled = !!b.soundEnabled;
      this.showTopbar = (b.showTopbar as "auto" | "on" | "off") ?? "auto";
      this.petBgFade = !!b.petBgFade;
      this.chatStreamingEnabled = !!b.chatStreamingEnabled;
      this.currentAgentId = b.currentAgentId ?? null;
      this.focusMode = normalizeFocusMode(b.focusMode);
      this.loaded = true;
    },
    async setFocusDurations(focus: number, rest: number) {
      await invoke("set_focus_durations", { focus, rest });
      this.focusMinutes = focus;
      this.restMinutes = rest;
    },
    async setFocusMode(mode: FocusMode) {
      const previous = this.focusMode;
      this.focusMode = mode;
      try {
        await invoke("set_focus_mode", { mode });
      } catch (error) {
        this.focusMode = previous;
        throw error;
      }
    },
    async setSound(enabled: boolean) {
      await invoke("set_sound_enabled", { enabled });
      this.soundEnabled = enabled;
    },
    async setPetBgFade(enabled: boolean) {
      await invoke("set_pet_bg_fade", { enabled });
      this.petBgFade = enabled;
    },
    async setChatStreamingEnabled(enabled: boolean) {
      await invoke("set_chat_streaming_enabled", { enabled });
      this.chatStreamingEnabled = enabled;
    },
    async setShowTopbar(mode: "auto" | "on" | "off") {
      await invoke("set_show_topbar", { mode });
      this.showTopbar = mode;
    },
    async setDistractionLists(black: string[]) {
      await invoke("set_distraction_lists", { black });
      this.distractionApps = black;
    },
  },
});
