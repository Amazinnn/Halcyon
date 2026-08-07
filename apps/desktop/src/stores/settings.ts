import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

export interface FocusTask {
  id: string;
  name: string;
  estimatedMinutes?: number | null;
  boundApp?: string | null;
}

/** v1.4 settings store: task, pomodoro durations, supervision lists, toggles. */
export const useSettingsStore = defineStore("settings", {
  state: () => ({
    loaded: false,
    tasks: [] as FocusTask[],
    currentTaskId: null as string | null,
    focusMinutes: 25,
    restMinutes: 5,
    distractionApps: [] as string[],
    allowedApps: [] as string[],
    supervisionEnabled: true,
    supervisionPaused: false,
    soundEnabled: true,
    showTopbar: "auto" as "auto" | "on" | "off",
    petBgFade: true,
  }),
  getters: {
    currentTask(state): FocusTask | null {
      return state.tasks.find((t) => t.id === state.currentTaskId) ?? null;
    },
  },
  actions: {
    async load() {
      const b = await invoke<{
        tasks?: FocusTask[];
        currentTaskId?: string | null;
        focusMinutes?: number;
        restMinutes?: number;
        distractionApps?: string[];
        allowedApps?: string[];
        supervisionEnabled?: boolean;
        supervisionPauseUntil?: number | null;
        soundEnabled?: boolean;
        showTopbar?: string;
        petBgFade?: boolean;
      }>("get_bootstrap");
      this.tasks = b.tasks ?? [];
      this.currentTaskId = b.currentTaskId ?? null;
      this.focusMinutes = b.focusMinutes ?? 25;
      this.restMinutes = b.restMinutes ?? 5;
      this.distractionApps = b.distractionApps ?? [];
      this.allowedApps = b.allowedApps ?? [];
      this.supervisionEnabled = !!b.supervisionEnabled;
      const pu = b.supervisionPauseUntil;
      this.supervisionPaused = typeof pu === "number" && pu > Date.now() / 1000;
      this.soundEnabled = !!b.soundEnabled;
      this.showTopbar = (b.showTopbar as "auto" | "on" | "off") ?? "auto";
      this.petBgFade = !!b.petBgFade;
      this.loaded = true;
    },
    async setFocusDurations(focus: number, rest: number) {
      await invoke("set_focus_durations", { focus, rest });
      this.focusMinutes = focus;
      this.restMinutes = rest;
    },
    async setSound(enabled: boolean) {
      await invoke("set_sound_enabled", { enabled });
      this.soundEnabled = enabled;
    },
    async setPetBgFade(enabled: boolean) {
      await invoke("set_pet_bg_fade", { enabled });
      this.petBgFade = enabled;
    },
    async setShowTopbar(mode: "auto" | "on" | "off") {
      await invoke("set_show_topbar", { mode });
      this.showTopbar = mode;
    },
    async setSupervisionEnabled(enabled: boolean) {
      await invoke("set_supervision_enabled", { enabled });
      this.supervisionEnabled = enabled;
    },
    async pauseSupervision(minutes: number) {
      await invoke("set_supervision_paused", { minutes });
      this.supervisionPaused = true;
    },
    async resumeSupervision() {
      await invoke("resume_supervision");
      this.supervisionPaused = false;
    },
    async setDistractionLists(black: string[], white: string[]) {
      await invoke("set_distraction_lists", { black, white });
      this.distractionApps = black;
      this.allowedApps = white;
    },
    async saveTask(task: FocusTask) {
      const saved = await invoke<FocusTask>("save_task", { task });
      const i = this.tasks.findIndex((t) => t.id === saved.id);
      if (i >= 0) this.tasks[i] = saved;
      else this.tasks.push(saved);
      return saved;
    },
    async setCurrentTask(id: string | null) {
      await invoke("set_current_task", { id });
      this.currentTaskId = id;
    },
  },
});
