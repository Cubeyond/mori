/** Typed frontend module for browser-side Mori behavior. */
import { writable, get } from 'svelte/store';
import type { components } from './generated/schema';
import { bootstrap } from './bootstrap';
import { createApi, apiData } from './transport';
export { apiData } from './transport';
/** Typed media contract derived from the generated API or local browser state. */
export type MediaItem = components['schemas']['Video'];
/** Typed Album contract derived from the generated API or local browser state. */
export type Album = components['schemas']['Album'];
/** Administrator listing includes at most three completed poster previews per album. */
export type AlbumSummary = components['schemas']['AlbumSummary'];
/** Non-secret runtime policy returned by the upload API. */
export type UploadPolicy = components['schemas']['UploadPolicy'];
/** Redacted administrator storage summary generated from Rust DTOs. */
export type StorageInfo = components['schemas']['StorageInfo'];
/** Write-only storage setup fields generated from Rust DTOs. */
export type StorageInput = components['schemas']['StorageInput'];
/** Generated bulk-operation contract shared by gallery and album dialogs. */
export type BatchAction = components['schemas']['BatchAction'];
/** Server-discovered unfinished originals, independent of local browser persistence. */
export type UnfinishedUpload = components['schemas']['UnfinishedUpload'];
/** Typed Upload contract derived from the generated API or local browser state. */
export type Upload = components['schemas']['Upload'];
/** Session metadata used for UI state; the server authorizes every protected request. */
export const session = writable<{
  authenticated: boolean;
  csrf: string | null;
}>({
  authenticated: bootstrap?.session.authenticated ?? false,
  csrf: bootstrap?.session.csrf ?? null,
});
/** Shared generated client reads CSRF lazily, including after login and logout. */
export const api = createApi(
  typeof window === 'undefined' ? 'http://localhost' : window.location.origin,
  () => get(session).csrf,
);
api.use({
  /** Expired application sessions immediately update navigation and protected views. */
  onResponse({ response }): void {
    if (response.status === 401)
      session.set({ authenticated: false, csrf: null });
  },
});
/** Refresh the browser session without exposing its HttpOnly cookie. */
export async function loadSession(): Promise<void> {
  session.set(await apiData(api.GET('/v1/auth/session')));
}
/** Convert an unknown failure into a safe user-facing message. */
export const message = (error: unknown): string =>
  error instanceof Error
    ? error.message
    : 'Something went wrong. Please try again.';
