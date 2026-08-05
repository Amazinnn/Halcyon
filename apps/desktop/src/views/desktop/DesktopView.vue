<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useUiStore } from "../../stores/ui";
import { useSettingsStore } from "../../stores/settings";
import { useShortcutStore } from "../../stores/shortcuts";
import type { ShortcutType } from "../../lib/shortcuts";
import AppIcon from "../../components/AppIcon.vue";
import SettingsPopover from "../../components/SettingsPopover.vue";

const ui = useUiStore();
const settings = useSettingsStore();
const shortcuts = useShortcutStore();

const wallpaperUrl = ref("");
const dropActive = ref(false);
const addMenuOpen = ref(false);
const menuMode = ref<"" | "url" | "internal">("");
const urlName = ref("");
const urlValue = ref("");
const settingsOpen = ref(false);

// centered shortcut grid: up to 2 rows x 5 cols, centered (never touches the screen
// edges); the + flows at the end of the last row (max 9 shortcuts).
// ---- timer ----
function fmtClock(totalSec: number) {
  const s = Math.max(0, Math.floor(totalSec));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
}
function fmtShort(totalSec: number) {
  const s = Math.max(0, Math.floor(totalSec));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return h > 0 ? `${h}h${m}m` : `${m} 分钟`;
}

const timerText = computed(() =>
  ui.focusState === "rest" ? fmtClock(ui.restRemainingSec) : fmtClock(ui.focusRemainingSec),
);
const timerLabel = computed(() =>
  ui.focusState === "focus" ? "专注中" : ui.focusState === "rest" ? "休息中" : "未开始",
);
const ringCirc = 2 * Math.PI * 150;
const ringProgress = computed(() => {
  if (ui.focusState === "idle") return 1;
  if (ui.focusState === "rest" && ui.phaseDone) return 0;
  const total = ui.focusState === "focus" ? ui.focusMinutes * 60 : ui.restMinutes * 60;
  const remain = ui.focusState === "focus" ? ui.focusRemainingSec : ui.restRemainingSec;
  return total > 0 ? Math.max(0, Math.min(1, remain / total)) : 0;
});
const ringOffset = computed(() => ringCirc * (1 - ringProgress.value));
const todayText = computed(() => `今日专注 ${fmtShort(ui.todayFocusSec)} · 完成 ${ui.todayRounds} 轮`);

function glyphFor(type: ShortcutType): string {
  switch (type) {
    case "folder": return "folder";
    case "application": return "app";
    case "url": return "url";
    case "internal": return "panel";
    default: return "file";
  }
}

// ---- views tray: temporary panel for the three float views ----
const viewsTrayOpen = ref(false);
const MAX_SHORTCUTS = 9;
const canAdd = computed(() => shortcuts.items.length < MAX_SHORTCUTS);

async function openView(label: string) {
  viewsTrayOpen.value = false;
  try {
    await invoke("restore", { label });
  } catch (e) {
    console.error("open view failed", label, e);
  }
}
// ---- add menu ----
async function loadWallpaper() {
  const p = await invoke<string | null>("get_wallpaper");
  wallpaperUrl.value = p ? convertFileSrc(p) : "";
}

async function pickFiles() {
  addMenuOpen.value = false;
  menuMode.value = "";
  const sel = await open({ multiple: true });
  if (Array.isArray(sel)) for (const p of sel) await shortcuts.addPath(p);
  else if (typeof sel === "string") await shortcuts.addPath(sel);
}

async function pickFolders() {
  addMenuOpen.value = false;
  menuMode.value = "";
  const sel = await open({ directory: true, multiple: true });
  if (Array.isArray(sel)) for (const p of sel) await shortcuts.addPath(p);
  else if (typeof sel === "string") await shortcuts.addPath(sel);
}

async function submitUrl() {
  const url = urlValue.value.trim();
  if (!(url.startsWith("http://") || url.startsWith("https://"))) return;
  await shortcuts.addUrl(urlName.value.trim(), url);
  urlName.value = "";
  urlValue.value = "";
  addMenuOpen.value = false;
  menuMode.value = "";
}

async function submitInternal(name: string, target: string) {
  await shortcuts.addInternal(name, target);
  addMenuOpen.value = false;
  menuMode.value = "";
}

function remove(id: string) {
  void shortcuts.remove(id);
}

function quit() {
  void invoke("quit_app");
}

onMounted(async () => {
  await settings.load();
  try {
    const b = await invoke<{ focusSubtitle?: string }>("get_bootstrap");
    if (b.focusSubtitle) ui.focusSubtitle = b.focusSubtitle;
  } catch {
    /* ignore */
  }
  await loadWallpaper();
  await shortcuts.load();
  await ui.loadTodaySummary();
  const win = getCurrentWebviewWindow();
  await win.onDragDropEvent((e) => {
    const type = e.payload.type;
    dropActive.value = type === "over" || type === "enter";
    if (type === "drop") {
      const path = e.payload.paths?.[0];
      if (path) {
        void (async () => {
          try {
            const saved = await invoke<string>("persist_wallpaper", { src: path });
            wallpaperUrl.value = convertFileSrc(saved);
          } catch (err) {
            console.error("wallpaper import failed", err);
          }
        })();
      }
    }
  });
});
</script>

<template>
  <div class="desktop-view" :class="{ 'focus-active': ui.focusState === 'focus' }">
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

    <div v-if="dropActive" class="drop-hint">松开以设置为壁纸</div>

    <section class="hero">
      <div class="timer-wrap" :class="ui.focusState">
        <svg class="ring" viewBox="0 0 360 360">
          <circle class="ring-bg" cx="180" cy="180" r="150" />
          <circle
            class="ring-fg"
            cx="180"
            cy="180"
            r="150"
            :stroke-dasharray="ringCirc"
            :stroke-dashoffset="ringOffset"
          />
        </svg>
        <div class="timer num">{{ timerText }}</div>
      </div>
      <div class="timer-label">{{ timerLabel }}</div>
      <div class="task-line">{{ ui.focusSubtitle }}</div>
      <div v-if="ui.focusState !== 'idle' && !ui.phaseDone" class="timer-controls">
        <button class="ctl glass" @click="ui.pause()">
          <AppIcon :name="ui.timerPaused ? 'play' : 'pause'" />
          <span>{{ ui.timerPaused ? "继续" : "暂停" }}</span>
        </button>
        <button class="ctl glass" @click="ui.skip()">
          <AppIcon name="next" />
          <span>跳过</span>
        </button>
      </div>
      <div class="today-line num">{{ todayText }}</div>
    </section>

    <!-- centered shortcut grid (2 rows x 5 cols) + views tray -->
    <section class="icon-area">
      <div class="views-wrap" @mouseleave="viewsTrayOpen = false">
        <button class="views-btn glass" title="视图" @click="viewsTrayOpen = !viewsTrayOpen">
          <AppIcon name="panel" />
        </button>
        <div v-if="viewsTrayOpen" class="views-tray glass">
          <button class="view-item" @click="openView('chat')">
            <AppIcon name="chat" /><span>对话</span>
          </button>
          <button class="view-item" @click="openView('stats')">
            <AppIcon name="stats" /><span>统计</span>
          </button>
          <button class="view-item" @click="openView('music')">
            <AppIcon name="music" /><span>音乐</span>
          </button>
        </div>
      </div>

      <div class="shortcut-grid">
        <div
          v-for="sc in shortcuts.items"
          :key="sc.id"
          class="sc-card glass"
          @click="shortcuts.open(sc)"
        >
          <button class="rm" title="移除" @click.stop="remove(sc.id)">
            <AppIcon name="close" />
          </button>
          <img
            v-if="sc.type === 'application' && shortcuts.icons[sc.target]"
            class="sc-icon"
            :src="shortcuts.icons[sc.target]"
            alt=""
          />
          <span v-else class="glyph" :class="sc.type"><AppIcon :name="glyphFor(sc.type)" /></span>
          <span class="name">{{ sc.name }}</span>
        </div>

        <div v-if="canAdd" class="add-slot">
          <button class="add-fab glass" title="添加快捷方式" @click="addMenuOpen = !addMenuOpen">
            <AppIcon name="plus" />
          </button>
          <div v-if="addMenuOpen" class="menu-backdrop" @click="addMenuOpen = false"></div>
          <div v-if="addMenuOpen" class="add-menu glass">
            <button @click="pickFiles">文件 / 应用</button>
            <button @click="pickFolders">文件夹</button>
            <button @click="menuMode = menuMode === 'url' ? '' : 'url'">URL 链接</button>
            <button @click="menuMode = menuMode === 'internal' ? '' : 'internal'">内部页</button>
            <div v-if="menuMode === 'url'" class="menu-inline">
              <input v-model="urlName" class="text-input" placeholder="名称（可选）" @keydown.enter="submitUrl" />
              <input v-model="urlValue" class="text-input" placeholder="https://…" @keydown.enter="submitUrl" />
              <button class="btn" @click="submitUrl">添加</button>
            </div>
            <div v-if="menuMode === 'internal'" class="menu-inline">
              <button class="btn" @click="submitInternal('对话', 'chat')">对话</button>
              <button class="btn" @click="submitInternal('统计', 'stats')">统计</button>
              <button class="btn" @click="submitInternal('音乐', 'music')">音乐</button>
            </div>
          </div>
        </div>
      </div>
    </section>

    <footer class="dock glass">
      <button v-if="ui.focusState !== 'focus'" class="dock-btn" @click="ui.startFocus()">
        <AppIcon name="leaf" /><span>开始专注</span>
      </button>
      <button class="dock-btn" @click="settingsOpen = !settingsOpen">
        <AppIcon name="settings" /><span>设置</span>
      </button>
      <button class="dock-btn" @click="quit">
        <AppIcon name="power" /><span>退出</span>
      </button>
    </footer>

    <div v-if="settingsOpen" class="popover-backdrop" @click="settingsOpen = false"></div>
    <SettingsPopover v-if="settingsOpen" @close="settingsOpen = false" />
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
  transition: filter var(--t-slow) var(--ease-out);
}
.noise {
  position: absolute;
  inset: 0;
  opacity: 0.05;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='120'%3E%3Cfilter id='n'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2'/%3E%3C/filter%3E%3Crect width='120' height='120' filter='url(%23n)'/%3E%3C/svg%3E");
}
.wallpaper-img { position: absolute; inset: 0; background-size: cover; background-position: center; transition: filter var(--t-slow) var(--ease-out); }
.wallpaper-blur {
  position: absolute; inset: -48px;
  background-size: cover; background-position: center;
  filter: blur(30px); opacity: 0.6;
  -webkit-mask-image: radial-gradient(ellipse at center, transparent 52%, black 100%);
  mask-image: radial-gradient(ellipse at center, transparent 52%, black 100%);
  transition: filter var(--t-slow) var(--ease-out);
}
.wallpaper-tint { position: absolute; inset: 0; background: rgba(7, 11, 9, 0.28); transition: background var(--t-slow) var(--ease-out); }
.wallpaper-vignette {
  position: absolute; inset: 0;
  background: radial-gradient(ellipse at center, transparent 52%, var(--bg-0) 100%);
}
.desktop-view.focus-active .wallpaper-img { filter: brightness(1.08) saturate(1.14); }
.desktop-view.focus-active .wallpaper-blur { filter: blur(30px) brightness(1.1) saturate(1.16); }
.desktop-view.focus-active .wallpaper-tint { background: rgba(163, 230, 53, 0.1); }
.desktop-view.focus-active .bg-default { filter: brightness(1.08) saturate(1.12); }

.drop-hint {
  position: absolute; inset: 0; z-index: 20;
  display: flex; align-items: center; justify-content: center;
  background: rgba(7, 11, 9, 0.55);
  border: 2px dashed var(--accent);
  color: var(--accent-bright); font-size: 18px;
}

/* hero */
.hero {
  position: relative; z-index: 5;
  display: flex; flex-direction: column; align-items: center;
  padding-top: 4vh;
  pointer-events: none;
}
.hero > * { pointer-events: auto; }
.timer-wrap { position: relative; display: inline-flex; align-items: center; justify-content: center; }
.ring { width: 360px; height: 360px; transform: rotate(-90deg); }
.ring-bg { fill: none; stroke: rgba(163, 230, 53, 0.1); stroke-width: 4; }
.ring-fg { fill: none; stroke: var(--accent); stroke-width: 4; stroke-linecap: round; transition: stroke-dashoffset 0.6s linear, stroke 0.3s; }
.timer-wrap.rest .ring-fg { stroke: var(--warn); }
.timer {
  position: absolute;
  font-size: clamp(2.4rem, 7vw, 4.6rem);
  font-weight: 300;
  letter-spacing: 0.05em;
  color: var(--accent-bright);
  animation: breathe 2.4s ease-in-out infinite;
}
.timer-label { font-size: 14px; color: var(--text-mid); margin-top: 6px; }
.task-line { font-size: 13px; color: var(--text-low); margin-top: 6px; }
.timer-controls { display: flex; gap: 10px; margin-top: 12px; }
.ctl {
  display: inline-flex; align-items: center; gap: 6px;
  border: 1px solid var(--glass-border); color: var(--text-hi);
  border-radius: var(--r-pill); padding: 6px 14px; font-size: 13px; cursor: pointer;
  transition: border-color var(--t-fast), color var(--t-fast), background var(--t-fast);
}
.ctl:hover { border-color: var(--accent); color: var(--accent-bright); background: var(--accent-wash); }
.today-line { font-size: 12px; color: var(--text-mid); margin-top: 12px; }

/* centered shortcut grid: up to 2 rows x 5 cols, centered, never touches the
   screen edges; the + flows at the end of the last row (max 9 shortcuts). */
.icon-area {
  position: absolute;
  left: 50%;
  top: calc(50% + 150px);
  transform: translate(-50%, -50%);
  z-index: 4;
  display: flex;
  align-items: flex-start;
  gap: 18px;
  pointer-events: none;
}
.shortcut-grid {
  width: calc(5 * 104px + 4 * 8px);
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px;
  pointer-events: none;
}
.sc-card {
  pointer-events: auto;
  position: relative;
  box-sizing: border-box;
  width: 104px; height: 92px;
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: 6px;
  border-radius: var(--r-md);
  cursor: pointer;
  transition: transform var(--t-base) var(--ease-out), box-shadow var(--t-base) var(--ease-out), border-color var(--t-base) var(--ease-out);
}
.sc-card:hover { transform: translateY(-3px); border-color: var(--accent); box-shadow: 0 8px 24px rgba(163, 230, 53, 0.18); }
.rm {
  position: absolute; top: 4px; right: 4px;
  border: none; background: rgba(0, 0, 0, 0.35); color: var(--text-mid);
  border-radius: 50%; padding: 2px; cursor: pointer;
  display: inline-flex; opacity: 0; transition: opacity var(--t-fast) var(--ease-out), color var(--t-fast);
  z-index: 2;
}
.sc-card:hover .rm { opacity: 1; }
.rm:hover { color: var(--err); }
.glyph { display: inline-flex; }
.glyph.file { color: var(--text-mid); }
.glyph.folder { color: var(--accent); }
.glyph.application { color: var(--accent-bright); }
.glyph.url { color: var(--accent); }
.glyph.internal { color: var(--accent-bright); }
.sc-icon { width: 32px; height: 32px; border-radius: 6px; object-fit: contain; }
.name {
  font-size: 12px; color: var(--text-mid);
  max-width: 96px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
/* + flows at the end of the grid rows */
.add-slot {
  pointer-events: auto;
  position: relative;
  box-sizing: border-box;
  width: 104px; height: 92px;
  display: flex; align-items: center; justify-content: center;
}
.add-fab {
  box-sizing: border-box;
  width: 52px; height: 52px;
  border-radius: var(--r-pill);
  display: flex; align-items: center; justify-content: center;
  color: var(--accent-bright);
  cursor: pointer;
  transition: transform var(--t-base) var(--ease-out), border-color var(--t-base), box-shadow var(--t-base);
}
.add-fab:hover { transform: translateY(-2px); border-color: var(--accent); box-shadow: 0 8px 24px rgba(163, 230, 53, 0.25); }
.menu-backdrop { position: fixed; inset: 0; z-index: 10; }
.add-menu {
  position: absolute;
  left: 50%;
  bottom: calc(100% + 10px);
  transform: translateX(-50%);
  z-index: 12;
  display: flex; flex-direction: column; gap: 4px;
  padding: 6px;
  border-radius: var(--r-md);
  box-shadow: 0 10px 28px rgba(0, 0, 0, 0.35);
}
.add-menu button {
  border: none; background: transparent; color: var(--text-hi);
  border-radius: var(--r-sm); padding: 8px 22px; font-size: 13px; cursor: pointer; text-align: left;
}
.add-menu button:hover { background: var(--accent-wash); color: var(--accent-bright); }
.menu-inline {
  display: flex; flex-direction: column; gap: 6px;
  padding: 6px 8px 8px;
}
.text-input {
  border: 1px solid var(--glass-border); background: var(--glass-strong);
  color: var(--text-hi); border-radius: var(--r-sm); padding: 4px 8px; font-size: 12px;
  font-family: inherit; min-width: 180px;
}
.menu-inline .btn {
  border: 1px solid var(--glass-border); background: var(--glass-strong);
  color: var(--text-hi); border-radius: var(--r-sm); padding: 5px 10px;
  font-size: 12px; cursor: pointer; text-align: center;
}
.menu-inline .btn:hover { border-color: var(--accent); color: var(--accent-bright); }
/* views icon + temporary tray (mouse leaves -> tray hides) */
.views-wrap {
  pointer-events: auto;
  position: relative;
  margin-top: 22px;
}
.views-btn {
  box-sizing: border-box;
  width: 48px; height: 48px;
  border-radius: var(--r-pill);
  display: flex; align-items: center; justify-content: center;
  color: var(--accent-bright);
  cursor: pointer;
  transition: transform var(--t-base) var(--ease-out), border-color var(--t-base), box-shadow var(--t-base);
}
.views-btn:hover { transform: translateY(-2px); border-color: var(--accent); box-shadow: 0 8px 24px rgba(163, 230, 53, 0.25); }
.views-tray {
  position: absolute;
  top: calc(100% + 10px);
  left: 0;
  z-index: 12;
  display: flex; flex-direction: column; gap: 4px;
  padding: 6px;
  border-radius: var(--r-md);
  box-shadow: 0 10px 28px rgba(0, 0, 0, 0.35);
}
.view-item {
  border: none; background: transparent; color: var(--text-hi);
  display: flex; align-items: center; gap: 8px;
  border-radius: var(--r-sm); padding: 8px 16px; font-size: 13px; cursor: pointer; text-align: left;
  white-space: nowrap;
}
.view-item:hover { background: var(--accent-wash); color: var(--accent-bright); }
/* dock */
.dock {
  position: relative; z-index: 5;
  align-self: center;
  margin: auto auto 22px;
  display: flex; gap: 8px; padding: 8px 12px;
  border-radius: var(--r-pill);
}
.dock-btn {
  border: none; background: transparent; color: var(--text-mid);
  display: flex; align-items: center; gap: 6px;
  padding: 7px 12px; border-radius: var(--r-pill); cursor: pointer; font-size: 13px;
  transition: color var(--t-fast) var(--ease-out), background var(--t-fast) var(--ease-out);
}
.dock-btn:hover { color: var(--accent-bright); background: var(--accent-wash); }

.popover-backdrop { position: fixed; inset: 0; z-index: 20; }
</style>
