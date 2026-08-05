import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { openPath } from "@tauri-apps/plugin-opener";
import type { DesktopShortcut } from "../lib/shortcuts";

/** v1.3 file-shortcut zone store: load / add / remove / reorder / open. */
export const useShortcutStore = defineStore("shortcuts", {
  state: () => ({
    items: [] as DesktopShortcut[],
  }),
  actions: {
    async load() {
      const b = await invoke<{ shortcuts?: DesktopShortcut[] }>("get_bootstrap");
      this.items = (b.shortcuts ?? []).map((s, i) => ({ ...s, order: i }));
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
