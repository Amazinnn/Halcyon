import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

const { invoke, emit } = vi.hoisted(() => ({ invoke: vi.fn(), emit: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({ emit, listen: vi.fn() }));
vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({ label: "desktop" }),
}));

import { useUiStore } from "./ui";

describe("focus pause transition", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    invoke.mockReset();
    emit.mockReset();
    Object.assign(globalThis, {
      window: {
        clearInterval: vi.fn(),
        setInterval: vi.fn(() => 42),
        setTimeout: vi.fn(),
      },
    });
  });

  it("rejects pause and skip at the action boundary during scholar work focus", async () => {
    const ui = useUiStore();
    ui.focusState = "focus";
    ui.activeFocusMode = "scholar";
    ui.focusRemainingSec = 120;
    ui._ticker = 7;

    await ui.pause();
    await ui.skip();

    expect(ui.timerPaused).toBe(false);
    expect(ui.focusState).toBe("focus");
    expect(ui.focusRemainingSec).toBe(120);
    expect(window.clearInterval).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith("desktop_set_focus_lock", expect.anything());
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

  it("finishes the final focus tick in rest once a pending pause unlock completes", async () => {
    let releasePauseUnlock!: () => void;
    let releaseCompletionUnlock!: () => void;
    const pauseUnlockPending = new Promise<void>((resolve) => { releasePauseUnlock = resolve; });
    const completionUnlockPending = new Promise<void>((resolve) => { releaseCompletionUnlock = resolve; });
    let observeCompletedRest!: () => void;
    const completedRest = new Promise<void>((resolve) => { observeCompletedRest = resolve; });
    let unlockCount = 0;
    invoke.mockImplementation((command: string, args?: { mode?: string }) => {
      if (command === "desktop_set_focus_lock" && args?.mode === "none") {
        unlockCount += 1;
        return unlockCount === 1 ? pauseUnlockPending : completionUnlockPending;
      }
      return Promise.resolve();
    });
    emit.mockImplementation((event: string, payload?: { state?: string; completed?: boolean }) => {
      if (event === "focus:state_changed" && payload?.state === "rest" && payload.completed) {
        observeCompletedRest();
      }
    });
    const ui = useUiStore();
    ui.focusState = "focus";
    ui.focusRemainingSec = 1;
    ui._ticker = 7;

    const pause = ui.pause();
    await Promise.resolve();
    await Promise.resolve();
    ui.tick();

    expect(ui.focusRemainingSec).toBe(0);
    expect(unlockCount).toBe(1);

    releasePauseUnlock();
    await pause;
    await Promise.resolve();
    await Promise.resolve();
    expect(unlockCount).toBe(2);

    releaseCompletionUnlock();
    await completedRest;

    expect(ui.focusState).toBe("rest");
    expect(ui.timerPaused).toBe(false);
    expect(ui.desktopLockTransitionPending).toBe(false);
    expect(emit.mock.calls.filter(([event, payload]) =>
      event === "focus:state_changed"
      && payload?.state === "rest"
      && payload?.completed === true,
    )).toHaveLength(1);
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
