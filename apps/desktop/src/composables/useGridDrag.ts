import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { PhysicalPosition } from "@tauri-apps/api/dpi";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { GridMetrics } from "../lib/grid";

const win = getCurrentWebviewWindow();

let dragging = false;
let scale = 1;
let startScreen = { x: 0, y: 0 };
let startPos = { x: 0, y: 0 };
let curPos = { x: 0, y: 0 };
let metrics: GridMetrics | null = null;
let lastCell = "";

/**
 * Custom 12x8 grid drag: window follows the pointer (physical px), the grid
 * overlay shows the target cell live, and on release Rust validates occupancy
 * + min-width guardrail and snaps/persists (snaps back when occupied).
 */
export function useGridDrag(label: string) {
  async function onPointerDown(e: PointerEvent) {
    const t = e.target as HTMLElement;
    if (t.closest("button, input, a, [data-no-drag]")) return;
    dragging = true;
    startScreen = { x: e.screenX, y: e.screenY };
    scale = await win.scaleFactor();
    const p = await win.outerPosition();
    startPos = { x: p.x, y: p.y };
    curPos = { ...startPos };
    metrics = await invoke<GridMetrics>("get_grid_metrics");
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    void emit("grid:drag_start", { label });
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const dx = (e.screenX - startScreen.x) * scale;
    const dy = (e.screenY - startScreen.y) * scale;
    curPos = { x: startPos.x + dx, y: startPos.y + dy };
    void win.setPosition(new PhysicalPosition(curPos.x, curPos.y));
    if (metrics) {
      const lx = curPos.x / scale;
      const ly = curPos.y / scale;
      const col = Math.round(lx / metrics.cellW);
      const row = Math.round(ly / metrics.cellH);
      const key = `${col}:${row}`;
      if (key !== lastCell) {
        lastCell = key;
        void emit("grid:drag_move", { label, col, row });
      }
    }
  }

  async function onPointerUp() {
    if (!dragging) return;
    dragging = false;
    lastCell = "";
    if (metrics) {
      const lx = curPos.x / scale;
      const ly = curPos.y / scale;
      const col = Math.max(0, Math.round(lx / metrics.cellW));
      const row = Math.max(0, Math.round(ly / metrics.cellH));
      await invoke("place_window", { label, col, row });
    }
    void emit("grid:drag_end", { label });
  }

  return { onPointerDown, onPointerMove, onPointerUp };
}