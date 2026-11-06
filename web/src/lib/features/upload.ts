// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Typed frontend module for browser-side Mori behavior. */
import { api, apiData } from '#lib/api/client';
import { discardPrepared, type OptimizationBatch } from './optimization/client';
import { readPrepared, removePrepared } from './optimization/storage';
import type { PreparedUpload } from './optimization/client';
import { prepareUploadPreview } from './upload-preview';
import { putUploadPart } from './upload-part';
import { waitForJob } from './jobs';
import {
  rememberUpload as remember,
  forgetUpload,
  savedUploads,
  type PendingUpload,
} from './upload-session';
export { savedUploads, type PendingUpload } from './upload-session';
/** A live worker must never reuse a multipart identity owned by another worker. */
const activeUploads = new Set<string>();

/** Open the shared uploader, optionally preselecting an album for a new draft. */
export function openUploader(albumId: string | null = null): void {
  window.dispatchEvent(new CustomEvent('mori:upload', { detail: albumId }));
}

/** Transfer summary retained when the upload dialog is dismissed. */
export interface TransferStatus {
  /** Completed percentage of the current file transfer. */
  progress: number;
  /** Concise user-facing transfer stage. */
  phase: string;
  /** A resumable session remains available after an interruption. */
  paused?: boolean;
}
/** Hash a bounded local prefix to detect a mismatched resume file. */
async function fingerprint(file: File) {
  const data = await file.slice(0, 65536).arrayBuffer();
  return Array.from(
    new Uint8Array(await crypto.subtle.digest('SHA-256', data)),
    (b) => b.toString(16).padStart(2, '0'),
  ).join('');
}
/** Transfer checksummed parts directly to S3 and request server-side verification. */
export async function uploadVideo(
  file: File,
  album_id: string | null,
  visibility: string,
  onprogress: (value: number, label: string) => void,
  signal: AbortSignal,
  kind: 'image' | 'video' = 'video',
  preparation?: {
    batch: OptimizationBatch;
    onprepared: (bytes: number) => void;
  },
): Promise<void> {
  const transfer = new AbortController();
  signal = AbortSignal.any([signal, transfer.signal]);
  onprogress(0, 'Preparing media');
  let preview: ReturnType<typeof prepareUploadPreview> | undefined;
  let uploadId: string | undefined;
  try {
    const hash = await fingerprint(file);
    signal.throwIfAborted();
    let resume = savedUploads().find(
      (upload) => !activeUploads.has(upload.id) && matches(upload, file, hash),
    );
    let prepared: PreparedUpload = { file };
    if (resume?.prepared !== undefined) {
      const payload = await readPrepared(resume.prepared);
      if (payload === null) {
        // Missing local bytes cannot be regenerated inside an existing multipart identity.
        await cancelUpload(file);
        resume = undefined;
      } else prepared = { file: payload, copy: resume.prepared };
    }
    if (resume === undefined && preparation !== undefined)
      prepared = await preparation.batch.prepare(file, kind, signal);
    signal.throwIfAborted();
    const payload = prepared.file;
    preparation?.onprepared(payload.size);
    if (resume !== undefined) activeUploads.add(resume.id);
    uploadId = resume?.id;
    preview =
      resume === undefined
        ? prepareUploadPreview(payload, signal, kind)
        : undefined;
    const metadata = await preview?.metadata;
    const extension = payload.name.split('.').at(-1)?.toLowerCase() ?? '';
    const upload = resume
      ? await apiData(
          api.GET('/v1/admin/uploads/{id}', {
            params: { path: { id: resume.id } },
            signal,
          }),
        )
      : await apiData(
          api.POST('/v1/admin/uploads', {
            body: {
              album_id,
              size: payload.size,
              visibility,
              extension,
              width: metadata?.width ?? 0,
              height: metadata?.height ?? 0,
              duration: metadata?.duration ?? 0,
            },
            // Settle reservation creation before a batch cancellation discovers its server identity.
            signal: AbortSignal.timeout(30_000),
          }),
        );
    const pending: PendingUpload = {
      id: upload.id,
      name: upload.name,
      expires_at: upload.expires_at,
      size: file.size,
      modified: file.lastModified,
      fingerprint: hash,
      source_name: file.name,
      ...(prepared.copy === undefined ? {} : { prepared: prepared.copy }),
    };
    uploadId = upload.id;
    activeUploads.add(upload.id);
    remember(pending);
    signal.throwIfAborted();
    const posterTask = (async () => {
      const poster = await preview?.poster;
      if (poster && poster.size <= 1_048_576) {
        await apiData(
          api.PUT('/v1/admin/videos/{id}/poster', {
            params: { path: { id: upload.video_id } },
            headers: { 'content-type': 'image/jpeg' },
            body: poster,
            bodySerializer: (body) => body,
            signal,
          }),
        );
      }
    })();
    let next = 1;
    let completed = 0;
    const inflight = new Map<number, number>();
    /** Aggregate bytes across both workers so large parts never leave progress frozen. */
    function report(): void {
      const bytes =
        completed +
        Array.from(inflight.values()).reduce((sum, value) => sum + value, 0);
      onprogress(Math.round((bytes / payload.size) * 100), 'Uploading');
    }
    /** Upload consecutive parts with checksum-based resume, bounded retries, and cancellation. */
    async function worker() {
      while (next <= upload.part_count) {
        signal.throwIfAborted();
        const part = next++;
        const chunk = payload.slice(
          (part - 1) * upload.part_size,
          part * upload.part_size,
        );
        const checksum = btoa(
          String.fromCharCode(
            ...new Uint8Array(
              await crypto.subtle.digest('SHA-256', await chunk.arrayBuffer()),
            ),
          ),
        );
        const existing = upload.parts.find(
          (p) =>
            p.number === part &&
            p.size === chunk.size &&
            p.checksum === checksum,
        );
        if (existing) {
          completed += chunk.size;
          onprogress(Math.round((completed / payload.size) * 100), 'Resuming');
          continue;
        }
        for (let attempt = 0; ; attempt++) {
          try {
            const { url } = await apiData(
              api.POST('/v1/admin/uploads/{id}/parts/{part}', {
                params: { path: { id: upload.id, part } },
                body: { checksum },
                signal,
              }),
            );
            await putUploadPart(url, chunk, checksum, signal, (bytes) => {
              inflight.set(part, bytes);
              report();
            });
            break;
          } catch (e) {
            inflight.delete(part);
            if (attempt >= 2 || signal.aborted) throw e;
            await new Promise((r) => setTimeout(r, 500 * 2 ** attempt));
          }
        }
        inflight.delete(part);
        completed += chunk.size;
        report();
      }
    }
    const tasks = [worker(), worker(), posterTask];
    try {
      await Promise.all(tasks);
    } catch (failure) {
      transfer.abort();
      await Promise.allSettled(tasks);
      throw failure;
    }
    signal.throwIfAborted();
    onprogress(100, 'Verifying upload');
    const accepted = await apiData(
      api.POST('/v1/admin/uploads/{id}/complete', {
        params: { path: { id: upload.id } },
        signal,
      }),
    );
    forgetUpload(upload.id);
    if (prepared.copy !== undefined) await removePrepared(prepared.copy);
    await discardPrepared(file);
    if (accepted.job_id !== null) {
      try {
        await waitForJob(accepted.job_id, signal);
      } catch (error) {
        // After acceptance, pausing stops monitoring and the next queue entry, not this durable job.
        if (!signal.aborted) throw error;
      }
    }
  } catch (e) {
    transfer.abort();
    throw e;
  } finally {
    preview?.dispose();
    if (uploadId !== undefined) activeUploads.delete(uploadId);
  }
}
/** Cancel selected saved originals or the server-discovered queue after browser storage loss. */
export async function cancelUpload(file?: File): Promise<void> {
  const hash = file === undefined ? undefined : await fingerprint(file);
  const local = savedUploads().filter(
    (upload) => file === undefined || matches(upload, file, hash ?? ''),
  );
  if (file !== undefined && local.length === 0) {
    await discardPrepared(file);
    return;
  }
  const server = await apiData(api.GET('/v1/admin/uploads'));
  const active = new Set(server.map((upload) => upload.id));
  const ids =
    file === undefined
      ? [...active]
      : local
          .filter((upload) => active.has(upload.id))
          .map((upload) => upload.id);
  if (ids.length > 0)
    await apiData(api.DELETE('/v1/admin/uploads', { body: { ids } }));
  for (const upload of local) {
    forgetUpload(upload.id);
    if (upload.prepared !== undefined) await removePrepared(upload.prepared);
  }
  await discardPrepared(file);
}
/** A resumed file must match its original bytes and metadata independently of queue position. */
function matches(upload: PendingUpload, file: File, hash: string): boolean {
  return (
    (upload.source_name === undefined || upload.source_name === file.name) &&
    upload.size === file.size &&
    upload.modified === file.lastModified &&
    upload.fingerprint === hash
  );
}
