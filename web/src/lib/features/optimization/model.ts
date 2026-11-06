// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Product presets and validated references to exact browser-local upload copies. */
export type OptimizationMode = 'original' | 'balanced' | 'quality';
/** A closed list keeps encoding details out of the settings interface. */
export const optimizationModes: readonly {
  value: OptimizationMode;
  label: string;
  description: string;
}[] = [
  {
    value: 'balanced',
    label: 'Balanced',
    description: 'Balance file size and visual quality.',
  },
  {
    value: 'quality',
    label: 'Higher quality',
    description: 'Keep more detail, usually with larger files.',
  },
  {
    value: 'original',
    label: 'Keep originals',
    description: 'Upload files unchanged.',
  },
];
/** Migrate existing preferences to the balanced automatic policy. */
export function parseOptimizationMode(value: unknown): OptimizationMode {
  return value === 'original' || value === 'quality' ? value : 'balanced';
}
/** Immutable OPFS output, retained until its multipart upload completes or is cancelled. */
export interface PreparedCopy {
  /** Random identifier; never a user filename or arbitrary filesystem path. */
  id: string;
  /** Only these browser-generated media formats enter this cache. */
  extension: 'webp' | 'mp4';
  /** Exact byte count of the finalized upload source. */
  size: number;
  /** Creation time supports cleanup of abandoned preparation jobs. */
  created_at: number;
}
/** Reject malformed paths and byte lengths before touching browser storage. */
export function parsePreparedCopy(value: unknown): PreparedCopy | null {
  if (
    value === null ||
    typeof value !== 'object' ||
    !('id' in value) ||
    typeof value.id !== 'string' ||
    !/^[0-9a-f]{32}$/.test(value.id) ||
    !('extension' in value) ||
    (value.extension !== 'webp' && value.extension !== 'mp4') ||
    !('size' in value) ||
    typeof value.size !== 'number' ||
    !Number.isSafeInteger(value.size) ||
    value.size < 16 ||
    !('created_at' in value) ||
    typeof value.created_at !== 'number' ||
    !Number.isSafeInteger(value.created_at) ||
    value.created_at < 0
  )
    return null;
  return {
    id: value.id,
    extension: value.extension,
    size: value.size,
    created_at: value.created_at,
  };
}
/** Saving at least five percent avoids expensive conversions with negligible benefit. */
export function worthwhile(original: number, optimized: number): boolean {
  return optimized >= 16 && optimized <= Math.floor(original * 0.95);
}
/** One or two independent workers leave most reported logical cores available. */
export function optimizationConcurrency(cores: number): number {
  return Number.isFinite(cores)
    ? Math.max(1, Math.min(2, Math.floor(cores / 4)))
    : 1;
}
/** Dedicated-worker requests contain local files and policy, never S3 URLs or credentials. */
export interface OptimizationRequest {
  /** Browser-local original remains unchanged. */
  file: File;
  /** Server-selected media category. */
  kind: 'image' | 'video';
  /** Product preset captured when this batch begins. */
  mode: Exclude<OptimizationMode, 'original'>;
  /** Assigned before work starts so partial output can always be removed. */
  copy: PreparedCopy;
  /** Bound image decoding before allocating full-size pixel buffers. */
  maxPixels: number;
  /** Total wall-time available to this job, including parsing and writing. */
  budgetMs: number;
}
/** Only a finalized optional disk descriptor crosses back from the worker. */
export interface OptimizationResponse {
  /** Distinguish the final response from unexpected worker messages. */
  type: 'done';
  /** Null preserves the selected original without failing its upload. */
  copy: PreparedCopy | null;
}
