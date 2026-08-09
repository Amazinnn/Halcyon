import type { DesktopLockMode } from "./focus-mode";

/**
 * Serializes desktop-lock requests from the focus state machine. Every user
 * action is retained in order so rapid clicks cannot leave the Windows shell
 * in the state requested by an older action.
 */
export function createFocusLockQueue(apply: (mode: DesktopLockMode) => Promise<void>) {
  let tail = Promise.resolve();

  return {
    request(mode: DesktopLockMode): Promise<void> {
      const next = tail.then(() => apply(mode));
      // Keep the queue alive after a failed operation while preserving the
      // original rejection for its caller.
      tail = next.catch(() => undefined);
      return next;
    },
  };
}

/** Serializes stateful focus actions as well as their native lock requests. */
export function createSerialActionQueue() {
  let tail = Promise.resolve();

  return {
    request<T>(action: () => Promise<T>): Promise<T> {
      const next = tail.then(action);
      tail = next.then(() => undefined, () => undefined);
      return next;
    },
  };
}
