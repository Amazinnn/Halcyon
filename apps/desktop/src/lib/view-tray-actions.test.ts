import { describe, expect, it } from "vitest";
import { ViewTrayActions } from "./view-tray-actions";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((next) => { resolve = next; });
  return { promise, resolve };
}

describe("view tray actions", () => {
  it("does not start another restore or toggle the tray while a restore is pending", async () => {
    const pending = deferred<void>();
    const invoked: string[] = [];
    const tray = new ViewTrayActions((label) => {
      invoked.push(label);
      return pending.promise;
    });

    tray.toggle();
    expect(tray.open).toBe(true);

    const first = tray.restore("chat");
    expect(tray.busy).toBe(true);
    expect(tray.open).toBe(false);
    expect(tray.toggle()).toBe(false);
    await tray.restore("stats");
    expect(invoked).toEqual(["chat"]);

    pending.resolve();
    await first;
    expect(tray.busy).toBe(false);
  });
});
