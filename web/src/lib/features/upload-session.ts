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
}
/** Keep the established key so existing valid browser sessions remain resumable. */
const key = 'mori.pending-upload';
/** In-memory continuity when browser persistence is blocked. */
let memory: PendingUpload | null = null;

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
  };
}
/** Timestamps and byte lengths must survive JSON round trips exactly. */
function nonnegative(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}
/** Persist only validated session metadata, keeping a fallback when storage is blocked. */
export function rememberUpload(upload: PendingUpload | null): void {
  memory = upload;
  try {
    if (upload === null) localStorage.removeItem(key);
    else localStorage.setItem(key, JSON.stringify(upload));
  } catch {
    /* Browser persistence is optional. */
  }
}
/** Recover a safe resume identity while tolerating malformed or unavailable storage. */
export function savedUpload(): PendingUpload | null {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(key) ?? 'null');
    return parsePendingUpload(value) ?? memory;
  } catch {
    return memory;
  }
}
