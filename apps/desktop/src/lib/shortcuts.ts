export type ShortcutType = "file" | "folder" | "application";

export interface DesktopShortcut {
  id: string;
  name: string;
  type: ShortcutType;
  target: string;
  order: number;
}
