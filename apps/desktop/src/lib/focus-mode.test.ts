import { describe, expect, it } from "vitest";
import {
  DEFAULT_FOCUS_MODE,
  desktopLockForFocus,
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
});
