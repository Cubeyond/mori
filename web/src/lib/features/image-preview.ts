/** Image metadata and thumbnails share the multipart uploader without transforming originals. */
import {
  loadLocalImage,
  renderImagePreview,
  type LocalImage,
} from '#lib/features/image';
import type { MediaMetadata, UploadPreview } from './upload-preview';

/** Unsupported image codecs retain their original bytes and resolve to a neutral preview. */
export function prepareImagePreview(
  file: File,
  signal: AbortSignal,
): UploadPreview {
  const metadata = { width: 0, height: 0, duration: 0 };
  const details = Promise.withResolvers<MediaMetadata>();
  const poster = Promise.withResolvers<Blob | null>();
  const decoder = new AbortController();
  let local: LocalImage | null = null;
  let finished = false;
  const timer = setTimeout(() => {
    finish(null);
  }, 3000);
  /** Settle both stages and close the decoder exactly once. */
  function finish(value: Blob | null): void {
    if (finished) return;
    finished = true;
    clearTimeout(timer);
    decoder.abort();
    signal.removeEventListener('abort', abort);
    details.resolve(metadata);
    poster.resolve(value);
    local?.dispose();
  }
  /** Cancelling a batch also releases an image still being decoded. */
  function abort(): void {
    finish(null);
  }
  signal.addEventListener('abort', abort, { once: true });
  if (signal.aborted) abort();
  else
    void loadLocalImage(file, decoder.signal)
      .then(async (decoded) => {
        local = decoded;
        if (finished) {
          local.dispose();
          return;
        }
        metadata.width = decoded.image.naturalWidth;
        metadata.height = decoded.image.naturalHeight;
        details.resolve({ ...metadata });
        finish(
          await renderImagePreview(
            decoded.image,
            metadata.width,
            metadata.height,
          ),
        );
      })
      .catch(() => {
        finish(null);
      });
  return { metadata: details.promise, poster: poster.promise, dispose: abort };
}
