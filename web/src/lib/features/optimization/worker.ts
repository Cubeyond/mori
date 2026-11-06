// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** A disposable dedicated worker has no network credentials and processes only one local file. */
import { optimizeImage } from './image';
import { copyDirectory, copyName } from './storage';
import {
  parsePreparedCopy,
  worthwhile,
  type OptimizationRequest,
  type OptimizationResponse,
} from './model';

/** Construct a trusted request from the structured-clone boundary without type assertions. */
function parseRequest(value: unknown): OptimizationRequest | null {
  if (
    value === null ||
    typeof value !== 'object' ||
    !('file' in value) ||
    !(value.file instanceof File) ||
    !('kind' in value) ||
    (value.kind !== 'image' && value.kind !== 'video') ||
    !('mode' in value) ||
    (value.mode !== 'balanced' && value.mode !== 'quality') ||
    !('copy' in value) ||
    !('maxPixels' in value) ||
    typeof value.maxPixels !== 'number' ||
    !('budgetMs' in value) ||
    typeof value.budgetMs !== 'number' ||
    value.budgetMs <= 0 ||
    value.budgetMs > 20_000
  )
    return null;
  const copy = parsePreparedCopy(value.copy);
  if (
    copy === null ||
    !Number.isFinite(value.maxPixels) ||
    value.maxPixels <= 0 ||
    value.maxPixels > 24_000_000
  )
    return null;
  return {
    file: value.file,
    kind: value.kind,
    mode: value.mode,
    copy,
    maxPixels: value.maxPixels,
    budgetMs: value.budgetMs,
  };
}
/** Only small structured metadata crosses back to the application, never whole-video buffers. */
function report(value: OptimizationResponse): void {
  self.postMessage(value);
}
/** Finalize disk bytes before making their descriptor eligible for a resumable upload. */
async function run(request: OptimizationRequest): Promise<void> {
  let result = null;
  try {
    const estimate = await navigator.storage.estimate();
    if (
      (estimate.quota ?? 0) - (estimate.usage ?? 0) <
      request.file.size + (4 << 20)
    )
      return;
    if (request.kind === 'image') {
      const blob = await optimizeImage(
        request.file,
        request.mode,
        request.maxPixels,
      );
      if (blob === null || !worthwhile(request.file.size, blob.size)) return;
      const handle = await (
        await copyDirectory()
      ).getFileHandle(copyName(request.copy), { create: true });
      const writable = await handle.createWritable();
      try {
        await writable.write(blob);
        await writable.close();
      } catch (failure) {
        await writable.abort();
        throw failure;
      }
    } else {
      const { optimizeVideo } = await import('./video');
      if (!(await optimizeVideo(request))) return;
    }
    const file = await (
      await (await copyDirectory()).getFileHandle(copyName(request.copy))
    ).getFile();
    if (worthwhile(request.file.size, file.size))
      result = { ...request.copy, size: file.size };
  } catch {
    /* Optional optimization always falls back to the selected original. */
  } finally {
    report({ type: 'done', copy: result });
  }
}
self.addEventListener('message', (event: MessageEvent<unknown>) => {
  const request = parseRequest(event.data);
  if (request !== null) void run(request);
  else report({ type: 'done', copy: null });
});
