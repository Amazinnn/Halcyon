import { defineStore } from "pinia";
import { listen, emit } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useSettingsStore } from "./settings";
import { playChime } from "../lib/sound";

export type FocusState = "idle" | "focus" | "rest";
export type SupervisionStatus = "ok" | "drift" | "paused";

/**
 * UI coordinator + pomodoro timer (v1.4 / v1.4.1): focus/rest both count down;
 * focus reaching 0 auto-enters rest, rest reaching 0 stops and waits for the
 * user; pause freezes the timer, skip advances the phase. Phase transitions
 * are reported to the Rust supervision engine via `focus:state_changed`.
 *
 * v1.4.1: durations/sound/topbar config are NOT duplicated here - they are
 * getters delegating to the settings store (single source of truth), so
 * changes made in the settings popover take effect immediately. A `focus:tick`
 * event is broadcast every second so the always-on-top status capsule
 * (topbar window) can mirror the live countdown.
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
    todayFocusSec: 0,
    todayRounds: 0,
    supervisionStatus: "ok" as SupervisionStatus,
    focusSubtitle: "保持节奏，阳光会照到每一片叶子",
    _ticker: null as number | null,
    // v1.11.1: set while a workflow focus/idle node drives the timer, so its
    // countdown ending must NOT fire the focus_end workflow trigger (that
    // would cascade workflow runs unexpectedly).
    workflowDriven: false,
  }),
  getters: {
    focusMinutes(): number {
      return useSettingsStore().focusMinutes;
    },
    restMinutes(): number {
      return useSettingsStore().restMinutes;
    },
    soundEnabled(): boolean {
      return useSettingsStore().soundEnabled;
    },
    showTopbar(): "auto" | "on" | "off" {
      return useSettingsStore().showTopbar;
    },
  },
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
      // Workflow node system actions (v1.10.4 #51): focus/idle/ring nodes of a
      // workflow drive the timer/sound from the Rust engine via the event bus.
      await listen<{ action: string; seconds: number }>("workflow:system-action", (e) => {
        if (getCurrentWebviewWindow().label !== "desktop") return;
        const { action, seconds } = e.payload;
        if (action === "focus") {
          // v1.11.1: mark the timer as workflow-driven so its countdown end
          // never fires the focus_end workflow trigger.
          this.workflowDriven = true;
          this.startFocusFor(Math.max(1, seconds));
        } else if (action === "idle") {
          this.workflowDriven = true;
          this.startRestFor(Math.max(1, seconds));
        } else if (action === "ring") {
          // v1.11.1: ring once — the engine blocks for the duration.
          this.ringFor(Math.max(1, seconds));
        }
      });
      // Agent CLI control plane (v1.5): `focus-cli timer ...` routes through
      // the desktop webview, which runs the action and replies with live state.
      await listen<{ id: number; action: string }>("cli:timer", (e) => {
        if (getCurrentWebviewWindow().label !== "desktop") return;
        const { id, action } = e.payload;
        if (action === "start") this.startFocus();
        else if (action === "pause") this.pause();
        else if (action === "skip") this.skip();
        void emit("cli:timer-done", {
          id,
          state: this.focusState,
          focusRemainingSec: this.focusRemainingSec,
          restRemainingSec: this.restRemainingSec,
          paused: this.timerPaused,
          phaseDone: this.phaseDone,
        });
      });
    },
    applyConfig(cfg: { focusSubtitle?: string }) {
      if (cfg.focusSubtitle) this.focusSubtitle = cfg.focusSubtitle;
    },
    emitTick() {
      void emit("focus:tick", {
        state: this.focusState,
        focusRemainingSec: this.focusRemainingSec,
        restRemainingSec: this.restRemainingSec,
        paused: this.timerPaused,
        phaseDone: this.phaseDone,
      });
    },
    startFocus() {
      this.workflowDriven = false;
      this.stopTicker();
      this.focusState = "focus";
      this.timerPaused = false;
      this.phaseDone = false;
      this.focusRemainingSec = this.focusMinutes * 60;
      void emit("focus:state_changed", { state: "focus" });
      this.emitTick();
      this._ticker = window.setInterval(() => this.tick(), 1000);
    },
    /** v1.10.4 (#51): focus for a custom number of seconds (workflow focus node). */
    startFocusFor(seconds: number) {
      this.stopTicker();
      this.focusState = "focus";
      this.timerPaused = false;
      this.phaseDone = false;
      this.focusRemainingSec = seconds;
      void emit("focus:state_changed", { state: "focus" });
      this.emitTick();
      this._ticker = window.setInterval(() => this.tick(), 1000);
    },
    /** v1.10.4 (#51): idle/rest for a custom number of seconds (workflow idle node). */
    startRestFor(seconds: number) {
      this.stopTicker();
      this.focusState = "rest";
      this.timerPaused = false;
      this.phaseDone = false;
      this.restRemainingSec = seconds;
      void emit("focus:state_changed", { state: "rest", completed: false });
      this.emitTick();
      this._ticker = window.setInterval(() => this.tick(), 1000);
    },
    /** v1.10.4 (#51): ring N times, once per second (workflow ring node).
     *  v1.11.1: the engine now blocks for the ring duration, so this only
     *  rings once per ring node execution — no setTimeout stacking. */
    ringFor(seconds: number) {
      if (this.soundEnabled) playChime();
      void seconds;
    },
    startRest(completed: boolean) {
      this.workflowDriven = false;
      this.stopTicker();
      this.focusState = "rest";
      this.timerPaused = false;
      this.phaseDone = false;
      this.restRemainingSec = this.restMinutes * 60;
      void emit("focus:state_changed", { state: "rest", completed });
      this.emitTick();
      this._ticker = window.setInterval(() => this.tick(), 1000);
    },
    tick() {
      if (this.timerPaused) return;
      if (this.focusState === "focus") {
        this.focusRemainingSec--;
        if (this.focusRemainingSec <= 0) {
          this.onPhaseChime();
          window.setTimeout(() => void this.loadTodaySummary(), 600);
          // v1.11.1: a workflow-driven focus countdown ending restarts the
          // timer state but must not fire the focus_end workflow trigger.
          this.startRest(this.workflowDriven ? false : true);
          return;
        }
        this.emitTick();
      } else if (this.focusState === "rest") {
        this.restRemainingSec--;
        if (this.restRemainingSec <= 0) {
          this.stopTicker();
          this.restRemainingSec = 0;
          this.phaseDone = true;
          this.onPhaseChime();
        }
        this.emitTick();
      }
    },
    onPhaseChime() {
      if (this.soundEnabled) playChime();
    },
    pause() {
      if (this.focusState === "idle" || this.phaseDone) return;
      this.timerPaused = !this.timerPaused;
      void emit("focus:state_changed", { state: this.focusState, paused: this.timerPaused });
      this.emitTick();
    },
    skip() {
      if (this.focusState === "focus") {
        this.startRest(true); // v1.8.2: skipped focus still records elapsed focus time
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
