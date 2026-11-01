/** Typed frontend module for browser-side Mori behavior. */
import { api, apiData } from '#lib/api/client';
import { prepareUploadPreview } from './upload-preview';
import { putUploadPart } from './upload-part';
import { waitForJob } from './jobs';
import {
  rememberUpload as remember,
  savedUpload,
  type PendingUpload,
} from './upload-session';
export { savedUpload, type PendingUpload } from './upload-session';

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
  resume: PendingUpload | null = null,
): Promise<void> {
  const transfer = new AbortController();
  signal = AbortSignal.any([signal, transfer.signal]);
  onprogress(0, 'Preparing video');
  const preview = resume === null ? prepareUploadPreview(file, signal) : null;
  try {
    const [hash, metadata] = await Promise.all([
      fingerprint(file),
      preview?.metadata,
    ]);
    signal.throwIfAborted();
    if (
      resume &&
      (resume.size !== file.size ||
        resume.modified !== file.lastModified ||
        resume.fingerprint !== hash)
    )
      throw new Error('Choose the same original file to resume this upload.');
    const extension = file.name.split('.').at(-1)?.toLowerCase() ?? '';
    const upload = resume
      ? await apiData(
          api.GET('/v1/admin/uploads/{id}', {
            params: { path: { id: resume.id } },
          }),
        )
      : await apiData(
          api.POST('/v1/admin/uploads', {
            body: {
              album_id,
              size: file.size,
              visibility,
              extension,
              width: metadata?.width ?? 0,
              height: metadata?.height ?? 0,
              duration: metadata?.duration ?? 0,
            },
            signal,
          }),
        );
    const pending: PendingUpload = {
      id: upload.id,
      name: upload.name,
      expires_at: upload.expires_at,
      size: file.size,
      modified: file.lastModified,
      fingerprint: hash,
    };
    remember(pending);
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
      onprogress(Math.round((bytes / file.size) * 100), 'Uploading');
    }
    /** Upload consecutive parts with checksum-based resume, bounded retries, and cancellation. */
    async function worker() {
      while (next <= upload.part_count) {
        signal.throwIfAborted();
        const part = next++;
        const chunk = file.slice(
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
          onprogress(Math.round((completed / file.size) * 100), 'Resuming');
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
    await Promise.all([worker(), worker(), posterTask]);
    signal.throwIfAborted();
    onprogress(100, 'Verifying video');
    const accepted = await apiData(
      api.POST('/v1/admin/uploads/{id}/complete', {
        params: { path: { id: upload.id } },
        signal,
      }),
    );
    remember(null);
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
  }
}
/** Abort the active multipart operation before discarding local resume metadata. */
export async function cancelUpload(): Promise<void> {
  const saved = savedUpload();
  if (saved) {
    await apiData(
      api.DELETE('/v1/admin/uploads/{id}', {
        params: { path: { id: saved.id } },
      }),
    );
    remember(null);
  }
}
