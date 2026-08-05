import { defineStore } from "pinia";
import { listen, emit } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "./settings";
import { playChime } from "../lib/sound";

export type FocusState = "idle" | "focus" | "rest";
export type SupervisionStatus = "ok" | "drift" | "paused";

/**
 * UI coordinator + pomodoro timer (v1.4): focus/rest both count down; focus
 * reaching 0 auto-enters rest, rest reaching 0 stops and waits for the user;
 * pause freezes the timer, skip advances the phase. Phase transitions are
 * reported to the Rust supervision engine via `focus:state_changed`.
 */
export const useUiStore = defineStore("ui", {
  state: () => ({
    chatOpen: true,
    doNotDisturb: false,
    lockActive: false,
    focusState: "idle" as FocusState,
    focusRemainingSec: 0,
    restRemainingSec: 0,
    timerPaused: false,
    phaseDone: false,
    focusMinutes: 25,
    restMinutes: 5,
    soundEnabled: true,
    showTopbar: "auto" as "auto" | "on" | "off",
    todayFocusSec: 0,
    todayRounds: 0,
    supervisionStatus: "ok" as SupervisionStatus,
    focusSubtitle: "保持节奏，阳光会照到每一片叶子",
    _ticker: null as number | null,
  }),
  actions: {
    async init() {
      await listen("window:visibility", (e) => {
        const p = e.payload as { label: string; visible: boolean };
        if (p.label === "chat") this.chatOpen = p.visible;
      });
      await listen("supervision:status", (e) => {
        const st = (e.payload as { status?: string })?.status;
        if (st === "drift" || st === "paused" || st === "ok") this.supervisionStatus = st;
      });
    },
    applyConfig(cfg: {
      focusMinutes?: number;
      restMinutes?: number;
      soundEnabled?: boolean;
      showTopbar?: string;
      focusSubtitle?: string;
    }) {
      if (typeof cfg.focusMinutes === "number") this.focusMinutes = cfg.focusMinutes;
      if (typeof cfg.restMinutes === "number") this.restMinutes = cfg.restMinutes;
      if (typeof cfg.soundEnabled === "boolean") this.soundEnabled = cfg.soundEnabled;
      if (cfg.showTopbar === "auto" || cfg.showTopbar === "on" || cfg.showTopbar === "off") {
        this.showTopbar = cfg.showTopbar;
      }
      if (cfg.focusSubtitle) this.focusSubtitle = cfg.focusSubtitle;
    },
    startFocus() {
      this.stopTicker();
      this.focusState = "focus";
      this.timerPaused = false;
      this.phaseDone = false;
      this.focusRemainingSec = this.focusMinutes * 60;
      void emit("focus:state_changed", { state: "focus" });
      this._ticker = window.setInterval(() => this.tick(), 1000);
    },
    startRest(completed: boolean) {
      this.stopTicker();
      this.focusState = "rest";
      this.timerPaused = false;
      this.phaseDone = false;
      this.restRemainingSec = this.restMinutes * 60;
      void emit("focus:state_changed", { state: "rest", completed });
      this._ticker = window.setInterval(() => this.tick(), 1000);
    },
    tick() {
      if (this.timerPaused) return;
      if (this.focusState === "focus") {
        this.focusRemainingSec--;
        if (this.focusRemainingSec <= 0) {
          this.onPhaseChime();
          window.setTimeout(() => void this.loadTodaySummary(), 600);
          this.startRest(true);
        }
      } else if (this.focusState === "rest") {
        this.restRemainingSec--;
        if (this.restRemainingSec <= 0) {
          this.stopTicker();
          this.restRemainingSec = 0;
          this.phaseDone = true;
          this.onPhaseChime();
        }
      }
    },
    onPhaseChime() {
      const settings = useSettingsStore();
      if (settings.soundEnabled || this.soundEnabled) playChime();
    },
    pause() {
      if (this.focusState === "idle" || this.phaseDone) return;
      this.timerPaused = !this.timerPaused;
      void emit("focus:state_changed", { state: this.focusState, paused: this.timerPaused });
    },
    skip() {
      if (this.focusState === "focus") {
        this.startRest(false); // skipped focus -> rest (no session recorded)
      } else {
        this.startFocus(); // idle or rest -> focus
      }
    },
    toggleFocus() {
      if (this.focusState === "idle") this.startFocus();
      else if (this.focusState === "focus") this.pause();
      else this.startFocus(); // rest (done or not) -> back to focus
    },
    stopTicker() {
      if (this._ticker !== null) {
        window.clearInterval(this._ticker);
        this._ticker = null;
      }
    },
    async loadTodaySummary() {
      try {
        const [total, rounds] = await invoke<[number, number]>("get_today_focus_summary");
        this.todayFocusSec = total;
        this.todayRounds = rounds;
      } catch {
        /* ignore */
      }
    },
  },
});
