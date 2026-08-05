<script setup lang="ts">
import { onMounted } from "vue";
import { useMusicStore } from "../../stores/music";

const music = useMusicStore();

function fmt(ms: number) {
  const s = Math.floor(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

function onSeek(e: Event) {
  const el = e.target as HTMLInputElement;
  music.seek(Number(el.value) / 100);
}

onMounted(() => {
  music.startTicker();
});
</script>

<template>
  <div class="music-window">
    <div class="cover" data-tauri-drag-region>{{ music.track.cover }}</div>
    <div class="meta">
      <div class="title">{{ music.track.title }} · {{ music.track.artist }}</div>
      <div class="progress">
        <input
          type="range"
          min="0"
          max="100"
          :value="Math.round(music.progressRatio * 100)"
          @input="onSeek"
        />
        <span class="times">{{ fmt(music.positionMs) }} / {{ fmt(music.track.durationMs) }}</span>
      </div>
      <div class="controls">
        <button @click.stop="music.prev()">⏮</button>
        <button @click.stop="music.toggle()">{{ music.playing ? "⏸" : "▶" }}</button>
        <button @click.stop="music.next()">⏭</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.music-window {
  height: 100vh;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background: rgba(24, 28, 46, 0.92);
  border-radius: 12px;
  color: #eef;
  box-sizing: border-box;
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.35);
}
.cover {
  width: 56px;
  height: 56px;
  border-radius: 8px;
  background: linear-gradient(135deg, #4f7cff, #9b59b6);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 26px;
  flex-shrink: 0;
}
.meta {
  flex: 1;
  min-width: 0;
}
.title {
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.progress {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 2px;
}
.progress input {
  flex: 1;
  accent-color: #4f7cff;
}
.times {
  font-size: 11px;
  color: #aab4d0;
  font-variant-numeric: tabular-nums;
}
.controls {
  display: flex;
  gap: 8px;
  margin-top: 2px;
}
.controls button {
  border: none;
  background: rgba(255, 255, 255, 0.12);
  color: #eef;
  border-radius: 8px;
  padding: 2px 10px;
  cursor: pointer;
  font-size: 13px;
}
.controls button:hover {
  background: rgba(255, 255, 255, 0.22);
}
</style>