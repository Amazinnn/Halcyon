<script setup lang="ts">
import { onMounted } from "vue";
import { useMusicStore } from "../../stores/music";
import WindowHeader from "../../components/WindowHeader.vue";
import AppIcon from "../../components/AppIcon.vue";

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
    <WindowHeader title="音乐" collapsible />
    <div class="body">
      <div class="cover" :class="`g-${music.track.cover}`"></div>
      <div class="meta">
        <div class="title">{{ music.track.title }} · {{ music.track.artist }}</div>
        <div class="progress">
          <input type="range" min="0" max="100" :value="Math.round(music.progressRatio * 100)" @input="onSeek" />
          <span class="times num">{{ fmt(music.positionMs) }} / {{ fmt(music.track.durationMs) }}</span>
        </div>
        <div class="controls">
          <button @click.stop="music.prev()"><AppIcon name="prev" /></button>
          <button @click.stop="music.toggle()"><AppIcon :name="music.playing ? 'pause' : 'play'" /></button>
          <button @click.stop="music.next()"><AppIcon name="next" /></button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.music-window {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: transparent;
  border: 1px solid var(--glass-border);
  border-radius: var(--r-lg);
  overflow: hidden;
  box-sizing: border-box;
}
.body { display: flex; align-items: center; gap: 10px; padding: 10px 12px; flex: 1; }
.cover { width: 52px; height: 52px; border-radius: var(--r-md); flex-shrink: 0; }
.cover.g-1 { background: linear-gradient(135deg, #365314, #a3e635); }
.cover.g-2 { background: linear-gradient(135deg, #14532d, #4ade80); }
.cover.g-3 { background: linear-gradient(135deg, #1a2e05, #bef264); }
.meta { flex: 1; min-width: 0; }
.title { font-size: 12px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.progress { display: flex; align-items: center; gap: 8px; margin-top: 4px; }
.progress input { flex: 1; accent-color: var(--accent); }
.times { font-size: 11px; color: var(--text-mid); }
.controls { display: flex; gap: 8px; margin-top: 4px; }
.controls button {
  border: none; background: #16231c; color: var(--text-hi);
  border-radius: var(--r-sm); padding: 3px 10px; cursor: pointer; display: inline-flex;
}
.controls button:hover { background: var(--accent-wash); color: var(--accent-bright); }
</style>