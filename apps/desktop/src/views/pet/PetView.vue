<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { useAgentStore } from "../../stores/agent";
import { useSettingsStore } from "../../stores/settings";
import { useUiStore } from "../../stores/ui";
import { useGridDrag } from "../../composables/useGridDrag";

// Official hatch-pet contract (ADR-0009): fixed 8x9 atlas, 192x208 cells.
const ATLAS_COLS = 8;
const ATLAS_ROWS = 9;
const CELL_W = 192;
const CELL_H = 208;
const ATLAS_W = ATLAS_COLS * CELL_W; // 1536
const ATLAS_H = ATLAS_ROWS * CELL_H; // 1872

interface AnimDef {
  row: number;
  durations: number[]; // per-frame ms; last entry holds the final frame
  loop: boolean;
}

// App animation name -> official hatch-pet row (animation-rows.md).
const ANIMS: Record<string, AnimDef> = {
  idle: { row: 0, durations: [280, 110, 110, 140, 140, 320], loop: true },
  thinking: { row: 7, durations: [120, 120, 120, 120, 120, 120, 120, 220], loop: true }, // running
  editing: { row: 8, durations: [150, 150, 150, 150, 150, 280], loop: true }, // review
  waiting: { row: 6, durations: [150, 150, 150, 150, 150, 260], loop: true },
  success: { row: 4, durations: [140, 140, 140, 140, 280], loop: false }, // jumping
  error: { row: 5, durations: [140, 140, 140, 140, 140, 140, 140, 240], loop: false }, // failed
};

const SIZES: Array<[number, number]> = [
  [1, 1],
  [1, 2],
  [2, 1],
  [2, 2],
];

interface PetInfo {
  id: string;
  displayName: string;
  description: string;
  spritesheetPath: string;
}

const agent = useAgentStore();
const ui = useUiStore();
const settingsStore = useSettingsStore();
const { onPointerDown, onPointerMove, onPointerUp } = useGridDrag("pet");

// ---- pet pack state ----
const pet = ref<PetInfo | null>(null);
const sheet = ref<HTMLImageElement | ImageBitmap | null>(null);
const sheetError = ref("");
const hovered = ref(false);

// ---- sprite playback ----
const canvasRef = ref<HTMLCanvasElement | null>(null);
const frameIdx = ref(0);
const animKey = computed(() => ANIMS[agent.animation] ? agent.animation : "idle");
// Actually playing animation; non-loop animations switch back to "idle" here.
let currentAnim = "idle" as string;
let timer: ReturnType<typeof setTimeout> | null = null;

function drawFrame(idx: number) {
  const canvas = canvasRef.value;
  const img = sheet.value;
  if (!canvas || !img) return;
  const def = ANIMS[currentAnim] ?? ANIMS.idle;
  const col = idx % ATLAS_COLS;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.drawImage(img, col * CELL_W, def.row * CELL_H, CELL_W, CELL_H, 0, 0, canvas.width, canvas.height);
  if (settingsStore.petBgFade) applyEdgeFade(ctx);
}

/** Soften the outermost ring so any remnant background blends into the
 *  wallpaper (optional, controlled by Settings -> petBgFade). */
function applyEdgeFade(ctx: CanvasRenderingContext2D) {
  try {
    const w = ctx.canvas.width;
    const h = ctx.canvas.height;
    const f = Math.max(2, Math.round(Math.min(w, h) * 0.05));
    const img = ctx.getImageData(0, 0, w, h);
    const d = img.data;
    for (let y = 0; y < h; y++) {
      const vf = Math.max(y < f ? 1 - y / f : 0, y >= h - f ? 1 - (h - 1 - y) / f : 0);
      for (let x = 0; x < w; x++) {
        const hf = Math.max(x < f ? 1 - x / f : 0, x >= w - f ? 1 - (w - 1 - x) / f : 0);
        const t = Math.max(vf, hf);
        if (t > 0) {
          const i = (y * w + x) * 4;
          d[i + 3] = Math.round(d[i + 3] * (1 - t * 0.8));
        }
      }
    }
    ctx.putImageData(img, 0, 0);
  } catch {
    // v1.10 (#32): never let a tainted canvas break pet playback; skip fade.
  }
}

function scheduleNext(idx: number) {
  if (timer) clearTimeout(timer);
  const def = ANIMS[currentAnim] ?? ANIMS.idle;
  const d = def.durations[Math.min(idx, def.durations.length - 1)];
  timer = setTimeout(() => {
    let next = idx + 1;
    if (next >= def.durations.length) {
      if (def.loop) {
        next = 0;
      } else {
        // non-loop finished: snap back to idle
        currentAnim = "idle";
        frameIdx.value = 0;
        drawFrame(0);
        scheduleIdleLoop();
        return;
      }
    }
    frameIdx.value = next;
    drawFrame(next);
    scheduleNext(next);
  }, d);
}

function scheduleIdleLoop() {
  if (timer) clearTimeout(timer);
  const def = ANIMS.idle;
  const d = def.durations[Math.min(frameIdx.value, def.durations.length - 1)];
  timer = setTimeout(() => {
    const next = (frameIdx.value + 1) % def.durations.length;
    frameIdx.value = next;
    drawFrame(next);
    scheduleIdleLoop();
  }, d);
}

function resetPlayback() {
  if (timer) clearTimeout(timer);
  currentAnim = animKey.value;
  frameIdx.value = 0;
  drawFrame(0);
  scheduleNext(0);
}

watch(animKey, () => {
  if (sheet.value) resetPlayback();
});

// ---- load active pack / sheet ----
async function loadSheet(info: PetInfo) {
  sheetError.value = "";
  // v1.10 (#32): load pixels same-origin (base64 -> Blob -> createImageBitmap)
  // so canvas getImageData (edge fade) is not blocked by cross-origin taint.
  const b64 = await invoke<string>("pet_sheet_data", { id: info.id });
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  const mime = info.spritesheetPath.toLowerCase().endsWith(".png") ? "image/png" : "image/webp";
  const blob = new Blob([bytes], { type: mime });
  const bmp = await createImageBitmap(blob);
  if (bmp.width !== ATLAS_W || bmp.height !== ATLAS_H) {
    throw new Error(`spritesheet 尺寸不符：需要 ${ATLAS_W}x${ATLAS_H}，实际 ${bmp.width}x${bmp.height}`);
  }
  sheet.value = bmp;
  pet.value = info;
  resetPlayback();
}

async function refresh() {
  try {
    const active = await invoke<PetInfo | null>("pet_active");
    if (active) {
      await loadSheet(active);
    } else {
      pet.value = null;
      sheet.value = null;
    }
  } catch (e) {
    sheetError.value = String(e);
    pet.value = null;
    sheet.value = null;
  }
  try {
    const b = await invoke<{ grid?: Record<string, { cols: number; rows: number }> }>("get_bootstrap");
    const g = b.grid?.pet;
    if (g) {
      const i = SIZES.findIndex(([c, r]) => c === g.cols && r === g.rows);
      if (i >= 0) sizeIdx = i;
    }
  } catch (e) {
    console.error("[pet] bootstrap failed", e);
  }
}

// ---- resize handle (v1.10.3 #43): nearest-corner snapping. Sizes are
// ordered by area (1x1 < 1x2/2x1 < 2x2); the target size is the one whose
// bottom-right corner is closest to the pointer (Pythagorean distance), so
// small drags stay put and diagonal drags count. Preview while held, commit
// on release. ----
let sizeIdx = 0;
let startSizeIdx = 0;
let targetSizeIdx = 0;
let winLeft = 0;
let winTop = 0;
let winW = 0;
let winH = 0;
let curCols = 1;
let curRows = 1;
let resizePointer = -1;
let resizeChanged = false;

function showResizePreview(idx: number) {
  const [cols, rows] = SIZES[idx];
  void invoke("resize_preview", { label: "pet", visible: true, cols, rows }).catch((err) =>
    console.error("[pet] resize preview failed", err),
  );
}

function onResizePointerDown(e: PointerEvent) {
  if (resizePointer !== -1) return;
  resizePointer = e.pointerId;
  startSizeIdx = sizeIdx;
  targetSizeIdx = sizeIdx;
  resizeChanged = false;
  const [cc, cr] = SIZES[sizeIdx];
  curCols = cc;
  curRows = cr;
  winLeft = window.screenX;
  winTop = window.screenY;
  winW = window.outerWidth;
  winH = window.outerHeight;
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  showResizePreview(targetSizeIdx);
}

function onResizePointerMove(e: PointerEvent) {
  if (e.pointerId !== resizePointer) return;
  let best = targetSizeIdx;
  let bestDist = Infinity;
  for (let i = 0; i < SIZES.length; i++) {
    const [c, r] = SIZES[i];
    const cx = winLeft + (winW * c) / curCols;
    const cy = winTop + (winH * r) / curRows;
    const d = Math.hypot(e.screenX - cx, e.screenY - cy);
    if (d < bestDist) {
      bestDist = d;
      best = i;
    }
  }
  if (best !== targetSizeIdx) {
    targetSizeIdx = best;
    resizeChanged = true;
    showResizePreview(targetSizeIdx);
  }
}

async function onResizePointerUp(e: PointerEvent) {
  if (e.pointerId !== resizePointer) return;
  resizePointer = -1;
  void invoke("resize_preview", { label: "pet", visible: false }).catch(() => undefined);
  if (!resizeChanged) return;
  sizeIdx = targetSizeIdx;
  const [cols, rows] = SIZES[sizeIdx];
  try {
    await invoke("resize_window", { label: "pet", cols, rows });
  } catch (err) {
    // Conflict: the window stayed at its original size; revert to the start slot.
    sizeIdx = startSizeIdx;
    console.error("[pet] resize rejected", err);
  }
}

// The fullscreen grid overlay used to steal activation on show, which made
// the browser fire pointercancel/lostpointercapture instead of pointerup.
// Treat either as a release so the handle never gets stuck and the preview
// is always hidden.
function onResizeCancel() {
  if (resizePointer === -1) return;
  resizePointer = -1;
  void invoke("resize_preview", { label: "pet", visible: false }).catch(() => undefined);
  if (!resizeChanged) return;
  sizeIdx = targetSizeIdx;
  const [cols, rows] = SIZES[sizeIdx];
  void invoke("resize_window", { label: "pet", cols, rows }).catch((err) => {
    sizeIdx = startSizeIdx;
    console.error("[pet] resize rejected", err);
  });
}

// ---- bubble / chat ----
const bubbleVisible = computed(() => {
  if (!agent.bubble) return false;
  if (ui.chatOpen) return false;
  if (ui.focusState === "focus" && (agent.bubble.priority === "low" || agent.bubble.priority === "normal")) {
    return false;
  }
  return Date.now() < agent.bubble.expiresAt;
});

function toggleChat() {
  hovered.value = false;
  void emit("ui:toggle_chat", {});
}

let resizeObserver: ResizeObserver | null = null;

function fitCanvas() {
  const canvas = canvasRef.value;
  const wrap = canvas?.parentElement;
  if (!canvas || !wrap) return;
  const availW = wrap.clientWidth - 16;
  const availH = wrap.clientHeight - 16;
  const scale = Math.min(availW / CELL_W, availH / CELL_H);
  const w = Math.max(16, Math.floor(CELL_W * scale));
  const h = Math.max(18, Math.floor(CELL_H * scale));
  canvas.width = w;
  canvas.height = h;
  drawFrame(frameIdx.value);
}

let unlistenPet: (() => void) | null = null;

onMounted(async () => {
  unlistenPet = await listen("pet:changed", () => void refresh());
  await refresh();
  resizeObserver = new ResizeObserver(() => fitCanvas());
  if (canvasRef.value?.parentElement) resizeObserver.observe(canvasRef.value.parentElement);
  fitCanvas();
});

onBeforeUnmount(() => {
  unlistenPet?.();
  if (timer) clearTimeout(timer);
  resizeObserver?.disconnect();
});
</script>

<template>
  <div
    class="pet-window"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPointerUp"
    @mouseenter="hovered = true"
    @mouseleave="hovered = false"
  >
    <div v-if="bubbleVisible" class="bubble" :class="`prio-${agent.bubble?.priority}`" data-no-drag>
      {{ agent.bubble?.text }}
    </div>

    <div v-if="hovered && pet?.displayName" class="pet-name" data-no-drag>{{ pet.displayName }}</div>

    <div class="pet-stage">
      <canvas
        v-if="sheet"
        ref="canvasRef"
        class="pet-canvas"
        :title="pet?.displayName ?? ''"
      ></canvas>
      <div v-else class="sprout" :class="`anim-${agent.animation}`">
        <svg viewBox="0 0 64 64" width="72" height="72">
          <path d="M32 58 C32 42 32 30 32 22" stroke="#a3e635" stroke-width="3" fill="none" stroke-linecap="round" />
          <path d="M32 34 C20 30 15 20 19 11 C28 11 34 21 32 34Z" fill="#4ade80" />
          <path d="M32 26 C44 22 49 14 45 6 C37 8 31 16 32 26Z" fill="#a3e635" />
          <path d="M30 46 C22 44 18 38 20 32 C26 33 30 38 30 46Z" fill="#16a34a" />
        </svg>
        <span class="halo" :class="`st-${agent.state}`"></span>
      </div>
      <div v-if="sheetError" class="sheet-err">{{ sheetError }}</div>
    </div>

    <button v-if="hovered && !ui.chatOpen" class="chat-btn" data-no-drag @click="toggleChat">对话</button>

    <div
      class="resize-handle"
      data-no-drag
      @pointerdown="onResizePointerDown"
      @pointermove="onResizePointerMove"
      @pointerup="onResizePointerUp"
      @pointercancel="onResizeCancel"
      @lostpointercapture="onResizeCancel"
      title="长按并拖动调整桌宠大小（1×1 / 1×2 / 2×1 / 2×2）"
    ></div>
  </div>
</template>

<style scoped>
.pet-window {
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  box-sizing: border-box;
  cursor: grab;
  border-radius: var(--window-host-radius);
  overflow: hidden;
}
.pet-stage {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: min(8%, 8px);
  box-sizing: border-box;
  position: relative;
}
.pet-canvas {
  max-width: 100%;
  max-height: 100%;
}
.sprout { position: relative; display: flex; align-items: center; justify-content: center; }
.halo {
  position: absolute; inset: -6px;
  border-radius: 50%;
  border: 2px solid rgba(163, 230, 53, 0.45);
  filter: blur(1px);
}
.halo.st-waiting_permission { border-color: var(--warn); }
.halo.st-error { border-color: var(--err); }
.anim-thinking { animation: sway 0.6s ease-in-out infinite alternate; }
.anim-editing { animation: shake 0.3s ease-in-out infinite alternate; }
.anim-waiting { animation: pulse 1.2s ease-in-out infinite; }
.anim-success { animation: bloom 0.6s ease-out 1; }
.anim-error { transform: rotate(-8deg); filter: grayscale(0.5); }
@keyframes sway { from { transform: rotate(-5deg); } to { transform: rotate(5deg); } }
@keyframes shake { from { transform: translateX(-3px); } to { transform: translateX(3px); } }
@keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.55; } }
@keyframes bloom { 0% { transform: scale(0.8); } 60% { transform: scale(1.12); } 100% { transform: scale(1); } }
.bubble {
  position: absolute; top: 2px; left: 50%; transform: translateX(-50%);
  max-width: 150px; background: #eef7e6; color: #12211a;
  border-radius: 10px; padding: 5px 10px; font-size: 12px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis; z-index: 5;
}
.bubble.prio-high, .bubble.prio-critical { border: 2px solid var(--warn); }
.pet-name {
  position: absolute; top: 4px; left: 50%; transform: translateX(-50%);
  font-size: 11px; color: var(--text-hi); background: var(--glass-strong);
  border: 1px solid var(--glass-border); border-radius: var(--r-pill);
  padding: 2px 10px; max-width: 70%;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap; z-index: 6;
}
.chat-btn {
  position: absolute; right: 12px; bottom: 24px;
  border: 1px solid var(--glass-border); border-radius: var(--r-pill);
  padding: 4px 12px; font-size: 12px; cursor: pointer;
  background: var(--glass-strong); color: var(--accent-bright); z-index: 6;
}
.chat-btn:hover { border-color: var(--accent); }
.sheet-err {
  position: absolute; bottom: 4px; left: 8px; right: 8px;
  font-size: 10px; color: var(--err);
  text-align: center; word-break: break-all;
}
.resize-handle {
  position: absolute; right: 2px; bottom: 2px;
  width: 14px; height: 14px;
  cursor: nwse-resize;
  border-right: 2px solid var(--text-low);
  border-bottom: 2px solid var(--text-low);
  border-bottom-right-radius: 3px;
  opacity: 0.55;
}
.resize-handle:hover { opacity: 1; border-color: var(--accent-bright); }
</style>
