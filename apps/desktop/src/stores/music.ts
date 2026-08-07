import { defineStore } from "pinia";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface Track {
  path: string;
  title: string;
  artist: string | null;
  album: string | null;
}

export type PlayMode = "list" | "loop" | "single";

export const useMusicStore = defineStore("music", {
  state: () => ({
    folder: null as string | null,
    tracks: [] as Track[],
    currentIndex: -1,
    playing: false,
    positionMs: 0,
    durationMs: 0,
    cover: null as string | null,
    mode: "loop" as PlayMode,
    audio: null as HTMLAudioElement | null,
  }),
  getters: {
    current(state): Track | null {
      return state.currentIndex >= 0 && state.currentIndex < state.tracks.length
        ? state.tracks[state.currentIndex]
        : null;
    },
  },
  actions: {
    ensureAudio() {
      if (this.audio) return;
      const a = new Audio();
      a.preload = "metadata";
      a.addEventListener("timeupdate", () => {
        this.positionMs = Math.round(a.currentTime * 1000);
      });
      a.addEventListener("loadedmetadata", () => {
        this.durationMs = Math.round((a.duration || 0) * 1000);
      });
      a.addEventListener("play", () => { this.playing = true; });
      a.addEventListener("pause", () => { this.playing = false; });
      a.addEventListener("ended", () => this.onEnded());
      a.addEventListener("error", () => {
        console.error("audio error", a.src);
      });
      this.audio = a;
    },
    async init() {
      this.ensureAudio();
      try {
        this.folder = await invoke<string | null>("music_get_folder");
        this.tracks = await invoke<Track[]>("music_list");
      } catch (e) {
        console.error("music init failed", e);
      }
    },
    async chooseFolder() {
      const dir = await open({ directory: true, title: "选择音乐文件夹" });
      if (typeof dir !== "string") return;
      try {
        this.tracks = await invoke<Track[]>("music_set_folder", { dir });
        this.folder = dir;
        this.currentIndex = -1;
        this.positionMs = 0;
        this.durationMs = 0;
        this.cover = null;
        this.playing = false;
      } catch (e) {
        console.error("music_set_folder failed", e);
      }
    },
    async loadCover() {
      const cur = this.current;
      if (!cur) {
        this.cover = null;
        return;
      }
      try {
        this.cover = await invoke<string | null>("music_cover", { path: cur.path });
      } catch {
        this.cover = null;
      }
    },
    async playTrack(i: number) {
      this.ensureAudio();
      const a = this.audio!;
      if (i < 0 || i >= this.tracks.length) return;
      this.currentIndex = i;
      a.src = convertFileSrc(this.tracks[i].path);
      this.positionMs = 0;
      this.durationMs = 0;
      void this.loadCover();
      try {
        await a.play();
      } catch (e) {
        console.error("play failed", e);
      }
    },
    async toggle() {
      this.ensureAudio();
      const a = this.audio!;
      if (!this.current) {
        if (this.tracks.length === 0) return;
        await this.playTrack(0);
        return;
      }
      if (a.paused) void a.play();
      else a.pause();
    },
    async next() {
      if (this.tracks.length === 0) return;
      await this.playTrack((this.currentIndex + 1) % this.tracks.length);
    },
    async prev() {
      if (this.tracks.length === 0) return;
      const n = this.tracks.length;
      await this.playTrack((this.currentIndex - 1 + n) % n);
    },
    seek(ms: number) {
      const a = this.audio;
      if (!a || !this.current) return;
      const max = Math.max(0, this.durationMs / 1000);
      a.currentTime = Math.min(max, Math.max(0, ms / 1000));
    },
    cycleMode() {
      const order: PlayMode[] = ["list", "loop", "single"];
      this.mode = order[(order.indexOf(this.mode) + 1) % order.length];
    },
    onEnded() {
      const a = this.audio;
      if (!a) return;
      if (this.mode === "single") {
        a.currentTime = 0;
        void a.play();
        return;
      }
      if (this.currentIndex < this.tracks.length - 1) {
        void this.playTrack(this.currentIndex + 1);
        return;
      }
      if (this.mode === "loop") {
        void this.playTrack(0);
        return;
      }
      // list mode at the end of the list: stop
      a.currentTime = 0;
      this.playing = false;
      this.positionMs = 0;
    },
  },
});