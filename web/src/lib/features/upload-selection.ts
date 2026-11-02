/** Local selection validation is separate from transfer state and never changes original bytes. */
import type { UploadPolicy } from '#lib/api/client';

/** Choose the preview decoder using the server's enabled image containers. */
export function uploadKind(
  file: File,
  policy: UploadPolicy,
): 'image' | 'video' {
  return policy.image_extensions.includes(
    file.name.split('.').at(-1)?.toLowerCase() ?? '',
  )
    ? 'image'
    : 'video';
}

/** Append one valid picker/drop batch while preserving previous choices and their order. */
export function appendUploadFiles(
  current: File[],
  selected: File[],
  policy: UploadPolicy,
): File[] {
  for (const file of selected) {
    const extension = file.name.split('.').at(-1)?.toLowerCase() ?? '';
    if (!policy.allowed_extensions.includes(extension))
      throw new Error(file.name + ': unsupported media format.');
    const image = uploadKind(file, policy) === 'image';
    const limit = image
      ? policy.max_image_upload_bytes
      : policy.max_upload_bytes;
    if (file.size < 16 || file.size > limit)
      throw new Error(
        file.name +
          ': ' +
          (image ? 'image' : 'video') +
          ' must be within the ' +
          formatBytes(limit) +
          ' upload limit.',
      );
  }
  const keys = new Set<string>();
  return [...current, ...selected].filter((file) => {
    const key = JSON.stringify([
      file.name,
      file.size,
      file.lastModified,
      file.type,
    ]);
    if (keys.has(key)) return false;
    keys.add(key);
    return true;
  });
}

/** Keep selection summaries and individual cards on the same binary size scale. */
export function formatBytes(bytes: number): string {
  const units = ['B', 'KiB', 'MiB', 'GiB'];
  const index = Math.min(
    3,
    Math.max(0, Math.floor(Math.log(Math.max(1, bytes)) / Math.log(1024))),
  );
  return (
    (bytes / 1024 ** index).toLocaleString(undefined, {
      maximumFractionDigits: 1,
    }) +
    ' ' +
    (units[index] ?? 'B')
  );
}
