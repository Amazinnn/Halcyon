<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useAgentStore } from "../../stores/agent";
import { useUiStore } from "../../stores/ui";
import { useShortcutStore } from "../../stores/shortcuts";
import type { DesktopShortcut, ShortcutType } from "../../lib/shortcuts";
import AppIcon from "../../components/AppIcon.vue";
import SettingsPopover from "../../components/SettingsPopover.vue";

const agent = useAgentStore();
const ui = useUiStore();
const shortcuts = useShortcutStore();

const wallpaperUrl = ref("");
const dropActive = ref(false);
const addMenuOpen = ref(false);
const settingsOpen = ref(false);

// pointer-based reorder within the shortcut grid
const dragId = ref<string | null>(null);
const dragOverId = ref<string | null>(null);
const dragMoved = ref(false);
const dragStart = { x: 0, y: 0 };
const cardRefs = ref<Record<string, HTMLElement | null>>({});

function fmtClock(totalSec: number) {
  const s = Math.max(0, Math.floor(totalSec));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  return `${String(h).padStart(2, "0")}:${String(m).padStart(2, "0")}:${String(sec).padStart(2, "0")}`;
}

const timerText = computed(() =>
  ui.focusState === "rest" ? fmtClock(ui.restRemainingSec) : fmtClock(ui.focusElapsedSec),
);
const timerLabel = computed(() =>
  ui.focusState === "focus" ? "专注中" : ui.focusState === "rest" ? "休息中" : "未开始",
);
const modeChip = computed(() => {
  if (ui.focusState === "focus") return `专注中 · ${fmtClock(ui.focusElapsedSec)}`;
  if (ui.focusState === "rest") return `休息中 · ${fmtClock(ui.restRemainingSec)}`;
  return "未开始";
});
const focusBtnText = computed(() =>
  ui.focusState === "idle" ? "开始专注" : ui.focusState === "focus" ? "休息" : "继续专注",
);
const focusIcon = computed(() =>
  ui.focusState === "idle" ? "leaf" : ui.focusState === "focus" ? "pause" : "play",
);

function glyphFor(type: ShortcutType): string {
  return type === "folder" ? "folder" : type === "application" ? "app" : "file";
}

async function loadWallpaper() {
  const p = await invoke<string | null>("get_wallpaper");
  wallpaperUrl.value = p ? convertFileSrc(p) : "";
}

async function pickFiles() {
  addMenuOpen.value = false;
  const sel = await open({ multiple: true });
  if (Array.isArray(sel)) for (const p of sel) await shortcuts.addPath(p);
  else if (typeof sel === "string") await shortcuts.addPath(sel);
}

async function pickFolders() {
  addMenuOpen.value = false;
  const sel = await open({ directory: true, multiple: true });
  if (Array.isArray(sel)) for (const p of sel) await shortcuts.addPath(p);
  else if (typeof sel === "string") await shortcuts.addPath(sel);
}

function setCardRef(id: string, el: unknown) {
  cardRefs.value[id] = el as HTMLElement | null;
}

function onCardDown(e: PointerEvent, sc: DesktopShortcut) {
  if (e.button !== 0) return;
  const t = e.target as HTMLElement;
  if (t.closest("button")) return;
  dragId.value = sc.id;
  dragMoved.value = false;
  dragStart.x = e.clientX;
  dragStart.y = e.clientY;
  try {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  } catch {
    /* ignore */
  }
}

function onCardMove(e: PointerEvent, sc: DesktopShortcut) {
  if (dragId.value !== sc.id) return;
  if (!dragMoved.value) {
    const dx = e.clientX - dragStart.x;
    const dy = e.clientY - dragStart.y;
    if (dx * dx + dy * dy < 36) return;
    dragMoved.value = true;
  }
  let best: string | null = null;
  let bestDist = Infinity;
  for (const [id, el] of Object.entries(cardRefs.value)) {
    if (id === sc.id || !el) continue;
    const r = el.getBoundingClientRect();
    const cx = r.left + r.width / 2;
    const cy = r.top + r.height / 2;
    const d = (e.clientX - cx) ** 2 + (e.clientY - cy) ** 2;
    if (d < bestDist) {
      bestDist = d;
      best = id;
    }
  }
  if (best && best !== dragOverId.value) {
    dragOverId.value = best;
    const from = shortcuts.items.findIndex((s) => s.id === sc.id);
    const to = shortcuts.items.findIndex((s) => s.id === best);
    if (from !== -1 && to !== -1 && from !== to) {
      const arr = [...shortcuts.items];
      const [moved] = arr.splice(from, 1);
      arr.splice(to, 0, moved);
      shortcuts.items = arr.map((s, i) => ({ ...s, order: i }));
    }
  }
}

async function onCardUp(e: PointerEvent, sc: DesktopShortcut) {
  if (dragId.value !== sc.id) return;
  const wasDrag = dragMoved.value;
  dragId.value = null;
  dragOverId.value = null;
  dragMoved.value = false;
  try {
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  } catch {
    /* ignore */
  }
  if (wasDrag) {
    await shortcuts.reorder(shortcuts.items.map((s) => s.id));
  } else {
    await shortcuts.open(sc);
  }
}

function remove(id: string) {
  void shortcuts.remove(id);
}

function quit() {
  void invoke("quit_app");
}

onMounted(async () => {
  await loadWallpaper();
  await shortcuts.load();
  try {
    const b = await invoke<{ focusSubtitle?: string }>("get_bootstrap");
    if (b.focusSubtitle) ui.focusSubtitle = b.focusSubtitle;
  } catch {
    /* ignore */
  }
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

    <header class="topbar">
      <span class="task">当前任务：实现统计模块</span>
      <span class="agent-status">
        Agent
        <span class="dot" :class="`st-${agent.state}`"></span>
      </span>
      <span class="mode-chip" :class="ui.focusState">{{ modeChip }}</span>
    </header>

    <section class="hero">
      <div class="timer num">{{ timerText }}</div>
      <div class="timer-label">{{ timerLabel }}</div>
      <div class="task-line">{{ ui.focusSubtitle }}</div>
    </section>

    <section class="icon-zone">
      <div class="sc-grid">
        <div
          v-for="sc in shortcuts.items"
          :key="sc.id"
          class="sc-card glass"
          :class="{ dragging: dragId === sc.id, over: dragOverId === sc.id }"
          :ref="(el) => setCardRef(sc.id, el)"
          @pointerdown="onCardDown($event, sc)"
          @pointermove="onCardMove($event, sc)"
          @pointerup="onCardUp($event, sc)"
        >
          <button class="rm" title="移除" @pointerdown.stop @click.stop="remove(sc.id)">
            <AppIcon name="close" />
          </button>
          <span class="glyph" :class="sc.type"><AppIcon :name="glyphFor(sc.type)" /></span>
          <span class="name">{{ sc.name }}</span>
        </div>
        <div class="sc-card add glass" @click="addMenuOpen = !addMenuOpen">
          <span class="glyph"><AppIcon name="plus" /></span>
          <span class="name">添加</span>
        </div>
      </div>
      <div v-if="addMenuOpen" class="menu-backdrop" @click="addMenuOpen = false"></div>
      <div v-if="addMenuOpen" class="add-menu glass">
        <button @click="pickFiles">文件 / 应用</button>
        <button @click="pickFolders">文件夹</button>
      </div>
    </section>

    <footer class="dock glass">
      <button class="dock-btn" @click="ui.toggleFocus()">
        <AppIcon :name="focusIcon" /><span>{{ focusBtnText }}</span>
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
/* wallpaper layers */
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
/* focus "spring" ambient: brighter, more lime sunlight */
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

/* floating glass-capsule top bar */
.topbar {
  position: relative; z-index: 5;
  align-self: center; margin-top: 14px;
  display: inline-flex; align-items: center; gap: 20px;
  padding: 9px 20px;
  border-radius: var(--r-pill);
  background: var(--glass);
  border: 1px solid var(--glass-border);
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.25);
}
.task { font-size: 13px; font-weight: 600; }
.agent-status { font-size: 12px; color: var(--text-mid); display: flex; align-items: center; gap: 6px; }
.dot { width: 8px; height: 8px; border-radius: 50%; background: var(--text-low); }
.dot.st-thinking, .dot.st-reading, .dot.st-searching { background: var(--accent); }
.dot.st-editing, .dot.st-running, .dot.st-testing { background: var(--accent-bright); }
.dot.st-waiting_permission { background: var(--warn); }
.dot.st-success { background: var(--accent); }
.dot.st-error { background: var(--err); }
.mode-chip {
  font-size: 12px; color: var(--text-mid);
  border: 1px solid var(--glass-border); border-radius: var(--r-pill);
  padding: 2px 10px;
}
.mode-chip.focus { color: var(--accent-bright); border-color: rgba(163, 230, 53, 0.4); }
.mode-chip.rest { color: var(--warn); border-color: rgba(251, 191, 36, 0.4); }

.hero {
  position: relative; z-index: 5;
  display: flex; flex-direction: column; align-items: center;
  padding-top: 6vh;
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

/* file-shortcut zone */
.icon-zone {
  position: relative; z-index: 5;
  display: flex; justify-content: center; align-items: flex-start;
  margin-top: 5vh;
}
.sc-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, 104px);
  gap: 14px;
  justify-content: center;
  width: min(880px, 84vw);
}
.sc-card {
  width: 104px; height: 92px;
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: 6px;
  border-radius: var(--r-md);
  cursor: pointer;
  position: relative;
  transition: transform var(--t-base) var(--ease-out), box-shadow var(--t-base) var(--ease-out), border-color var(--t-base) var(--ease-out);
}
.sc-card:hover { transform: translateY(-3px); border-color: var(--accent); box-shadow: 0 8px 24px rgba(163, 230, 53, 0.18); }
.sc-card.dragging { opacity: 0.7; transform: scale(0.96); }
.sc-card.over { border-color: var(--accent); box-shadow: 0 0 0 2px var(--accent-wash); }
.sc-card.add { border: 1px dashed var(--glass-border); color: var(--text-low); }
.sc-card.add:hover { color: var(--accent-bright); border-color: var(--accent); }
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
.name {
  font-size: 12px; color: var(--text-mid);
  max-width: 96px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.menu-backdrop { position: fixed; inset: 0; z-index: 10; }
.add-menu {
  position: absolute; top: calc(100% + 8px); left: 50%; transform: translateX(-50%);
  z-index: 12;
  display: flex; flex-direction: column; gap: 4px;
  padding: 6px; border-radius: var(--r-md);
  box-shadow: 0 10px 28px rgba(0, 0, 0, 0.35);
}
.add-menu button {
  border: none; background: transparent; color: var(--text-hi);
  border-radius: var(--r-sm); padding: 8px 22px; font-size: 13px; cursor: pointer; text-align: left;
}
.add-menu button:hover { background: var(--accent-wash); color: var(--accent-bright); }

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
