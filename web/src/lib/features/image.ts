// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** A decoded local image owns its URL until the caller releases the bitmap. */
export interface LocalImage {
  /** Browser decoding keeps unsupported formats out of canvas processing. */
  image: HTMLImageElement;
  /** Release local data after a preview has been drawn. */
  dispose: () => void;
}
/** Decode local bytes without uploading, retaining object URLs, or fetching an external image. */
export function loadLocalImage(
  blob: Blob,
  signal: AbortSignal,
): Promise<LocalImage> {
  signal.throwIfAborted();
  return new Promise<LocalImage>((resolve, reject) => {
    const image = new Image();
    const url = URL.createObjectURL(blob);
    /** Release event handlers and the object URL on success disposal or any failure. */
    function dispose(): void {
      image.onload = image.onerror = null;
      signal.removeEventListener('abort', abort);
      image.removeAttribute('src');
      URL.revokeObjectURL(url);
    }
    /** Interrupt decoding when a newer selection replaces this image. */
    function abort(): void {
      dispose();
      reject(new DOMException('Image decoding cancelled', 'AbortError'));
    }
    image.decoding = 'async';
    image.onload = () => {
      signal.removeEventListener('abort', abort);
      image.onload = image.onerror = null;
      resolve({ image, dispose });
    };
    image.onerror = () => {
      dispose();
      reject(new Error('This browser cannot preview this image format.'));
    };
    signal.addEventListener('abort', abort, { once: true });
    image.src = url;
  });
}
/** Make a bounded derivative in the browser while leaving uploaded originals byte-for-byte intact. */
export function renderImagePreview(
  source: CanvasImageSource,
  width: number,
  height: number,
  maximum = 1280,
  type = 'image/jpeg',
): Promise<Blob | null> {
  const canvas = document.createElement('canvas');
  const scale = Math.min(1, maximum / width, maximum / height);
  canvas.width = Math.max(1, Math.round(width * scale));
  canvas.height = Math.max(1, Math.round(height * scale));
  const context = canvas.getContext('2d');
  if (context === null) return Promise.resolve(null);
  context.fillStyle = '#0b0c14';
  context.fillRect(0, 0, canvas.width, canvas.height);
  context.drawImage(source, 0, 0, canvas.width, canvas.height);
  return new Promise((resolve) => {
    canvas.toBlob(resolve, type, 0.82);
  });
}
