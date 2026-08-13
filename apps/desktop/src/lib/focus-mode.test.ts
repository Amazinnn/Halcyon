import { describe, expect, it } from "vitest";
import {
  DEFAULT_FOCUS_MODE,
  desktopLockForFocus,
  focusControlPolicy,
  type FocusMode,
} from "./focus-mode";

describe("focus lock modes", () => {
  it("defaults new users to the standard mode", () => {
    expect(DEFAULT_FOCUS_MODE).toBe("standard");
  });

  it.each<[FocusMode, "none" | "keys" | "strict"]>([
    ["light", "none"],
    ["standard", "keys"],
    ["scholar", "strict"],
  ])("maps %s to the intended desktop lock", (mode, expected) => {
    expect(desktopLockForFocus(mode)).toBe(expected);
  });

  it("hides only the work-phase controls restricted by each locked mode", () => {
    expect(focusControlPolicy("light", "focus")).toEqual({
      quitVisible: true, pauseVisible: true, skipVisible: true,
    });
    expect(focusControlPolicy("standard", "focus")).toEqual({
      quitVisible: false, pauseVisible: true, skipVisible: true,
    });
    expect(focusControlPolicy("scholar", "focus")).toEqual({
      quitVisible: false, pauseVisible: false, skipVisible: false,
    });
    expect(focusControlPolicy("scholar", "rest")).toEqual({
      quitVisible: true, pauseVisible: true, skipVisible: true,
    });
  });
});
