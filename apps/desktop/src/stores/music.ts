import { defineStore } from "pinia";
import { emit } from "@tauri-apps/api/event";

// Fake playback state (no real media control in the spike, per plan boundaries).
export interface FakeTrack {
  id: string;
  title: string;
  artist: string;
  album: string;
  genre: string;
  durationMs: number;
  cover: string;
}

export const FAKE_PLAYLIST: FakeTrack[] = [
  { id: "t1", title: "Midnight Focus", artist: "Focus Ensemble", album: "Deep Work", genre: "纯音乐", durationMs: 246_000, cover: "🌙" },
  { id: "t2", title: "Rainy Window", artist: "Ambient Lab", album: "Study Rain", genre: "白噪音", durationMs: 312_000, cover: "🌧" },
  { id: "t3", title: "Neon Circuit", artist: "Pixel Waves", album: "Night Drive", genre: "电子", durationMs: 198_000, cover: "🎧" },
];

export const useMusicStore = defineStore("music", {
  state: () => ({
    trackIndex: 0,
    positionMs: 0,
    playing: false,
    tickerStarted: false,
  }),
  getters: {
    track: (s) => FAKE_PLAYLIST[s.trackIndex],
    progressRatio(): number {
      const t = this.track;
      return t.durationMs > 0 ? Math.min(1, this.positionMs / t.durationMs) : 0;
    },
  },
  actions: {
    startTicker() {
      if (this.tickerStarted) return;
      this.tickerStarted = true;
      window.setInterval(() => {
        if (!this.playing) return;
        this.positionMs += 1000;
        if (this.positionMs >= this.track.durationMs) {
          this.next();
        }
        // Exercises the frontend -> core -> bus path (music.playback.tick).
        void emit("music:playback_tick", {
          positionMs: this.positionMs,
          durationMs: this.track.durationMs,
        });
      }, 1000);
    },
    toggle() {
      this.playing = !this.playing;
    },
    next() {
      this.trackIndex = (this.trackIndex + 1) % FAKE_PLAYLIST.length;
      this.positionMs = 0;
    },
    prev() {
      this.trackIndex = (this.trackIndex - 1 + FAKE_PLAYLIST.length) % FAKE_PLAYLIST.length;
      this.positionMs = 0;
    },
    seek(ratio: number) {
      this.positionMs = Math.round(Math.min(1, Math.max(0, ratio)) * this.track.durationMs);
    },
  },
});