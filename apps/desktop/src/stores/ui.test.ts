import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ emit: vi.fn(), listen: vi.fn() }));
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({ label: "desktop" }),
}));

import { useUiStore } from "./ui";

describe("focus pause transition", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    Object.assign(globalThis, {
      window: {
        clearInterval: vi.fn(),
        setInterval: vi.fn(() => 42),
        setTimeout: vi.fn(),
      },
    });
  });

  it("stops the ticker before waiting for the pause unlock", async () => {
    let releaseUnlock!: () => void;
    const unlockPending = new Promise<void>((resolve) => { releaseUnlock = resolve; });
    invoke.mockImplementation((command: string, args?: { mode?: string }) =>
      command === "desktop_set_focus_lock" && args?.mode === "none" ? unlockPending : Promise.resolve(),
    );
    const ui = useUiStore();
    ui.focusState = "focus";
    ui.focusRemainingSec = 1;
    ui._ticker = 7;

    const pause = ui.pause();
    await Promise.resolve();
    await Promise.resolve();

    expect(window.clearInterval).toHaveBeenCalledWith(7);
    expect(ui.timerPaused).toBe(false);

    releaseUnlock();
    await pause;
    expect(ui.timerPaused).toBe(true);
  });

  it("queues skip behind a pending pause instead of dropping the final action", async () => {
    let releaseUnlock!: () => void;
    const unlockPending = new Promise<void>((resolve) => { releaseUnlock = resolve; });
    invoke.mockImplementation((command: string, args?: { mode?: string }) =>
      command === "desktop_set_focus_lock" && args?.mode === "none" ? unlockPending : Promise.resolve(),
    );
    const ui = useUiStore();
    ui.focusState = "focus";
    ui.focusRemainingSec = 60;
    ui._ticker = 7;

    const pause = ui.pause();
    await Promise.resolve();
    await Promise.resolve();
    const skip = ui.skip();
    releaseUnlock();
    await Promise.all([pause, skip]);

    expect(ui.focusState).toBe("rest");
    expect(ui.timerPaused).toBe(false);
  });

  it("applies rapid skip actions in order", async () => {
    let releaseUnlock!: () => void;
    const unlockPending = new Promise<void>((resolve) => { releaseUnlock = resolve; });
    invoke.mockImplementation((command: string, args?: { mode?: string }) =>
      command === "desktop_set_focus_lock" && args?.mode === "none" ? unlockPending : Promise.resolve(),
    );
    const ui = useUiStore();
    ui.focusState = "focus";
    ui.focusRemainingSec = 60;
    ui._ticker = 7;

    const firstSkip = ui.skip();
    await Promise.resolve();
    await Promise.resolve();
    const secondSkip = ui.skip();
    releaseUnlock();
    await Promise.all([firstSkip, secondSkip]);

    expect(ui.focusState).toBe("focus");
    expect(ui.timerPaused).toBe(false);
  });

  it("stops an existing ticker before waiting to lock a new focus round", async () => {
    let releaseLock!: () => void;
    const lockPending = new Promise<void>((resolve) => { releaseLock = resolve; });
    invoke.mockImplementation((command: string, args?: { mode?: string }) =>
      command === "desktop_set_focus_lock" && args?.mode === "keys" ? lockPending : Promise.resolve(),
    );
    const ui = useUiStore();
    ui.focusState = "focus";
    ui.focusRemainingSec = 1;
    ui._ticker = 7;

    const start = ui.startFocus();
    await Promise.resolve();
    await Promise.resolve();

    expect(window.clearInterval).toHaveBeenCalledWith(7);
    releaseLock();
    await start;
    expect(ui.focusState).toBe("focus");
    expect(ui.focusRemainingSec).toBe(25 * 60);
  });

  it("keeps a queued workflow focus marked as workflow-driven", async () => {
    let releaseUnlock!: () => void;
    const unlockPending = new Promise<void>((resolve) => { releaseUnlock = resolve; });
    invoke.mockImplementation((command: string, args?: { mode?: string }) =>
      command === "desktop_set_focus_lock" && args?.mode === "none" ? unlockPending : Promise.resolve(),
    );
    const ui = useUiStore();
    ui.focusState = "focus";
    ui.focusRemainingSec = 60;
    ui._ticker = 7;

    const skip = ui.skip();
    await Promise.resolve();
    await Promise.resolve();
    const workflowFocus = (ui.startFocusFor as (seconds: number, workflowDriven?: boolean) => Promise<void>)(30, true);
    releaseUnlock();
    await Promise.all([skip, workflowFocus]);

    expect(ui.focusState).toBe("focus");
    expect(ui.focusRemainingSec).toBe(30);
    expect(ui.workflowDriven).toBe(true);
  });
});
