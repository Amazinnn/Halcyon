export type ShortcutType = "file" | "folder" | "application" | "url" | "internal";

/** v2 (v1.5): free placement (col/row on the 12x8 desktop grid) + optional
 *  window-fit slot remembered for launched windows. */
export interface DesktopShortcut {
  id: string;
  name: string;
  type: ShortcutType;
  target: string;
  col: number;
  row: number;
  windowFit: "grid" | "none";
  fitCol?: number | null;
  fitRow?: number | null;
  fitCols?: number | null;
  fitRows?: number | null;
}
