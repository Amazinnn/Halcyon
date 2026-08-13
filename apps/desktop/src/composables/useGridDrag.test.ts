import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { useGridDrag } from "./useGridDrag";

function dragTarget() {
  return {
    closest: vi.fn(() => null),
    setPointerCapture: vi.fn(),
  };
}

describe("useGridDrag release lifecycle", () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockResolvedValue(undefined);
  });

  it("uses one release path for cancel, lost capture, and pointer up", async () => {
    const drag = useGridDrag("pet");
    const target = dragTarget();
    const down = { target, currentTarget: target, pointerId: 1 } as unknown as PointerEvent;

    drag.onPointerDown(down);
    await Promise.resolve();
    drag.finishPointerDrag();
    drag.finishPointerDrag();
    drag.onPointerUp();

    expect(invoke).toHaveBeenCalledTimes(4);
    expect(invoke).toHaveBeenNthCalledWith(1, "drag_start", { label: "pet" });
    expect(invoke).toHaveBeenNthCalledWith(2, "drag_diagnostic_browser_event", {
      label: "pet", stage: "browser:pointerdown", sequence: null,
    });
    expect(invoke).toHaveBeenNthCalledWith(3, "drag_diagnostic_browser_event", {
      label: "pet", stage: "browser:pointerup", sequence: null,
    });
    expect(invoke).toHaveBeenNthCalledWith(4, "drag_end", { label: "pet" });
  });

  it("records a cancellation boundary before ending the pet drag", async () => {
    const drag = useGridDrag("pet");
    const target = dragTarget();
    const down = { target, currentTarget: target, pointerId: 9 } as unknown as PointerEvent;

    drag.onPointerDown(down);
    await Promise.resolve();
    drag.onPointerCancel();

    expect(invoke).toHaveBeenNthCalledWith(3, "drag_diagnostic_browser_event", {
      label: "pet",
      sequence: null,
      stage: "browser:pointercancel",
    });
    expect(invoke).toHaveBeenNthCalledWith(4, "drag_end", { label: "pet" });
  });
});
