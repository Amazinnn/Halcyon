import { describe, expect, it } from "vitest";
import { createFocusLockQueue } from "./focus-lock-queue";

describe("createFocusLockQueue", () => {
  it("applies rapid lock changes strictly in request order", async () => {
    const calls: boolean[] = [];
    let releaseFirst!: () => void;
    const firstPending = new Promise<void>((resolve) => {
      releaseFirst = resolve;
    });
    let first = true;
    const queue = createFocusLockQueue(async (locked) => {
      calls.push(locked);
      if (first) {
        first = false;
        await firstPending;
      }
    });

    const start = queue.request(true);
    const skip = queue.request(false);
    const restart = queue.request(true);

    await Promise.resolve();
    expect(calls).toEqual([true]);
    releaseFirst();
    await Promise.all([start, skip, restart]);

    expect(calls).toEqual([true, false, true]);
  });

  it("continues with the next requested state after a failed lock operation", async () => {
    const calls: boolean[] = [];
    const queue = createFocusLockQueue(async (locked) => {
      calls.push(locked);
      if (locked) throw new Error("hook unavailable");
    });

    await expect(queue.request(true)).rejects.toThrow("hook unavailable");
    await expect(queue.request(false)).resolves.toBeUndefined();
    expect(calls).toEqual([true, false]);
  });
});
