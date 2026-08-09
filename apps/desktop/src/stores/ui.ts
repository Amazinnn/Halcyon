import { defineStore } from "pinia";
import { listen, emit } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useSettingsStore } from "./settings";
import { playChime } from "../lib/sound";
import { createFocusLockQueue, createSerialActionQueue } from "../lib/focus-lock-queue";
import { desktopLockForFocus, type DesktopLockMode, type FocusMode } from "../lib/focus-mode";

const desktopLockQueue = createFocusLockQueue(async (mode) => {
  await invoke("desktop_set_focus_lock", { mode });
});
const focusTransitionQueue = createSerialActionQueue();

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
    desktopLockError: "",
    desktopLockTransitionPending: false,
    activeFocusMode: null as FocusMode | null,
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
    async syncDesktopLock(mode: DesktopLockMode): Promise<boolean> {
      try {
        await desktopLockQueue.request(mode);
        this.desktopLockError = "";
        return true;
      } catch (error) {
        this.desktopLockError = mode !== "none"
          ? `专注桌面锁定失败：${String(error)}`
          : `桌面恢复失败：${String(error)}`;
        return false;
      }
    },
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
          void this.startFocusFor(Math.max(1, seconds), true);
        } else if (action === "idle") {
          void this.startRestFor(Math.max(1, seconds), true);
        } else if (action === "ring") {
          // v1.11.1: ring once — the engine blocks for the duration.
          this.ringFor(Math.max(1, seconds));
        }
      });
      // Agent CLI control plane (v1.5): `focus-cli timer ...` routes through
      // the desktop webview, which runs the action and replies with live state.
      await listen<{ id: number; action: string }>("cli:timer", async (e) => {
        if (getCurrentWebviewWindow().label !== "desktop") return;
        const { id, action } = e.payload;
        if (action === "start") await this.startFocus();
        else if (action === "pause") await this.pause();
        else if (action === "skip") await this.skip();
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
    async unlockBeforeStateChange(): Promise<boolean> {
      if (this.desktopLockTransitionPending) return false;
      this.desktopLockTransitionPending = true;
      try {
        return await this.syncDesktopLock("none");
      } finally {
        this.desktopLockTransitionPending = false;
      }
    },
    async startFocus() {
      return focusTransitionQueue.request(async () => {
        this.workflowDriven = false;
        return this.runStartFocusFor(this.focusMinutes * 60);
      });
      // v1.12.2: focus start locks the desktop (taskbar/icons hidden, keys
      // blocked). Lock failure must NOT block focusing — warn only.
    },
    /** v1.10.4 (#51): focus for a custom number of seconds (workflow focus node). */
    async startFocusFor(seconds: number, workflowDriven = false) {
      return focusTransitionQueue.request(() => this.runStartFocusFor(seconds, workflowDriven));
    },
    async runStartFocusFor(seconds: number, workflowDriven = false) {
      const mode = useSettingsStore().focusMode;
      // Freeze any previous phase before awaiting the native lock. A final
      // tick from the old round must not enqueue a stale rest transition
      // behind this freshly requested focus round.
      const wasTicking = this._ticker !== null;
      this.stopTicker();
      if (!(await this.syncDesktopLock(desktopLockForFocus(mode)))) {
        if (wasTicking && !this.timerPaused && (this.focusState !== "focus" || this.focusRemainingSec > 0)) {
          this._ticker = window.setInterval(() => this.tick(), 1000);
        }
        return;
      }
      this.workflowDriven = workflowDriven;
      this.activeFocusMode = mode;
      this.focusState = "focus";
      this.timerPaused = false;
      this.phaseDone = false;
      this.focusRemainingSec = seconds;
      void emit("focus:state_changed", { state: "focus" });
      this.emitTick();
      this._ticker = window.setInterval(() => this.tick(), 1000);
    },
    /** v1.10.4 (#51): idle/rest for a custom number of seconds (workflow idle node). */
    async startRestFor(seconds: number, workflowDriven = false): Promise<boolean> {
      return focusTransitionQueue.request(() => this.runStartRestFor(seconds, workflowDriven));
    },
    async runStartRestFor(seconds: number, workflowDriven = false): Promise<boolean> {
      const wasTicking = this._ticker !== null;
      this.stopTicker();
      if (!(await this.unlockBeforeStateChange())) {
        if (wasTicking && (this.focusState !== "focus" || this.focusRemainingSec > 0)) {
          this._ticker = window.setInterval(() => this.tick(), 1000);
        }
        return false;
      }
      this.workflowDriven = workflowDriven;
      this.activeFocusMode = null;
      this.focusState = "rest";
      this.timerPaused = false;
      this.phaseDone = false;
      this.restRemainingSec = seconds;
      void emit("focus:state_changed", { state: "rest", completed: false });
      this.emitTick();
      this._ticker = window.setInterval(() => this.tick(), 1000);
      return true;
    },
    /** v1.10.4 (#51): ring N times, once per second (workflow ring node).
     *  v1.11.1: the engine now blocks for the ring duration, so this only
     *  rings once per ring node execution — no setTimeout stacking. */
    ringFor(seconds: number) {
      if (this.soundEnabled) playChime();
      void seconds;
    },
    async startRest(completed: boolean): Promise<boolean> {
      return focusTransitionQueue.request(() => this.runStartRest(completed));
    },
    async runStartRest(completed: boolean): Promise<boolean> {
      const wasTicking = this._ticker !== null;
      this.stopTicker();
      if (!(await this.unlockBeforeStateChange())) {
        if (wasTicking && (this.focusState !== "focus" || this.focusRemainingSec > 0)) {
          this._ticker = window.setInterval(() => this.tick(), 1000);
        }
        return false;
      }
      this.workflowDriven = false;
      this.activeFocusMode = null;
      this.focusState = "rest";
      this.timerPaused = false;
      this.phaseDone = false;
      this.restRemainingSec = this.restMinutes * 60;
      void emit("focus:state_changed", { state: "rest", completed });
      this.emitTick();
      this._ticker = window.setInterval(() => this.tick(), 1000);
      return true;
    },
    tick() {
      if (this.timerPaused) return;
      if (this.focusState === "focus") {
        this.focusRemainingSec--;
        if (this.focusRemainingSec <= 0) {
          this.onPhaseChime();
          // v1.12.2: focus round naturally ends → unlock the desktop.
          window.setTimeout(() => void this.loadTodaySummary(), 600);
          // v1.11.1: a workflow-driven focus countdown ending restarts the
          // timer state but must not fire the focus_end workflow trigger.
          void this.startRest(this.workflowDriven ? false : true);
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
    async pause() {
      return focusTransitionQueue.request(() => this.runPauseTransition());
    },
    async runPauseTransition() {
      if (this.focusState === "idle" || this.phaseDone) return;
      if (this.focusState === "focus" && !this.timerPaused) {
        // Freeze the countdown before awaiting the native unlock. Otherwise a
        // final tick can race this pause and attempt to enter rest while the
        // pause action still owns the transition.
        const wasTicking = this._ticker !== null;
        this.stopTicker();
        if (!(await this.unlockBeforeStateChange())) {
          if (wasTicking && this.focusRemainingSec > 0) {
            this._ticker = window.setInterval(() => this.tick(), 1000);
          }
          return;
        }
        this.timerPaused = true;
      } else if (this.focusState === "focus" && this.timerPaused) {
        const mode = this.activeFocusMode ?? useSettingsStore().focusMode;
        if (!(await this.syncDesktopLock(desktopLockForFocus(mode)))) return;
        this.timerPaused = false;
        if (this._ticker === null) {
          this._ticker = window.setInterval(() => this.tick(), 1000);
        }
      } else {
        this.timerPaused = !this.timerPaused;
      }
      void emit("focus:state_changed", { state: this.focusState, paused: this.timerPaused });
      this.emitTick();
    },
    async skip() {
      return focusTransitionQueue.request(() => this.runSkip());
    },
    async runSkip() {
      if (this.focusState === "focus") {
        await this.runStartRest(true); // v1.8.2: skipped focus still records elapsed focus time
      } else {
        this.workflowDriven = false;
        await this.runStartFocusFor(this.focusMinutes * 60); // idle or rest -> focus
      }
    },
    toggleFocus() {
      if (this.focusState === "idle") void this.startFocus();
      else if (this.focusState === "focus") void this.pause();
      else void this.startFocus(); // rest (done or not) -> back to focus
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
