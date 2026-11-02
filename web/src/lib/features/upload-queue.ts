/** Consume the live queue sequentially, preserving failed files and accepting later edits. */
export async function runUploadQueue(
  current: () => File[],
  transfer: (file: File, position: number, total: number) => Promise<void>,
  signal: AbortSignal,
): Promise<number> {
  let completed = 0;
  while (current()[0] !== undefined) {
    signal.throwIfAborted();
    const file = current()[0];
    if (file === undefined) break;
    await transfer(file, completed + 1, completed + current().length);
    const files = current();
    const index = files.indexOf(file);
    if (index >= 0) files.splice(index, 1);
    completed++;
  }
  return completed;
}
