import { describe, expect, it } from "vitest";
import { restoreWindowError } from "./window-restore";

describe("restoreWindowError", () => {
  it("explains that a full grid requires folding another window", () => {
    expect(restoreWindowError("No available grid position for this window")).toBe(
      "没有可用位置，请先折叠一个窗口",
    );
  });
});
