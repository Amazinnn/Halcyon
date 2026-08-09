/**
 * Serializes desktop-lock requests from the focus state machine. Every user
 * action is retained in order so rapid clicks cannot leave the Windows shell
 * in the state requested by an older action.
 */
export function createFocusLockQueue(apply: (locked: boolean) => Promise<void>) {
  let tail = Promise.resolve();

  return {
    request(locked: boolean): Promise<void> {
      const next = tail.then(() => apply(locked));
      // Keep the queue alive after a failed operation while preserving the
      // original rejection for its caller.
      tail = next.catch(() => undefined);
      return next;
    },
  };
}
