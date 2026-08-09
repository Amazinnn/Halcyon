export const FOCUS_MODES = ["light", "standard", "scholar"] as const;

export type FocusMode = (typeof FOCUS_MODES)[number];
export type DesktopLockMode = "none" | "keys" | "strict";

export const DEFAULT_FOCUS_MODE: FocusMode = "standard";

export function normalizeFocusMode(mode: string | undefined | null): FocusMode {
  return FOCUS_MODES.includes(mode as FocusMode) ? (mode as FocusMode) : DEFAULT_FOCUS_MODE;
}

export function desktopLockForFocus(mode: FocusMode): DesktopLockMode {
  switch (mode) {
    case "light": return "none";
    case "standard": return "keys";
    case "scholar": return "strict";
  }
}
