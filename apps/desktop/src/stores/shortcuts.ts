import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { openPath } from "@tauri-apps/plugin-opener";
import type { DesktopShortcut } from "../lib/shortcuts";

function rgbaToDataUrl(width: number, height: number, data: number[]): string {
  try {
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext("2d");
    if (!ctx) return "";
    const img = ctx.createImageData(width, height);
    img.data.set(data);
    ctx.putImageData(img, 0, 0);
    return canvas.toDataURL("image/png");
  } catch {
    return "";
  }
}

/** v1.3 file-shortcut zone store: load / add / remove / reorder / open. */
export const useShortcutStore = defineStore("shortcuts", {
  state: () => ({
    items: [] as DesktopShortcut[],
    icons: {} as Record<string, string>,
  }),
  actions: {
    async load() {
      const b = await invoke<{ shortcuts?: DesktopShortcut[] }>("get_bootstrap");
      this.items = (b.shortcuts ?? []).map((s, i) => ({ ...s, order: i }));
      for (const s of this.items) {
        if (s.type === "application") void this.loadIcon(s.target);
      }
    },
    async loadIcon(target: string) {
      if (this.icons[target]) return;
      try {
        const d = await invoke<{ width: number; height: number; data: number[] }>(
          "get_shortcut_icon",
          { path: target },
        );
        this.icons[target] = rgbaToDataUrl(d.width, d.height, d.data);
      } catch {
        /* no icon; keep the type glyph */
      }
    },
    async addPath(path: string) {
      const sc = await invoke<DesktopShortcut>("add_shortcut", { path });
      this.items.push(sc);
    },
    async remove(id: string) {
      await invoke("remove_shortcut", { id });
      this.items = this.items.filter((s) => s.id !== id).map((s, i) => ({ ...s, order: i }));
    },
    async reorder(ids: string[]) {
      await invoke("reorder_shortcuts", { ids });
      const byId = new Map(this.items.map((s) => [s.id, s]));
      this.items = ids
        .map((id) => byId.get(id))
        .filter((s): s is DesktopShortcut => !!s)
        .map((s, i) => ({ ...s, order: i }));
    },
    async open(sc: DesktopShortcut) {
      try {
        await openPath(sc.target);
      } catch (e) {
        console.error("[shortcuts] open failed", sc.target, e);
      }
    },
  },
});
