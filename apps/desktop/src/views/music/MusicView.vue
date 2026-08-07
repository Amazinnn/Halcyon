<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import { useMusicStore, type PlayMode } from "../../stores/music";
import WindowHeader from "../../components/WindowHeader.vue";
import AppIcon from "../../components/AppIcon.vue";

const music = useMusicStore();

const MODE_LABEL: Record<PlayMode, string> = {
  list: "列表",
  loop: "循环",
  single: "单曲",
};

const current = computed(() => music.current);
const modeLabel = computed(() => MODE_LABEL[music.mode]);

function fmt(ms: number) {
  const s = Math.floor(ms / 1000);
  if (!Number.isFinite(s) || s < 0) return "0:00";
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

function coverClass() {
  const t = current.value?.title ?? "";
  let h = 0;
  for (let i = 0; i < t.length; i++) h = (h * 31 + t.charCodeAt(i)) >>> 0;
  return `g-${(h % 3) + 1}`;
}

function onSeek(e: Event) {
  const el = e.target as HTMLInputElement;
  music.seek(Number(el.value) * 1000);
}

onMounted(() => {
  void music.init();
});

onUnmounted(() => {
  music.audio?.pause();
});
</script>

<template>
  <div class="music-window">
    <WindowHeader title="音乐" collapsible />

    <div v-if="!music.folder" class="empty">
      <p class="empty-title">还没有音乐文件夹</p>
      <p class="empty-sub">选择存放 MP3 / FLAC / M4A 的文件夹，即刻开始聆听</p>
      <button class="pick" @click="music.chooseFolder()">选择文件夹</button>
    </div>

    <template v-else>
      <div class="track-list">
        <button
          v-for="(t, i) in music.tracks"
          :key="t.path"
          class="track"
          :class="{ active: i === music.currentIndex }"
          @click="music.playTrack(i)"
        >
          <span class="track-title">{{ t.title }}</span>
          <span class="track-sub">{{ [t.artist, t.album].filter(Boolean).join(" · ") }}</span>
        </button>
        <div v-if="music.tracks.length === 0" class="empty-inline">文件夹里还没有音频文件</div>
      </div>

      <div class="control-bar">
        <div class="row-top">
          <div class="cover" :class="music.cover ? '' : coverClass()">
            <img v-if="music.cover" :src="music.cover" alt="" />
          </div>
          <div class="now-meta">
            <div class="now-title">{{ current?.title ?? "未播放" }}</div>
            <div class="now-artist">{{ current?.artist ?? "" }}</div>
          </div>
          <div class="transport">
            <button class="ctl" title="上一首" @click="music.prev()"><AppIcon name="prev" /></button>
            <button class="ctl play" title="播放/暂停" @click="music.toggle()">
              <AppIcon :name="music.playing ? 'pause' : 'play'" />
            </button>
            <button class="ctl" title="下一首" @click="music.next()"><AppIcon name="next" /></button>
          </div>
          <button class="mode" :title="modeLabel" @click="music.cycleMode()">{{ modeLabel }}</button>
        </div>
        <div class="timeline">
          <span class="time num">{{ fmt(music.positionMs) }}</span>
          <input
            class="progress"
            type="range"
            min="0"
            max="100"
            :value="music.durationMs > 0 ? Math.round((music.positionMs / music.durationMs) * 100) : 0"
            @input="onSeek"
          />
          <span class="time num">{{ fmt(music.durationMs) }}</span>
        </div>
      </div>
    </template>
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
  font-family: "Segoe UI Variable", "Segoe UI", "Microsoft YaHei UI", "PingFang SC", sans-serif;
}

/* ---- empty state ---- */
.empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 20px;
  text-align: center;
}
.empty-title { margin: 0; font-size: 14px; font-weight: 600; color: var(--text-hi); letter-spacing: -0.01em; }
.empty-sub { margin: 0; font-size: 12px; color: rgba(255, 255, 255, 0.66); }
.pick {
  margin-top: 10px;
  border: 1px solid var(--glass-border);
  background: var(--glass-strong);
  color: var(--text-hi);
  border-radius: var(--r-pill);
  padding: 7px 18px;
  font-size: 12px;
  font-weight: 500;
  cursor: pointer;
  transition: border-color var(--t-fast) var(--ease-out), color var(--t-fast) var(--ease-out);
}
.pick:hover { border-color: var(--accent); color: var(--accent-bright); }

/* ---- track list ---- */
.track-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 8px 6px;
}
.track {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 1px;
  width: 100%;
  border: none;
  background: transparent;
  color: var(--text-hi);
  padding: 6px 10px 6px 14px;
  border-radius: var(--r-sm);
  cursor: pointer;
  text-align: left;
  position: relative;
  transition: background var(--t-fast) var(--ease-out);
}
.track:hover { background: rgba(163, 230, 53, 0.08); }
.track.active { background: rgba(163, 230, 53, 0.12); }
.track.active::before {
  content: "";
  position: absolute;
  left: 4px;
  top: 8px;
  bottom: 8px;
  width: 2px;
  border-radius: 2px;
  background: var(--accent);
}
.track-title {
  font-size: 13px;
  font-weight: 500;
  letter-spacing: -0.01em;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.track.active .track-title { color: var(--accent-bright); }
.track-sub {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.66);
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.empty-inline {
  padding: 20px 10px;
  font-size: 12px;
  color: rgba(255, 255, 255, 0.66);
  text-align: center;
}

/* ---- control bar ---- */
.control-bar {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 14px;
  border-top: 1px solid var(--glass-border);
}
.row-top { display: flex; align-items: center; gap: 12px; }
.cover {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  overflow: hidden;
  flex-shrink: 0;
  background: var(--glass-strong);
  border: 1px solid rgba(255, 255, 255, 0.18);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
}
.cover img { width: 100%; height: 100%; object-fit: cover; display: block; }
.cover.g-1 { background: linear-gradient(135deg, #365314, #a3e635); }
.cover.g-2 { background: linear-gradient(135deg, #14532d, #4ade80); }
.cover.g-3 { background: linear-gradient(135deg, #1a2e05, #bef264); }

.now-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}
.now-title {
  font-size: 14px;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--text-hi);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.now-artist {
  font-size: 11px;
  color: rgba(255, 255, 255, 0.66);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.transport { display: flex; align-items: center; gap: 4px; flex-shrink: 0; }
.ctl {
  border: none;
  background: transparent;
  color: rgba(255, 255, 255, 0.75);
  border-radius: var(--r-sm);
  padding: 5px;
  cursor: pointer;
  display: inline-flex;
  transition: color var(--t-fast) var(--ease-out), background var(--t-fast) var(--ease-out);
}
.ctl:hover { color: var(--accent-bright); background: rgba(163, 230, 53, 0.1); }
.ctl.play {
  color: #0a110e;
  background: var(--accent);
  border-radius: var(--r-pill);
  padding: 7px;
}
.ctl.play:hover { background: var(--accent-bright); color: #0a110e; }

.mode {
  flex-shrink: 0;
  border: 1px solid var(--glass-border);
  background: transparent;
  color: rgba(255, 255, 255, 0.6);
  border-radius: var(--r-pill);
  padding: 5px 12px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  cursor: pointer;
  transition: border-color var(--t-fast) var(--ease-out), color var(--t-fast) var(--ease-out);
}
.mode:hover { border-color: var(--accent); color: var(--accent-bright); }

.timeline {
  display: flex;
  align-items: center;
  gap: 8px;
}
.time {
  font-size: 11px;
  font-weight: 500;
  color: rgba(255, 255, 255, 0.66);
}
.progress {
  flex: 1;
  appearance: none;
  -webkit-appearance: none;
  height: 4px;
  border-radius: 999px;
  background: rgba(255, 255, 255, 0.15);
  outline: none;
  cursor: pointer;
}
.progress::-webkit-slider-thumb {
  appearance: none;
  -webkit-appearance: none;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--accent);
  border: 2px solid #0a110e;
  box-shadow: 0 0 0 1px rgba(163, 230, 53, 0.4);
}
</style>