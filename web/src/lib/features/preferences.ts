// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

/** Browser-scoped presentation and upload defaults; access control remains server-enforced. */
import {
  parseOptimizationMode,
  type OptimizationMode,
} from '#lib/features/optimization/model';
import { writable } from 'svelte/store';
import {
  applyTheme,
  parseTheme,
  parseAccent,
  type Theme,
  type Accent,
} from '#lib/features/theme';
/** Explicitly supported local defaults without credentials or media metadata. */
export interface Preferences {
  /** Browser-local palette; saved explicit themes survive the default-theme upgrade. */
  theme: Theme;
  /** Accent colors apply to actions, selections, focus and progress. */
  accent: Accent;
  /** Visibility applied only to new uploads. */
  uploadVisibility: 'private' | 'public';
  /** Local media preparation preset; resumable sessions keep their existing bytes. */
  uploadOptimization: OptimizationMode;
  /** Local spacing and width of masonry columns. */
  density: 'comfortable' | 'compact';
  /** Image alpha; a conservative default preserves the dark interface. */
  backgroundOpacity: number;
  /** Explicit opt-in; original media viewing always remains fully opaque. */
  translucentThumbnails: boolean;
  /** Thumbnail alpha when translucency is enabled, independent of the wallpaper alpha. */
  thumbnailOpacity: number;
}
/** Conservative starting alpha when thumbnail translucency is explicitly enabled. */
export const DEFAULT_THUMBNAIL_OPACITY = 0.92;
/** Safe defaults used when browser persistence is unavailable. */
const defaults: Preferences = {
  theme: 'mori',
  accent: 'green',
  uploadVisibility: 'private',
  uploadOptimization: 'balanced',
  density: 'comfortable',
  backgroundOpacity: 0.1,
  translucentThumbnails: false,
  thumbnailOpacity: DEFAULT_THUMBNAIL_OPACITY,
};
/** Reactive local preferences shared by settings, upload, and thumbnail layout. */
export const preferences = writable<Preferences>({ ...defaults });
/** Restore only validated values from this browser's local storage. */
export function loadPreferences(): void {
  try {
    const raw: unknown = JSON.parse(
      localStorage.getItem('mori:preferences') ?? 'null',
    );
    const value = parsePreferences(raw);
    preferences.set(value);
    applyTheme(value.theme, value.accent);
  } catch {
    preferences.set({ ...defaults });
    applyTheme(defaults.theme, defaults.accent);
  }
}
/** Validate persisted appearance values and migrate older browser preferences safely. */
export function parsePreferences(raw: unknown): Preferences {
  if (raw === null || typeof raw !== 'object') return { ...defaults };
  return {
    theme: parseTheme('theme' in raw ? raw.theme : undefined),
    accent: parseAccent('accent' in raw ? raw.accent : undefined),
    uploadVisibility:
      'uploadVisibility' in raw && raw.uploadVisibility === 'public'
        ? 'public'
        : 'private',
    uploadOptimization: parseOptimizationMode(
      'uploadOptimization' in raw ? raw.uploadOptimization : undefined,
    ),
    density:
      'density' in raw && raw.density === 'compact' ? 'compact' : 'comfortable',
    backgroundOpacity: opacity(raw, 'backgroundOpacity'),
    translucentThumbnails:
      'translucentThumbnails' in raw && raw.translucentThumbnails === true,
    thumbnailOpacity: opacity(raw, 'thumbnailOpacity'),
  };
}
/** Accept finite normalized alpha values while migrating absent or damaged settings. */
function opacity(
  raw: object,
  key: 'backgroundOpacity' | 'thumbnailOpacity',
): number {
  const value: unknown = key in raw ? Reflect.get(raw, key) : undefined;
  return typeof value === 'number' &&
    Number.isFinite(value) &&
    value >= 0 &&
    value <= 1
    ? value
    : defaults[key];
}
/** Apply in-memory defaults and report whether browser persistence succeeded. */
export function savePreferences(value: Preferences): boolean {
  preferences.set(value);
  applyTheme(value.theme, value.accent);
  try {
    localStorage.setItem('mori:preferences', JSON.stringify(value));
    return true;
  } catch {
    return false;
  }
}
