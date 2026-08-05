<script setup lang="ts">
import { onMounted, ref } from "vue";
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import AppIcon from "./AppIcon.vue";

const emit = defineEmits<{ (e: "close"): void }>();

const wallpaperUrl = ref("");
const acrylicOn = ref(true);
const version = "v0.1.0";

async function load() {
  const b = await invoke<{ acrylicEnabled?: boolean }>("get_bootstrap");
  acrylicOn.value = !!b.acrylicEnabled;
  const p = await invoke<string | null>("get_wallpaper");
  wallpaperUrl.value = p ? convertFileSrc(p) : "";
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

onMounted(load);
</script>

<template>
  <div class="popover glass" @click.stop>
    <div class="head">
      <span class="title">设置</span>
      <button class="ghost" title="关闭" @click="emit('close')"><AppIcon name="close" /></button>
    </div>
    <div class="row">
      <span class="label">壁纸</span>
      <div class="btns">
        <button @click="pickWallpaper">导入</button>
        <button @click="resetWallpaper">重置</button>
      </div>
      <span v-if="wallpaperUrl" class="ok">已设置</span>
    </div>
    <div class="row">
      <span class="label">毛玻璃</span>
      <button class="switch" :class="{ on: acrylicOn }" @click="toggleAcrylic">
        {{ acrylicOn ? "开" : "关" }}
      </button>
    </div>
    <div class="about">Focus Desktop {{ version }} · MIT</div>
  </div>
</template>

<style scoped>
.popover {
  position: absolute;
  right: 24px;
  bottom: 72px;
  width: 260px;
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
.row {
  display: flex; align-items: center; gap: 8px;
}
.label { font-size: 12px; color: var(--text-mid); width: 48px; flex-shrink: 0; }
.btns { display: flex; gap: 6px; }
.btns button {
  border: 1px solid var(--glass-border); background: var(--glass-strong);
  color: var(--text-hi); border-radius: var(--r-sm); padding: 4px 10px;
  font-size: 12px; cursor: pointer;
}
.btns button:hover { border-color: var(--accent); color: var(--accent-bright); }
.ok { font-size: 11px; color: var(--accent-bright); }
.switch {
  border: 1px solid var(--glass-border); background: var(--glass-strong);
  color: var(--text-low); border-radius: var(--r-pill); padding: 4px 14px;
  font-size: 12px; cursor: pointer;
}
.switch.on { background: var(--accent); color: #0a110e; border-color: var(--accent); font-weight: 600; }
.about { font-size: 11px; color: var(--text-low); border-top: 1px solid var(--glass-border); padding-top: 8px; }
</style>
