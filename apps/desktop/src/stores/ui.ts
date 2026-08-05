import { defineStore } from "pinia";
import { listen } from "@tauri-apps/api/event";

export type FocusState = "idle" | "focus" | "rest";

/** Default rest duration for the fake focus/rest heartbeat (seconds). */
export const REST_DURATION_SEC = 600;

/**
 * Slim UI coordinator (v1.3): chat visibility drives the pet bubble mute rule;
 * focus/rest state drives a 1s fake heartbeat (focus counts up, rest counts
 * down to 0 then auto-returns to focus) plus the desktop "spring" ambient.
 */
export const useUiStore = defineStore("ui", {
  state: () => ({
    chatOpen: true,
    doNotDisturb: false,
    lockActive: false,
    focusState: "idle" as FocusState,
    focusElapsedSec: 0,
    restRemainingSec: REST_DURATION_SEC,
    focusSubtitle: "保持节奏，阳光会照到每一片叶子",
    _ticker: null as number | null,
  }),
  actions: {
    async init() {
      await listen("window:visibility", (e) => {
        const p = e.payload as { label: string; visible: boolean };
        if (p.label === "chat") this.chatOpen = p.visible;
      });
    },
    toggleFocus() {
      if (this.focusState === "idle") this.startFocus();
      else if (this.focusState === "focus") this.startRest();
      else this.startFocus(); // rest -> focus (manual)
    },
    startFocus() {
      this.stopTicker();
      this.focusState = "focus";
      this.focusElapsedSec = 0;
      this._ticker = window.setInterval(() => {
        this.focusElapsedSec++;
      }, 1000);
    },
    startRest() {
      this.stopTicker();
      this.focusState = "rest";
      this.restRemainingSec = REST_DURATION_SEC;
      this._ticker = window.setInterval(() => {
        this.restRemainingSec--;
        if (this.restRemainingSec <= 0) this.startFocus();
      }, 1000);
    },
    stopTicker() {
      if (this._ticker !== null) {
        window.clearInterval(this._ticker);
        this._ticker = null;
      }
    },
  },
});
