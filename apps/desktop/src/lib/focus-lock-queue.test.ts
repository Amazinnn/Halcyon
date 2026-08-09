import { describe, expect, it } from "vitest";
import { createFocusLockQueue, createSerialActionQueue } from "./focus-lock-queue";
import type { DesktopLockMode } from "./focus-mode";

describe("createFocusLockQueue", () => {
  it("applies rapid lock changes strictly in request order", async () => {
    const calls: DesktopLockMode[] = [];
    let releaseFirst!: () => void;
    const firstPending = new Promise<void>((resolve) => {
      releaseFirst = resolve;
    });
    let first = true;
    const queue = createFocusLockQueue(async (mode) => {
      calls.push(mode);
      if (first) {
        first = false;
        await firstPending;
      }
    });

    const start = queue.request("strict");
    const skip = queue.request("none");
    const restart = queue.request("keys");

    await Promise.resolve();
    expect(calls).toEqual(["strict"]);
    releaseFirst();
    await Promise.all([start, skip, restart]);

    expect(calls).toEqual(["strict", "none", "keys"]);
  });

  it("continues with the next requested state after a failed lock operation", async () => {
    const calls: DesktopLockMode[] = [];
    const queue = createFocusLockQueue(async (mode) => {
      calls.push(mode);
      if (mode === "strict") throw new Error("hook unavailable");
    });

    await expect(queue.request("strict")).rejects.toThrow("hook unavailable");
    await expect(queue.request("none")).resolves.toBeUndefined();
    expect(calls).toEqual(["strict", "none"]);
  });

  it("does not resolve a queued unlock until the unlock operation finishes", async () => {
    const calls: DesktopLockMode[] = [];
    let releaseUnlock!: () => void;
    const unlockPending = new Promise<void>((resolve) => {
      releaseUnlock = resolve;
    });
    const queue = createFocusLockQueue(async (mode) => {
      calls.push(mode);
      if (mode === "none") await unlockPending;
    });

    await queue.request("keys");
    let unlockDone = false;
    const unlock = queue.request("none").then(() => { unlockDone = true; });

    await Promise.resolve();
    expect(calls).toEqual(["keys", "none"]);
    expect(unlockDone).toBe(false);

    releaseUnlock();
    await unlock;
    expect(unlockDone).toBe(true);
  });

  it("serializes rapid resume/resume so a final pause is unlocked", async () => {
    const lockCalls: DesktopLockMode[] = [];
    let paused = true;
    let desktopLocked = false;
    const actions = createSerialActionQueue();

    const togglePause = () => actions.request(async () => {
      if (paused) {
        lockCalls.push("keys");
        desktopLocked = true;
        paused = false;
      } else {
        lockCalls.push("none");
        desktopLocked = false;
        paused = true;
      }
    });

    await Promise.all([togglePause(), togglePause()]);

    expect(lockCalls).toEqual(["keys", "none"]);
    expect(paused).toBe(true);
    expect(desktopLocked).toBe(false);
  });
});
