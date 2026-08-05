<script setup lang="ts">
import { onMounted, ref } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useSettingsStore } from "../stores/settings";
import AppIcon from "./AppIcon.vue";

const emit = defineEmits<{ (e: "close"): void }>();
const settings = useSettingsStore();

const version = "v0.1.0";
const wallpaperUrl = ref("");
const acrylicOn = ref(true);

// local mirrors for editors
const taskName = ref("");
const taskMinutes = ref<number | null>(null);
const blackText = ref("");
const whiteText = ref("");

const PRESETS = [
  { label: "25/5", focus: 25, rest: 5 },
  { label: "50/10", focus: 50, rest: 10 },
  { label: "90/15", focus: 90, rest: 15 },
];

async function load() {
  await settings.load();
  const b = await invoke<{ acrylicEnabled?: boolean }>("get_bootstrap");
  acrylicOn.value = !!b.acrylicEnabled;
  const p = await invoke<string | null>("get_wallpaper");
  wallpaperUrl.value = p ? convertFileSrc(p) : "";
  taskName.value = settings.currentTask?.name ?? "";
  taskMinutes.value = settings.currentTask?.estimatedMinutes ?? null;
  blackText.value = settings.distractionApps.join("\n");
  whiteText.value = settings.allowedApps.join("\n");
}

async function pickWallpaper() {
  const sel = await open({
    multiple: false,
    filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "webp"] }],
  });
  if (typeof sel === "string") {
    try {
      const saved = await invoke<string>("persist_wallpaper", { src: sel });
      wallpaperUrl.value = convertFileSrc(saved);
    } catch (e) {
      console.error("[settings] wallpaper import failed", e);
    }
  }
}

async function resetWallpaper() {
  await invoke("reset_wallpaper");
  wallpaperUrl.value = "";
}

async function toggleAcrylic() {
  acrylicOn.value = !acrylicOn.value;
  try {
    await invoke("set_acrylic", { enabled: acrylicOn.value });
  } catch (e) {
    console.error("[settings] set_acrylic failed", e);
    acrylicOn.value = !acrylicOn.value;
  }
}

function applyPreset(p: { focus: number; rest: number }) {
  void settings.setFocusDurations(p.focus, p.rest);
}

async function saveTask() {
  const name = taskName.value.trim();
  if (!name) return;
  const id = settings.currentTask?.id ?? `task-${Date.now()}`;
  const saved = await settings.saveTask({
    id,
    name,
    estimatedMinutes: taskMinutes.value ? Math.max(1, taskMinutes.value) : null,
  });
  await settings.setCurrentTask(saved.id);
  taskName.value = saved.name;
}

async function saveLists() {
  const black = blackText.value
    .split("\n")
    .map((s) => s.trim())
    .filter(Boolean);
  const white = whiteText.value
    .split("\n")
    .map((s) => s.trim())
    .filter(Boolean);
  await settings.setDistractionLists(black, white);
}

function onPauseSupervision() {
  void settings.pauseSupervision(30);
}

function onResumeSupervision() {
  void settings.resumeSupervision();
}

onMounted(load);
</script>

<template>
  <div class="popover glass" @click.stop>
    <div class="head">
      <span class="title">设置</span>
      <button class="ghost" title="关闭" @click="emit('close')"><AppIcon name="close" /></button>
    </div>

    <section class="group">
      <h4>壁纸</h4>
      <div class="row">
        <div class="btns">
          <button @click="pickWallpaper">导入</button>
          <button @click="resetWallpaper">重置</button>
        </div>
        <span v-if="wallpaperUrl" class="ok">已设置</span>
      </div>
    </section>

    <section class="group">
      <h4>外观</h4>
      <div class="row">
        <span class="label">毛玻璃</span>
        <button class="switch" :class="{ on: acrylicOn }" @click="toggleAcrylic">
          {{ acrylicOn ? "开" : "关" }}
        </button>
      </div>
      <div class="row">
        <span class="label">提示音</span>
        <button class="switch" :class="{ on: settings.soundEnabled }" @click="settings.setSound(!settings.soundEnabled)">
          {{ settings.soundEnabled ? "开" : "关" }}
        </button>
      </div>
      <div class="row">
        <span class="label">顶条</span>
        <div class="seg">
          <button :class="{ on: settings.showTopbar === 'auto' }" @click="settings.setShowTopbar('auto')">自动</button>
          <button :class="{ on: settings.showTopbar === 'on' }" @click="settings.setShowTopbar('on')">常显</button>
          <button :class="{ on: settings.showTopbar === 'off' }" @click="settings.setShowTopbar('off')">隐藏</button>
        </div>
      </div>
    </section>

    <section class="group">
      <h4>计时（下一轮生效）</h4>
      <div class="row presets">
        <button v-for="p in PRESETS" :key="p.label" @click="applyPreset(p)">{{ p.label }}</button>
      </div>
      <div class="row">
        <span class="label">专注</span>
        <input v-model.number="settings.focusMinutes" type="number" min="1" max="240" class="num-input" />
        <span class="unit">分钟</span>
      </div>
      <div class="row">
        <span class="label">休息</span>
        <input v-model.number="settings.restMinutes" type="number" min="1" max="120" class="num-input" />
        <span class="unit">分钟</span>
      </div>
      <div class="row">
        <button class="btn" @click="settings.setFocusDurations(settings.focusMinutes, settings.restMinutes)">应用时长</button>
      </div>
    </section>

    <section class="group">
      <h4>任务</h4>
      <div class="row">
        <span class="label">名称</span>
        <input v-model="taskName" type="text" class="text-input" placeholder="当前任务" />
      </div>
      <div class="row">
        <span class="label">预计</span>
        <input v-model.number="taskMinutes" type="number" min="1" max="600" class="num-input" placeholder="分钟" />
      </div>
      <div class="row">
        <button class="btn" @click="saveTask">保存任务</button>
      </div>
    </section>

    <section class="group">
      <h4>监督</h4>
      <div class="row">
        <span class="label">启用</span>
        <button class="switch" :class="{ on: settings.supervisionEnabled }" @click="settings.setSupervisionEnabled(!settings.supervisionEnabled)">
          {{ settings.supervisionEnabled ? "开" : "关" }}
        </button>
      </div>
      <div class="row">
        <span class="label">暂停</span>
        <button v-if="!settings.supervisionPaused" class="btn" @click="onPauseSupervision">暂停 30 分钟</button>
        <button v-else class="btn" @click="onResumeSupervision">恢复监督</button>
      </div>
      <div class="row col">
        <span class="label">分心应用（每行一个，支持 *通配*）</span>
        <textarea v-model="blackText" rows="3" class="ta"></textarea>
      </div>
      <div class="row col">
        <span class="label">豁免应用（每行一个）</span>
        <textarea v-model="whiteText" rows="2" class="ta"></textarea>
      </div>
      <div class="row">
        <button class="btn" @click="saveLists">保存清单</button>
      </div>
    </section>

    <div class="about">Focus Desktop {{ version }} · MIT</div>
  </div>
</template>

<style scoped>
.popover {
  position: absolute;
  right: 24px;
  bottom: 72px;
  width: 300px;
  max-height: 78vh;
  overflow-y: auto;
  z-index: 30;
  padding: 12px 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  border-radius: var(--r-md);
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.35);
}
.head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.title { font-size: 13px; font-weight: 600; color: var(--text-hi); }
.ghost {
  border: none; background: transparent; color: var(--text-mid);
  border-radius: var(--r-sm); padding: 3px; cursor: pointer; display: inline-flex;
}
.ghost:hover { color: var(--accent); background: var(--accent-wash); }
.group { display: flex; flex-direction: column; gap: 8px; padding-top: 8px; border-top: 1px solid var(--glass-border); }
.group h4 { margin: 0; font-size: 11px; color: var(--text-low); font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; }
.row { display: flex; align-items: center; gap: 8px; }
.row.col { flex-direction: column; align-items: stretch; }
.label { font-size: 12px; color: var(--text-mid); flex-shrink: 0; }
.btns { display: flex; gap: 6px; }
.btns button, .btn {
  border: 1px solid var(--glass-border); background: var(--glass-strong);
  color: var(--text-hi); border-radius: var(--r-sm); padding: 4px 10px;
  font-size: 12px; cursor: pointer;
}
.btns button:hover, .btn:hover { border-color: var(--accent); color: var(--accent-bright); }
.ok { font-size: 11px; color: var(--accent-bright); }
.switch {
  border: 1px solid var(--glass-border); background: var(--glass-strong);
  color: var(--text-low); border-radius: var(--r-pill); padding: 4px 14px;
  font-size: 12px; cursor: pointer;
}
.switch.on { background: var(--accent); color: #0a110e; border-color: var(--accent); font-weight: 600; }
.seg { display: flex; gap: 4px; }
.seg button {
  border: 1px solid var(--glass-border); background: transparent; color: var(--text-mid);
  border-radius: var(--r-sm); padding: 3px 10px; font-size: 12px; cursor: pointer;
}
.seg button.on { background: var(--accent-wash); color: var(--accent-bright); border-color: var(--accent); }
.presets { gap: 6px; }
.presets button {
  border: 1px solid var(--glass-border); background: transparent; color: var(--text-mid);
  border-radius: var(--r-sm); padding: 3px 12px; font-size: 12px; cursor: pointer;
}
.presets button:hover { border-color: var(--accent); color: var(--accent-bright); }
.num-input, .text-input, .ta {
  border: 1px solid var(--glass-border); background: var(--glass-strong);
  color: var(--text-hi); border-radius: var(--r-sm); padding: 4px 8px; font-size: 12px;
  font-family: inherit;
}
.num-input { width: 64px; }
.text-input { flex: 1; min-width: 0; }
.ta { width: 100%; resize: vertical; }
.unit { font-size: 11px; color: var(--text-low); }
.about { font-size: 11px; color: var(--text-low); border-top: 1px solid var(--glass-border); padding-top: 8px; }
</style>
