/** Bounded local resume metadata; multipart state is always reloaded from the server. */
import type { Upload } from '../api/client';

/** Only the session identity and local file match are persisted in the browser. */
export interface PendingUpload extends Pick<
  Upload,
  'id' | 'name' | 'expires_at'
> {
  /** Original file length used to reject mismatched resume selections. */
  size: number;
  /** Original file modification timestamp. */
  modified: number;
  /** SHA-256 of a bounded file prefix. */
  fingerprint: string;
  /** Original picker name distinguishes byte-identical files in concurrent selections. */
  source_name?: string;
}
/** Keep the established key so existing valid browser sessions remain resumable. */
const key = 'mori.pending-upload';
/** In-memory continuity when browser persistence is blocked. */
let memory: PendingUpload[] = [];

/** Accept only complete finite metadata rather than asserting arbitrary stored JSON. */
export function parsePendingUpload(value: unknown): PendingUpload | null {
  if (value === null || typeof value !== 'object') return null;
  if (
    !('id' in value) ||
    typeof value.id !== 'string' ||
    !/^[0-9a-f]{32}$/.test(value.id) ||
    !('name' in value) ||
    typeof value.name !== 'string' ||
    value.name.length === 0 ||
    value.name.length > 200 ||
    !('expires_at' in value) ||
    !nonnegative(value.expires_at) ||
    !('size' in value) ||
    !nonnegative(value.size) ||
    value.size < 16 ||
    !('modified' in value) ||
    !nonnegative(value.modified) ||
    !('fingerprint' in value) ||
    typeof value.fingerprint !== 'string' ||
    !/^[0-9a-f]{64}$/.test(value.fingerprint)
  )
    return null;
  return {
    id: value.id,
    name: value.name,
    expires_at: value.expires_at,
    size: value.size,
    modified: value.modified,
    fingerprint: value.fingerprint,
    ...('source_name' in value && typeof value.source_name === 'string'
      ? { source_name: value.source_name }
      : {}),
  };
}
/** Timestamps and byte lengths must survive JSON round trips exactly. */
function nonnegative(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}
/** Migrate the old single-file format and validate each independent concurrent session. */
export function parsePendingUploads(value: unknown): PendingUpload[] {
  const entries: unknown[] = Array.isArray(value) ? value : [value];
  const uploads = new Map<string, PendingUpload>();
  for (const entry of entries) {
    const parsed = parsePendingUpload(entry);
    if (parsed !== null) uploads.set(parsed.id, parsed);
  }
  return [...uploads.values()];
}
/** Persist the current session list without discarding another worker's resumable identity. */
function persist(uploads: PendingUpload[]): void {
  memory = uploads;
  try {
    if (uploads.length === 0) localStorage.removeItem(key);
    else localStorage.setItem(key, JSON.stringify(uploads));
  } catch {
    /* Browser persistence is optional. */
  }
}
/** Recover a safe resume identity while tolerating malformed or unavailable storage. */
export function savedUploads(): PendingUpload[] {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(key) ?? 'null');
    const parsed = parsePendingUploads(value);
    return parsed.length > 0 ? parsed : memory;
  } catch {
    return memory;
  }
}
/** Upsert only this file's identity; concurrent workers retain their own sessions. */
export function rememberUpload(upload: PendingUpload): void {
  persist([
    ...savedUploads().filter((entry) => entry.id !== upload.id),
    upload,
  ]);
}
/** Remove one completed or explicitly cancelled session without touching other uploads. */
export function forgetUpload(id: string): void {
  persist(savedUploads().filter((upload) => upload.id !== id));
}
