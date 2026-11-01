/** Typed frontend module for browser-side Mori behavior. */
import { api, apiData } from '#lib/api/client';
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
/** Browser-probed dimensions and an optional bounded poster; unsupported codecs remain uploadable. */
export interface MediaPreview {
  width: number;
  height: number;
  duration: number;
  poster: Blob | null;
}
/** Probe locally without converting the original file or requiring a particular container. */
export async function makePoster(file: File): Promise<MediaPreview> {
  const url = URL.createObjectURL(file);
  const video = document.createElement('video');
  video.muted = true;
  video.preload = 'auto';
  video.src = url;
  let width = 0,
    height = 0,
    duration = 0;
  try {
    return await new Promise<MediaPreview>((resolve) => {
      const timer = setTimeout(() => {
        done(null);
      }, 10000);
      const done = (poster: Blob | null) => {
        clearTimeout(timer);
        resolve({ width, height, duration, poster });
      };
      video.onerror = () => {
        done(null);
      };
      video.onloadedmetadata = () => {
        width = video.videoWidth;
        height = video.videoHeight;
        duration = Number.isFinite(video.duration)
          ? Math.max(0, video.duration)
          : 0;
        video.currentTime = Math.min(30, duration / 3);
      };
      video.onseeked = () => {
        const canvas = document.createElement('canvas');
        const scale = Math.min(
          1,
          1280 / video.videoWidth,
          1280 / video.videoHeight,
        );
        canvas.width = Math.round(video.videoWidth * scale);
        canvas.height = Math.round(video.videoHeight * scale);
        const context = canvas.getContext('2d');
        if (context === null) {
          done(null);
          return;
        }
        context.drawImage(video, 0, 0, canvas.width, canvas.height);
        canvas.toBlob(done, 'image/jpeg', 0.82);
      };
    });
  } finally {
    video.pause();
    video.removeAttribute('src');
    video.load();
    URL.revokeObjectURL(url);
  }
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
  const hash = await fingerprint(file);
  if (
    resume &&
    (resume.size !== file.size ||
      resume.modified !== file.lastModified ||
      resume.fingerprint !== hash)
  )
    throw new Error('Choose the same original file to resume this upload.');
  onprogress(0, 'Preparing cover');
  const preview = resume === null ? await makePoster(file) : null;
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
            width: preview?.width ?? 0,
            height: preview?.height ?? 0,
            duration: preview?.duration ?? 0,
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
  if (!resume) {
    const poster = preview?.poster;
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
  }
  let next = 1;
  let completed = 0;
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
          p.number === part && p.size === chunk.size && p.checksum === checksum,
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
          const response = await fetch(url, {
            method: 'PUT',
            headers: { 'x-amz-checksum-sha256': checksum },
            body: chunk,
            signal,
            credentials: 'omit',
          });
          if (!response.ok)
            throw new Error(`Part upload failed (${String(response.status)})`);
          break;
        } catch (e) {
          if (attempt >= 2 || signal.aborted) throw e;
          await new Promise((r) => setTimeout(r, 500 * 2 ** attempt));
        }
      }
      completed += chunk.size;
      onprogress(Math.round((completed / file.size) * 100), 'Uploading');
    }
  }
  try {
    await Promise.all([worker(), worker()]);
    signal.throwIfAborted();
    onprogress(100, 'Verifying video');
    await apiData(
      api.POST('/v1/admin/uploads/{id}/complete', {
        params: { path: { id: upload.id } },
        signal,
      }),
    );
    remember(null);
  } catch (e) {
    transfer.abort();
    throw e;
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
