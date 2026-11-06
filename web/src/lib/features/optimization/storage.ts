// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/// <reference lib="dom.asynciterable" />
/** Exact prepared bytes live in origin-private disk storage, never whole-video RAM buffers. */
import type { PreparedCopy } from '#lib/features/optimization/model';
/** One isolated namespace avoids touching other application or browser files. */
const directoryName = 'mori-upload-copies-v1';
/** Browser support is optional; unsupported devices upload their originals. */
export function hasPreparedStorage(): boolean {
  return (
    typeof navigator !== 'undefined' &&
    typeof navigator.storage !== 'undefined' &&
    typeof navigator.storage.getDirectory === 'function'
  );
}
/** Resolve only Mori's private directory. */
export async function copyDirectory(
  create = true,
): Promise<FileSystemDirectoryHandle> {
  return (await navigator.storage.getDirectory()).getDirectoryHandle(
    directoryName,
    { create },
  );
}
/** Descriptors were validated at the persistence boundary and contain no user paths. */
export function copyName(copy: PreparedCopy): string {
  return copy.id + '.' + copy.extension;
}
/** Read the same finalized bytes after a reload instead of encoding another multipart source. */
export async function readPrepared(copy: PreparedCopy): Promise<File | null> {
  try {
    const file = await (
      await (await copyDirectory(false)).getFileHandle(copyName(copy))
    ).getFile();
    if (file.size !== copy.size) return null;
    return new File([file], copyName(copy), {
      type: copy.extension === 'webp' ? 'image/webp' : 'video/mp4',
      lastModified: copy.created_at,
    });
  } catch {
    return null;
  }
}
/** Cleanup failures cannot turn an accepted S3 upload into another upload attempt. */
export async function removePrepared(copy: PreparedCopy): Promise<void> {
  if (!hasPreparedStorage()) return;
  try {
    await (await copyDirectory(false)).removeEntry(copyName(copy));
  } catch {
    /* Expired files are retried by namespace cleanup. */
  }
}
/** Retain resumable copies while removing day-old output left by a closed tab or failed worker. */
export async function cleanPrepared(
  retained: readonly PreparedCopy[],
): Promise<void> {
  if (!hasPreparedStorage()) return;
  try {
    const directory = await copyDirectory(false),
      keep = new Set(retained.map(copyName));
    for await (const [name, handle] of directory.entries()) {
      if (
        handle.kind !== 'file' ||
        !/^[0-9a-f]{32}\.(webp|mp4)$/.test(name) ||
        keep.has(name)
      )
        continue;
      const file = await (await directory.getFileHandle(name)).getFile();
      if (Date.now() - file.lastModified > 86_400_000)
        await directory.removeEntry(name);
    }
  } catch {
    /* Storage may be unavailable, blocked, or concurrently cleaned by another tab. */
  }
}
