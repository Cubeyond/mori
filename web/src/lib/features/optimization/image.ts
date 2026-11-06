// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

/** Conservative image inspection avoids flattening animation or altering unverified color formats. */
import type { OptimizationMode } from './model';
/** Only supported eight-bit, standard-color still images reach a bitmap decoder. */
export interface ImageShape {
  /** Encoded width before EXIF orientation. */
  width: number;
  /** Encoded height before EXIF orientation. */
  height: number;
  /** PNG receives a more conservative near-transparent quality setting. */
  png: boolean;
}
/** Compare fixed ASCII markers without decoding arbitrary media bytes as text. */
function marker(bytes: Uint8Array, at: number, text: string): boolean {
  for (let i = 0; i < text.length; i++)
    if (bytes[at + i] !== text.charCodeAt(i)) return false;
  return true;
}
/** Parse bounded headers; unknown, profiled, animated, high-bit-depth and malformed inputs pass through. */
export function inspectImage(bytes: Uint8Array): ImageShape | null {
  const data = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (
    bytes.length >= 33 &&
    bytes[0] === 137 &&
    marker(bytes, 1, 'PNG\r\n\x1a\n')
  ) {
    if (bytes[24] !== 8) return null;
    let at = 8;
    while (at + 12 <= bytes.length) {
      const length = data.getUint32(at),
        end = at + 12 + length;
      if (
        marker(bytes, at + 4, 'acTL') ||
        marker(bytes, at + 4, 'iCCP') ||
        marker(bytes, at + 4, 'cICP')
      )
        return null;
      if (
        marker(bytes, at + 4, 'gAMA') &&
        (length !== 4 || end > bytes.length || data.getUint32(at + 8) !== 45455)
      )
        return null;
      if (marker(bytes, at + 4, 'cHRM')) {
        // W3C PNG Table 17 defines these exact standard sRGB chromaticities.
        const srgb = [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000];
        if (
          length !== 32 ||
          end > bytes.length ||
          !srgb.every(
            (value, index) => data.getUint32(at + 8 + index * 4) === value,
          )
        )
          return null;
      }
      if (marker(bytes, at + 4, 'IDAT'))
        return {
          width: data.getUint32(16),
          height: data.getUint32(20),
          png: true,
        };
      if (end > bytes.length || end <= at) return null;
      at = end;
    }
    return null;
  }
  if (bytes.length >= 4 && bytes[0] === 255 && bytes[1] === 216) {
    let at = 2,
      shape: ImageShape | null = null;
    while (at + 4 <= bytes.length) {
      if (bytes[at] !== 255) return null;
      const tag = bytes[at + 1],
        length = data.getUint16(at + 2),
        end = at + 2 + length;
      if (tag === 218) return shape;
      if (length < 2 || end > bytes.length) return null;
      if (tag === 226 && marker(bytes, at + 4, 'ICC_PROFILE')) return null;
      if (tag === 235 || tag === 238) return null;
      if (tag === 225) {
        const text = new TextDecoder().decode(bytes.subarray(at + 4, end));
        if (/hdrgm|gainmap|hdrgain/iu.test(text)) return null;
      }
      if (tag === 192 || tag === 193 || tag === 194) {
        if (length < 8 || bytes[at + 4] !== 8 || bytes[at + 9] !== 3)
          return null;
        shape = {
          width: data.getUint16(at + 7),
          height: data.getUint16(at + 5),
          png: false,
        };
      }
      at = end;
    }
    return null;
  }
  if (
    bytes.length >= 30 &&
    marker(bytes, 0, 'RIFF') &&
    marker(bytes, 8, 'WEBP')
  ) {
    if (marker(bytes, 12, 'VP8X')) {
      if ((bytes[20] ?? 0) & (0x20 | 0x04 | 0x02)) return null;
      return {
        width:
          1 +
          ((bytes[24] ?? 0) |
            ((bytes[25] ?? 0) << 8) |
            ((bytes[26] ?? 0) << 16)),
        height:
          1 +
          ((bytes[27] ?? 0) |
            ((bytes[28] ?? 0) << 8) |
            ((bytes[29] ?? 0) << 16)),
        png: false,
      };
    }
    if (
      marker(bytes, 12, 'VP8 ') &&
      bytes[23] === 157 &&
      bytes[24] === 1 &&
      bytes[25] === 42
    )
      return {
        width: data.getUint16(26, true) & 0x3fff,
        height: data.getUint16(28, true) & 0x3fff,
        png: false,
      };
    if (marker(bytes, 12, 'VP8L') && bytes[20] === 47) {
      const packed = data.getUint32(21, true);
      return {
        width: 1 + (packed & 0x3fff),
        height: 1 + ((packed >>> 14) & 0x3fff),
        png: false,
      };
    }
  }
  return null;
}
/** Native browser encoders run in a dedicated worker and retain transparency and baked-in orientation. */
export async function optimizeImage(
  file: File,
  mode: Exclude<OptimizationMode, 'original'>,
  maxPixels: number,
): Promise<Blob | null> {
  if (
    typeof createImageBitmap !== 'function' ||
    typeof OffscreenCanvas === 'undefined'
  )
    return null;
  const shape = inspectImage(
    new Uint8Array(await file.slice(0, 524_288).arrayBuffer()),
  );
  if (
    shape === null ||
    shape.width <= 0 ||
    shape.height <= 0 ||
    shape.width * shape.height > maxPixels
  )
    return null;
  const bitmap = await createImageBitmap(file, {
    imageOrientation: 'from-image',
    colorSpaceConversion: 'default',
  });
  try {
    const canvas = new OffscreenCanvas(bitmap.width, bitmap.height),
      context = canvas.getContext('2d', { alpha: true, colorSpace: 'srgb' });
    if (context === null) return null;
    context.drawImage(bitmap, 0, 0);
    const result = await canvas.convertToBlob({
      type: 'image/webp',
      quality: shape.png
        ? mode === 'quality'
          ? 0.99
          : 0.98
        : mode === 'quality'
          ? 0.95
          : 0.9,
    });
    return result.type === 'image/webp' ? result : null;
  } finally {
    bitmap.close();
  }
}
