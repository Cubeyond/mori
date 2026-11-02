/** Bound simultaneous files while preserving live edits and settling every worker before returning. */
export async function runUploadQueue(
  current: () => File[],
  transfer: (file: File, signal: AbortSignal) => Promise<void>,
  signal: AbortSignal,
): Promise<number> {
  const stop = new AbortController();
  const batchSignal = AbortSignal.any([signal, stop.signal]);
  const claimed = new Set<File>();
  let completed = 0;
  /** Claim originals by identity so mutations never duplicate or skip an active transfer. */
  async function worker(): Promise<void> {
    try {
      for (;;) {
        batchSignal.throwIfAborted();
        const file = current().find((entry) => !claimed.has(entry));
        if (file === undefined) return;
        claimed.add(file);
        await transfer(file, batchSignal);
        const files = current();
        const index = files.indexOf(file);
        if (index >= 0) files.splice(index, 1);
        completed++;
      }
    } catch (failure) {
      stop.abort(failure);
      throw failure;
    }
  }
  const workers = await Promise.allSettled([worker(), worker()]);
  const failure = workers.find((result) => result.status === 'rejected');
  if (failure?.status === 'rejected') throw failure.reason;
  return completed;
}
