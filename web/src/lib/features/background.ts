/** Browser-local background rendering retains at most one bounded derivative and object URL. */
import { writable, get } from 'svelte/store';
import { backgroundStorage } from './background-storage';
import { loadLocalImage, renderImagePreview } from './image';

/** Ready-to-render local object URL; never persisted or sent to the API. */
export const background = writable<string | null>(null);
/** A non-sensitive presence marker stabilizes layout before IndexedDB finishes loading. */
const marker = 'mori:background-present';
/** Read only a boolean during client initialization; image bytes remain in IndexedDB. */
function initialPresence(): boolean {
  try {
    return (
      typeof localStorage !== 'undefined' &&
      localStorage.getItem(marker) === '1'
    );
  } catch {
    return false;
  }
}
/** Geometry can be restored synchronously even though the local image loads asynchronously. */
export const hasBackground = writable(initialPresence());
/** Ignore an obsolete load if the user has already chosen or removed a background. */
let revision = 0;
/** Replace one URL only after its replacement was decoded and persisted successfully. */
function apply(blob: Blob | null): void {
  const previous = get(background);
  background.set(blob === null ? null : URL.createObjectURL(blob));
  hasBackground.set(blob !== null);
  try {
    if (blob === null) localStorage.removeItem(marker);
    else localStorage.setItem(marker, '1');
  } catch {
    /* Storage restrictions must not prevent an already available background rendering. */
  }
  if (previous !== null) URL.revokeObjectURL(previous);
}
/** Restore a browser-local image without turning unavailable persistence into an application error. */
export async function loadBackground(): Promise<void> {
  const current = revision;
  try {
    const blob = await backgroundStorage();
    if (current === revision) apply(blob);
  } catch {
    if (get(background) === null) hasBackground.set(false);
    /* No personal background is the safe fallback when browser storage is blocked. */
  }
}
/** Prepare a static bounded background; the original is never uploaded or retained. */
export async function saveBackground(
  file: File,
  signal: AbortSignal,
): Promise<void> {
  const current = ++revision;
  if (file.size > 10 * 1024 * 1024)
    throw new Error('Choose an image smaller than 10 MiB.');
  const local = await loadLocalImage(
    file,
    AbortSignal.any([signal, AbortSignal.timeout(10_000)]),
  );
  try {
    const { naturalWidth: width, naturalHeight: height } = local.image;
    if (width < 1 || height < 1 || width * height > 36_000_000)
      throw new Error('Choose an image with at most 36 megapixels.');
    const blob = await renderImagePreview(
      local.image,
      width,
      height,
      2560,
      'image/webp',
    );
    signal.throwIfAborted();
    if (blob === null || blob.size > 4 * 1024 * 1024)
      throw new Error('Choose a smaller background image.');
    if (current !== revision) return;
    await backgroundStorage(blob);
    if (current === revision) apply(blob);
  } finally {
    local.dispose();
  }
}
/** Remove persisted bytes and release the rendered object URL without touching other preferences. */
export async function removeBackground(): Promise<void> {
  const current = ++revision;
  await backgroundStorage(null);
  if (current === revision) apply(null);
}
