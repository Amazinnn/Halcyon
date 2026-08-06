import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
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

/** v1.5 free-placement shortcut store: DB-backed (app_shortcuts + ui_layouts). */
export const useShortcutStore = defineStore("shortcuts", {
  state: () => ({
    items: [] as DesktopShortcut[],
    icons: {} as Record<string, string>,
    launching: {} as Record<string, boolean>,
  }),
  actions: {
    async load() {
      try {
        const b = await invoke<{ shortcuts?: DesktopShortcut[] }>("get_bootstrap");
        this.items = b.shortcuts ?? [];
        for (const s of this.items) {
          if (s.type === "application") void this.loadIcon(s.target);
        }
      } catch (e) {
        // Never silently blank the desktop: log so bootstrap regressions stay visible.
        console.error('[shortcuts] load failed', e);
        this.items = [];
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
      if (sc.type === "application") void this.loadIcon(sc.target);
    },
    async addUrl(name: string, url: string) {
      const sc = await invoke<DesktopShortcut>("add_url_shortcut", { name, url });
      this.items.push(sc);
    },
    async addInternal(name: string, target: string) {
      const sc = await invoke<DesktopShortcut>("add_internal_shortcut", { name, target });
      this.items.push(sc);
    },
    async remove(id: string) {
      await invoke("remove_shortcut", { id });
      this.items = this.items.filter((s) => s.id !== id);
    },
    async move(id: string, col: number, row: number) {
      await invoke("move_shortcut", { id, col, row });
      const sc = this.items.find((s) => s.id === id);
      if (sc) {
        sc.col = col;
        sc.row = row;
      }
    },
    async open(sc: DesktopShortcut) {
      // Single-flight: ignore repeated clicks while this shortcut is launching.
      if (this.launching[sc.id]) return;
      this.launching[sc.id] = true;
      try {
        await invoke("launch_shortcut", { id: sc.id });
      } catch (e) {
        console.error("[shortcuts] launch failed", sc.id, e);
      } finally {
        delete this.launching[sc.id];
      }
    },
  },
});
