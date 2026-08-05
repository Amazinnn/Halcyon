<script setup lang="ts">
import { onMounted, ref } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useAgentStore } from "../../stores/agent";
import AppIcon from "../../components/AppIcon.vue";

const agent = useAgentStore();
const wallpaperUrl = ref("");
const dropActive = ref(false);
const launcherApps = ["VS Code", "Obsidian", "浏览器", "文件夹"];
const quickPages = ["课程项目", "日记", "任务", "统计"];

async function loadWallpaper() {
  const p = await invoke<string | null>("get_wallpaper");
  wallpaperUrl.value = p ? convertFileSrc(p) : "";
}

async function applyWallpaper(path: string) {
  try {
    const saved = await invoke<string>("persist_wallpaper", { src: path });
    wallpaperUrl.value = convertFileSrc(saved);
  } catch (e) {
    console.error("wallpaper import failed", e);
  }
}

async function pickWallpaper() {
  const sel = await open({
    multiple: false,
    filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp"] }],
  });
  if (typeof sel === "string") await applyWallpaper(sel);
}

async function resetWallpaper() {
  await invoke("reset_wallpaper");
  wallpaperUrl.value = "";
}

function quit() {
  void invoke("quit_app");
}

onMounted(async () => {
  await loadWallpaper();
  const win = getCurrentWebviewWindow();
  await win.onDragDropEvent((e) => {
    const type = e.payload.type;
    dropActive.value = type === "over" || type === "enter";
    if (type === "drop") {
      const path = e.payload.paths?.[0];
      if (path) void applyWallpaper(path);
    }
  });
});
</script>

<template>
  <div class="desktop-view">
    <!-- background: default theme or imported wallpaper with blurred/vignette edges -->
    <div v-if="!wallpaperUrl" class="bg-default">
      <div class="noise"></div>
    </div>
    <template v-else>
      <div class="wallpaper-img" :style="{ backgroundImage: `url(${wallpaperUrl})` }"></div>
      <div class="wallpaper-blur" :style="{ backgroundImage: `url(${wallpaperUrl})` }"></div>
      <div class="wallpaper-tint"></div>
      <div class="wallpaper-vignette"></div>
    </template>

    <div v-if="dropActive" class="drop-hint">松开以设置壁纸</div>

    <header class="topbar">
      <span class="task">当前任务：实现统计模块</span>
      <span class="rest num">休息 12:00</span>
      <span class="agent-status">
        Agent: {{ agent.state }}
        <span class="dot" :class="`st-${agent.state}`"></span>
      </span>
    </header>

    <section class="hero">
      <div class="timer num">00:00:00</div>
      <div class="timer-label">专注中</div>
      <div class="task-line">保持节奏，阳光会照到每一片叶子</div>
    </section>

    <section class="icon-zone">
      <div v-for="app in launcherApps" :key="app" class="icon glass">
        <span class="glyph"><AppIcon name="leaf" /></span>
        <span class="name">{{ app }}</span>
      </div>
      <div v-for="page in quickPages" :key="page" class="icon glass small">
        <span class="name">{{ page }}</span>
      </div>
    </section>

    <footer class="dock glass">
      <button class="dock-btn" title="运行中应用（后续）" disabled><AppIcon name="stats" /><span>运行中</span></button>
      <button class="dock-btn" title="开始专注（后续）" disabled><AppIcon name="leaf" /><span>开始专注</span></button>
      <button class="dock-btn" @click="pickWallpaper"><AppIcon name="image" /><span>壁纸</span></button>
      <button class="dock-btn" @click="resetWallpaper" title="恢复默认背景"><AppIcon name="close" /><span>重置</span></button>
      <button class="dock-btn" title="设置（后续）" disabled><AppIcon name="pin" /><span>设置</span></button>
      <button class="dock-btn" @click="quit"><AppIcon name="close" /><span>退出</span></button>
    </footer>
  </div>
</template>

<style scoped>
.desktop-view {
  position: relative;
  height: 100vh;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
  color: var(--text-hi);
  background: var(--bg-0);
}
/* default theme background */
.bg-default {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(1200px 620px at 82% 8%, var(--accent-glow), transparent 60%),
    radial-gradient(1000px 700px at 12% 88%, rgba(163, 230, 53, 0.07), transparent 60%),
    linear-gradient(160deg, #0a110e 0%, var(--bg-0) 55%, var(--bg-1) 100%);
}
.noise {
  position: absolute;
  inset: 0;
  opacity: 0.05;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='120'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2'/%3E%3C/filter%3E%3Crect width='120' height='120' filter='url(%23n)'/%3E%3C/svg%3E");
}
/* wallpaper layers */
.wallpaper-img { position: absolute; inset: 0; background-size: cover; background-position: center; }
.wallpaper-blur {
  position: absolute; inset: -48px;
  background-size: cover; background-position: center;
  filter: blur(30px); opacity: 0.6;
  -webkit-mask-image: radial-gradient(ellipse at center, transparent 52%, black 100%);
  mask-image: radial-gradient(ellipse at center, transparent 52%, black 100%);
}
.wallpaper-tint { position: absolute; inset: 0; background: rgba(7, 11, 9, 0.28); }
.wallpaper-vignette {
  position: absolute; inset: 0;
  background: radial-gradient(ellipse at center, transparent 52%, var(--bg-0) 100%);
}
.drop-hint {
  position: absolute; inset: 0; z-index: 20;
  display: flex; align-items: center; justify-content: center;
  background: rgba(7, 11, 9, 0.55);
  border: 2px dashed var(--accent);
  color: var(--accent-bright); font-size: 18px;
}
.topbar {
  position: relative; z-index: 5;
  display: flex; align-items: center; gap: 24px;
  padding: 14px 24px;
}
.task { font-size: 15px; font-weight: 600; }
.rest { color: var(--text-mid); font-size: 13px; }
.agent-status { margin-left: auto; font-size: 13px; color: var(--text-mid); display: flex; align-items: center; gap: 6px; }
.dot { width: 8px; height: 8px; border-radius: 50%; background: var(--text-low); }
.dot.st-thinking, .dot.st-reading, .dot.st-searching { background: var(--accent); }
.dot.st-editing, .dot.st-running, .dot.st-testing { background: var(--accent-bright); }
.dot.st-waiting_permission { background: var(--warn); }
.dot.st-success { background: var(--accent); }
.dot.st-error { background: var(--err); }
.hero {
  position: relative; z-index: 5;
  display: flex; flex-direction: column; align-items: center;
  padding-top: 7vh;
}
.timer {
  font-size: clamp(3rem, 9vw, 6rem);
  font-weight: 300;
  letter-spacing: 0.06em;
  color: var(--accent-bright);
  animation: breathe 2.4s ease-in-out infinite;
}
.timer-label { font-size: 14px; color: var(--text-mid); margin-top: 4px; }
.task-line { font-size: 13px; color: var(--text-low); margin-top: 8px; }
.icon-zone {
  position: relative; z-index: 5;
  display: grid;
  grid-template-columns: repeat(4, 96px);
  gap: 14px;
  justify-content: center;
  margin-top: 6vh;
}
.icon { width: 96px; height: 84px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 6px; }
.icon.small { height: 40px; width: auto; flex-direction: row; padding: 0 14px; font-size: 12px; }
.glyph { color: var(--accent); }
.name { font-size: 12px; color: var(--text-mid); }
.dock {
  position: relative; z-index: 5;
  margin: auto auto 22px;
  display: flex; gap: 6px; padding: 8px 12px;
}
.dock-btn {
  border: none; background: transparent; color: var(--text-mid);
  display: flex; align-items: center; gap: 6px;
  padding: 6px 10px; border-radius: var(--r-sm); cursor: pointer; font-size: 12px;
}
.dock-btn:hover:not(:disabled) { color: var(--accent-bright); background: var(--accent-wash); }
.dock-btn:disabled { opacity: 0.5; cursor: default; }
</style>