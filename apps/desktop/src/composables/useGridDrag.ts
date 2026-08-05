import { invoke } from "@tauri-apps/api/core";

let dragging = false;

/**
 * v1.2.1: dragging is driven by Rust (src-tauri/src/drag.rs), which polls the
 * physical cursor on a ~15ms thread and repositions the window in raw physical
 * coordinates. The frontend only signals "pressed" (drag_start) and
 * "released" (drag_end); all setPosition / rAF / screenX / outerPosition /
 * scaleFactor logic was removed (it caused oscillation and (0,0) drops).
 */
export function useGridDrag(label: string) {
  function onPointerDown(e: PointerEvent) {
    const t = e.target as HTMLElement;
    if (t.closest("button, input, a, textarea, [data-no-drag]")) return;
    dragging = true;
    try {
      (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    } catch {
      /* non-pointer input; ignore */
    }
    void invoke("drag_start", { label }).catch((err) => {
      console.error("[grid-drag] drag_start failed", err);
      dragging = false;
    });
  }

  function onPointerMove() {
    // movement is handled by the Rust poller
  }

  function onPointerUp() {
    if (!dragging) return;
    dragging = false;
    void invoke("drag_end", { label }).catch((err) => {
      console.error("[grid-drag] drag_end failed", err);
    });
  }

  return { onPointerDown, onPointerMove, onPointerUp };
}
