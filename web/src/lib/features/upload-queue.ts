/** Consume files sequentially; failed or paused entries and their successors remain resumable. */
export async function runUploadQueue(
  files: File[],
  transfer: (file: File, position: number, total: number) => Promise<void>,
  signal: AbortSignal,
): Promise<void> {
  const total = files.length;
  let completed = 0;
  while (files[0] !== undefined) {
    signal.throwIfAborted();
    await transfer(files[0], completed + 1, total);
    files.shift();
    completed++;
  }
}
