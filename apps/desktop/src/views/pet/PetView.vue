<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { useAgentStore } from "../../stores/agent";
import { useSettingsStore } from "../../stores/settings";
import { useUiStore } from "../../stores/ui";
import { useGridDrag } from "../../composables/useGridDrag";
import {
  PetRequestCoordinator,
  type PetPackageRequest,
  petCanvasMetrics,
  replacePetBitmap,
} from "../../lib/pet-render";

interface AnimDef {
  columns: number;
  rows: number;
  frames: number;
  fps: number;
  loop: boolean;
  startRow: number;
  cellWidth: number;
  cellHeight: number;
  sourceRect: { x: number; y: number; width: number; height: number };
}

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
  animations: Array<{ id: string; cellWidth: number; cellHeight: number }>;
  hostTint: string;
  bubbleAccent: string;
  horizontalCorrection: number;
  qualityWarnings: string[];
}

const agent = useAgentStore();
const ui = useUiStore();
const settingsStore = useSettingsStore();
const {
  onPointerDown,
  onPointerMove,
  onPointerUp,
  onPointerCancel,
  onLostPointerCapture,
} = useGridDrag("pet");

// ---- pet pack state ----
const pet = ref<PetInfo | null>(null);
const sheet = ref<ImageBitmap | null>(null);
const sheetError = ref("");
const hovered = ref(false);
const petRequests = new PetRequestCoordinator();

// ---- sprite playback ----
const canvasRef = ref<HTMLCanvasElement | null>(null);
const stageRef = ref<HTMLElement | null>(null);
const frameIdx = ref(0);
const animKey = computed(() => agent.petState);
let currentAnim: AnimDef | null = null;
let timer: ReturnType<typeof setTimeout> | null = null;

function drawFrame(idx: number) {
  const canvas = canvasRef.value;
  const img = sheet.value;
  if (!canvas || !img) return;
  const def = currentAnim;
  if (!def) return;
  const col = idx % def.columns;
  const row = def.startRow + Math.floor(idx / def.columns);
  const ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  const sourceW = def.cellWidth;
  const sourceH = def.cellHeight;
  const source = def.sourceRect;
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  ctx.drawImage(
    img,
    col * sourceW + source.x,
    row * sourceH + source.y,
    source.width,
    source.height,
    0,
    0,
    canvas.width,
    canvas.height,
  );
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
  const def = currentAnim;
  if (!def) return;
  const d = Math.max(16, Math.round(1000 / def.fps));
  timer = setTimeout(() => {
    let next = idx + 1;
    if (next >= def.frames) {
      if (def.loop) {
        next = 0;
      } else {
        frameIdx.value = 0;
        scheduleNext(0);
        return;
      }
    }
    frameIdx.value = next;
    drawFrame(next);
    scheduleNext(next);
  }, d);
}

interface AnimationPayload {
  animation: { assetPath: string; columns: number; rows: number; frames: number; fps: number; looped: boolean; startRow: number; cellWidth: number; cellHeight: number };
  sourceRect: { x: number; y: number; width: number; height: number };
  horizontalCorrection: number;
}

function applyAnimation(payload: AnimationPayload) {
  const source = payload.animation;
  currentAnim = { columns: source.columns, rows: source.rows, frames: source.frames, fps: source.fps, loop: source.looped, startRow: source.startRow, cellWidth: source.cellWidth, cellHeight: source.cellHeight, sourceRect: payload.sourceRect };
  if (pet.value) pet.value.horizontalCorrection = payload.horizontalCorrection;
  fitCanvas();
  frameIdx.value = 0;
  drawFrame(0);
  scheduleNext(0);
}

async function resetPlayback() {
  if (timer) clearTimeout(timer);
  const characterId = agent.characterId;
  if (!characterId) return;
  const request = petRequests.beginAnimation(characterId);
  if (!request) return;
  const payload = await invoke<AnimationPayload>("pet_animation_data", {
    characterId,
    petState: animKey.value,
  });
  if (!petRequests.isCurrentAnimation(request) || agent.characterId !== characterId || pet.value?.id !== request.petId) return;
  applyAnimation(payload);
}

async function decodeSheet(data: string, assetPath: string) {
  const bin = atob(data);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  const blob = new Blob([bytes], { type: assetPath.toLowerCase().endsWith(".png") ? "image/png" : "image/webp" });
  return createImageBitmap(blob);
}

watch(animKey, () => {
  if (pet.value) void resetPlayback();
});

// ---- load active pack / sheet ----
async function loadSheet(info: PetInfo, request: PetPackageRequest) {
  // v1.10 (#32): load pixels same-origin (base64 -> Blob -> createImageBitmap)
  // so canvas getImageData (edge fade) is not blocked by cross-origin taint.
  const characterId = request.characterId;
  const requestedState = animKey.value;
  const [data, payload] = await Promise.all([
    invoke<string>("pet_sheet_data", { characterId }),
    invoke<AnimationPayload>("pet_animation_data", { characterId, petState: requestedState }),
  ]);
  const bitmap = await decodeSheet(data, payload.animation.assetPath);
  if (!petRequests.isCurrentPackage(request) || agent.characterId !== characterId) {
    bitmap.close();
    return;
  }
  if (!petRequests.commitPackage(request, info.id)) {
    bitmap.close();
    return;
  }
  sheetError.value = "";
  pet.value = info;
  sheet.value = replacePetBitmap(sheet.value, bitmap);
  await nextTick();
  if (!petRequests.isCurrentPackage(request) || agent.characterId !== characterId || pet.value?.id !== info.id) {
    return;
  }
  observePetStage();
  if (animKey.value === requestedState) {
    applyAnimation(payload);
  } else {
    void resetPlayback();
  }
}

async function refresh() {
  const characterId = agent.characterId ?? "";
  const request = petRequests.beginPackage(characterId);
  try {
    const active = await invoke<PetInfo | null>("pet_active");
    if (!petRequests.isCurrentPackage(request) || agent.characterId !== characterId) return;
    if (active) {
      await loadSheet(active, request);
    } else {
      if (!petRequests.clearPackage(request)) return;
      sheetError.value = "";
      pet.value = null;
      sheet.value = replacePetBitmap(sheet.value, null);
      resizeObserver?.disconnect();
    }
  } catch (e) {
    if (!petRequests.isCurrentPackage(request) || agent.characterId !== characterId) return;
    if (!petRequests.clearPackage(request)) return;
    sheetError.value = String(e);
    pet.value = null;
    sheet.value = replacePetBitmap(sheet.value, null);
    resizeObserver?.disconnect();
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

function toggleChat() {
  hovered.value = false;
  void emit("ui:toggle_chat", {});
}

let resizeObserver: ResizeObserver | null = null;

function observePetStage() {
  if (!resizeObserver) return;
  resizeObserver.disconnect();
  if (stageRef.value) resizeObserver.observe(stageRef.value);
}

function onPetPointerUp() {
  onPointerUp();
}

function onPetPointerCancel() {
  onPointerCancel();
}

function onPetLostPointerCapture() {
  onLostPointerCapture();
}

function fitCanvas() {
  const canvas = canvasRef.value;
  const wrap = stageRef.value;
  if (!canvas || !wrap) return;
  const anim = currentAnim;
  if (!anim) return;
  const frame = petCanvasMetrics(
    anim.sourceRect.width,
    anim.sourceRect.height,
    wrap.clientWidth,
    wrap.clientHeight,
    8,
    window.devicePixelRatio,
    pet.value?.horizontalCorrection ?? 1,
  );
  canvas.style.width = `${frame.cssWidth}px`;
  canvas.style.height = `${frame.cssHeight}px`;
  canvas.width = frame.backingWidth;
  canvas.height = frame.backingHeight;
  drawFrame(frameIdx.value);
}

let unlistenPet: (() => void) | null = null;

onMounted(async () => {
  unlistenPet = await listen("pet:changed", () => void refresh());
  resizeObserver = new ResizeObserver(() => fitCanvas());
  await refresh();
  await nextTick();
  observePetStage();
  fitCanvas();
});

onBeforeUnmount(() => {
  petRequests.invalidate();
  unlistenPet?.();
  if (timer) clearTimeout(timer);
  sheet.value = replacePetBitmap(sheet.value, null);
  resizeObserver?.disconnect();
});
</script>

<template>
  <div
    v-if="pet"
    class="pet-window"
    :style="{ '--pet-host-tint': pet.hostTint, '--pet-accent': pet.bubbleAccent }"
    @pointerdown="onPointerDown"
    @pointermove="onPointerMove"
    @pointerup="onPetPointerUp"
    @pointercancel="onPetPointerCancel"
    @lostpointercapture="onPetLostPointerCapture"
    @mouseenter="hovered = true"
    @mouseleave="hovered = false"
  >
    <div v-if="hovered && pet?.displayName" class="pet-name" data-no-drag>{{ pet.displayName }}</div>

    <div ref="stageRef" class="pet-stage">
      <canvas
        v-if="sheet"
        ref="canvasRef"
        class="pet-canvas"
        :title="pet?.displayName ?? ''"
      ></canvas>
      <div v-else class="sprout" :class="`anim-${agent.petState}`">
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
  background:
    linear-gradient(135deg,
      color-mix(in srgb, var(--pet-host-tint, #122018) 58%, transparent),
      color-mix(in srgb, var(--pet-host-tint, #122018) 42%, transparent));
  border: 1px solid color-mix(in srgb, var(--pet-host-tint, #122018) 44%, rgba(255, 255, 255, 0.68));
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.22), 0 8px 22px rgba(8, 20, 12, 0.22);
  backdrop-filter: blur(18px) saturate(125%);
  -webkit-backdrop-filter: blur(18px) saturate(125%);
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
  display: block;
  flex: none;
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
