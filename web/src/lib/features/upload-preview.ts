/** Browser metadata is independent of optional, bounded cover generation. */
import { prepareImagePreview } from '#lib/features/image-preview';
/** Shared image/video dimensions; unknown values never prevent preserving an original. */
export interface MediaMetadata {
  /** Native pixel width; zero means the browser cannot decode the container. */
  width: number;
  /** Native pixel height, without cropping the uploaded video. */
  height: number;
  /** Native duration in seconds; unknown values remain zero. */
  duration: number;
}
/** Independent stages let multipart transfer begin before a cover finishes encoding. */
export interface UploadPreview {
  /** Resolves as soon as browser metadata is available. */
  metadata: Promise<MediaMetadata>;
  /** Optional bounded JPEG; unsupported codecs never prevent upload. */
  poster: Promise<Blob | null>;
  /** Release the local decoder when the transfer is interrupted. */
  dispose: () => void;
}
/** Probe a local file without delaying transfer for seeking or JPEG generation. */
export function prepareUploadPreview(
  file: File,
  signal: AbortSignal,
  kind: 'image' | 'video' = 'video',
  maximum = 1280,
): UploadPreview {
  if (kind === 'image') return prepareImagePreview(file, signal, maximum);
  const video = document.createElement('video');
  const url = URL.createObjectURL(file);
  const metadata = { width: 0, height: 0, duration: 0 };
  let finished = false;
  const metadataResult = Promise.withResolvers<MediaMetadata>();
  const posterResult = Promise.withResolvers<Blob | null>();
  const result = {
    metadata: metadataResult.promise,
    poster: posterResult.promise,
    dispose: () => {
      finish(null);
    },
  };
  const timer = setTimeout(() => {
    finish(null);
  }, 3000);
  /** Complete both stages once and release browser resources on every exit. */
  function finish(poster: Blob | null): void {
    if (finished) return;
    finished = true;
    clearTimeout(timer);
    signal.removeEventListener('abort', abort);
    metadataResult.resolve(metadata);
    posterResult.resolve(poster);
    video.onloadedmetadata =
      video.onseeked =
      video.onloadeddata =
      video.onerror =
        null;
    video.pause();
    video.removeAttribute('src');
    video.load();
    URL.revokeObjectURL(url);
  }
  /** An aborted upload never retains a decoder or waits for its timeout. */
  function abort(): void {
    finish(null);
  }
  /** Render one nearby frame, avoiding a long decode seek through the original recording. */
  function cover(): void {
    if (
      finished ||
      video.readyState < 2 ||
      metadata.width === 0 ||
      metadata.height === 0
    )
      return;
    const canvas = document.createElement('canvas');
    const scale = Math.min(
      1,
      maximum / metadata.width,
      maximum / metadata.height,
    );
    canvas.width = Math.round(metadata.width * scale);
    canvas.height = Math.round(metadata.height * scale);
    const context = canvas.getContext('2d');
    if (context === null) {
      finish(null);
      return;
    }
    video.onseeked = video.onloadeddata = null;
    context.drawImage(video, 0, 0, canvas.width, canvas.height);
    canvas.toBlob(finish, 'image/jpeg', 0.82);
  }
  video.muted = true;
  video.preload = 'metadata';
  video.onerror = abort;
  video.onloadedmetadata = () => {
    metadata.width = video.videoWidth;
    metadata.height = video.videoHeight;
    metadata.duration = Number.isFinite(video.duration)
      ? Math.max(0, video.duration)
      : 0;
    metadataResult.resolve({ ...metadata });
    video.currentTime = Math.min(0.1, metadata.duration / 3);
  };
  video.onseeked = video.onloadeddata = cover;
  signal.addEventListener('abort', abort, { once: true });
  if (signal.aborted) abort();
  else video.src = url;
  return result;
}
