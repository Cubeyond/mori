/** Browser-scoped presentation and upload defaults; access control remains server-enforced. */
import { writable } from 'svelte/store';
/** Explicitly supported local defaults without credentials or media metadata. */
export interface Preferences {
  uploadVisibility: 'private' | 'public';
  density: 'comfortable' | 'compact';
  /** Image alpha; a conservative default preserves the dark interface. */
  backgroundOpacity: number;
}
/** Safe defaults used when browser persistence is unavailable. */
const defaults: Preferences = {
  uploadVisibility: 'private',
  density: 'comfortable',
  backgroundOpacity: 0.1,
};
/** Reactive local preferences shared by settings, upload, and thumbnail layout. */
export const preferences = writable<Preferences>({ ...defaults });
/** Restore only validated values from this browser's local storage. */
export function loadPreferences(): void {
  try {
    const raw: unknown = JSON.parse(
      localStorage.getItem('mori:preferences') ?? 'null',
    );
    preferences.set(parsePreferences(raw));
  } catch {
    preferences.set({ ...defaults });
  }
}
/** Validate persisted appearance values and migrate older browser preferences safely. */
export function parsePreferences(raw: unknown): Preferences {
  if (raw === null || typeof raw !== 'object') return { ...defaults };
  return {
    uploadVisibility:
      'uploadVisibility' in raw && raw.uploadVisibility === 'public'
        ? 'public'
        : 'private',
    density:
      'density' in raw && raw.density === 'compact' ? 'compact' : 'comfortable',
    backgroundOpacity:
      'backgroundOpacity' in raw &&
      typeof raw.backgroundOpacity === 'number' &&
      Number.isFinite(raw.backgroundOpacity) &&
      raw.backgroundOpacity >= 0 &&
      raw.backgroundOpacity <= 1
        ? raw.backgroundOpacity
        : defaults.backgroundOpacity,
  };
}
/** Apply in-memory defaults and report whether browser persistence succeeded. */
export function savePreferences(value: Preferences): boolean {
  preferences.set(value);
  try {
    localStorage.setItem('mori:preferences', JSON.stringify(value));
    return true;
  } catch {
    return false;
  }
}
