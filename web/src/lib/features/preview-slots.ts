// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

/** Limit concurrent local media decoders even when many thumbnail components become visible. */
interface WaitingPreview {
  /** Aborted queued work is removed before it can create a decoder. */
  signal: AbortSignal;
  /** Resolve the caller with its exclusive release callback. */
  resolve: (release: () => void) => void;
  /** Reject cancelled work without leaving a pending promise behind. */
  reject: (reason: unknown) => void;
  /** Remove this task when its owning thumbnail disappears. */
  abort: () => void;
}
/** Queued waiters are small references; only two callers may hold browser decoders. */
const waiting: WaitingPreview[] = [];
/** Decoders are independent of multipart transfer concurrency. */
let active = 0;

/** Give a visible thumbnail a slot, or cancel it before any decoding begins. */
export function previewSlot(signal: AbortSignal): Promise<() => void> {
  signal.throwIfAborted();
  return new Promise((resolve, reject) => {
    const task: WaitingPreview = { signal, resolve, reject, abort };
    /** Remove an aborted waiter and settle its promise exactly once. */
    function abort(): void {
      const index = waiting.indexOf(task);
      if (index < 0) return;
      waiting.splice(index, 1);
      signal.removeEventListener('abort', abort);
      reject(
        signal.reason instanceof Error
          ? signal.reason
          : new DOMException('Preview cancelled', 'AbortError'),
      );
    }
    waiting.push(task);
    signal.addEventListener('abort', abort, { once: true });
    drain();
  });
}

/** Release is idempotent so cancellation and normal completion cannot overfill the budget. */
function drain(): void {
  while (active < 2 && waiting.length > 0) {
    const task = waiting.shift();
    if (task === undefined) return;
    task.signal.removeEventListener('abort', task.abort);
    active++;
    let released = false;
    task.resolve(() => {
      if (released) return;
      released = true;
      active--;
      drain();
    });
  }
}
