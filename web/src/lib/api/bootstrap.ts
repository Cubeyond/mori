/** Request-scoped metadata avoids rendering a false anonymous shell during page startup. */
import type { components } from './generated/schema';

/** Initial DTOs are authorized by Rust and delivered in a private, uncached HTML response. */
interface PageBootstrap {
  /** The route for which the initial collection data was produced. */
  path: string;
  /** Authoritative cookie validation result without exposing the cookie itself. */
  session: components['schemas']['Session'];
  /** Initial administrator album choices, absent on public routes. */
  albums: components['schemas']['Album'][] | null;
  /** Initial authorized album and its bounded first page. */
  album_page: components['schemas']['AlbumPage'] | null;
}

/** Decode only the trusted server document; Vite and non-browser callers use ordinary API loading. */
function readBootstrap(): PageBootstrap | null {
  if (typeof document === 'undefined') return null;
  const element = document.getElementById('mori-bootstrap');
  if (element === null) return null;
  try {
    const value: unknown = JSON.parse(element.textContent);
    if (
      value === null ||
      typeof value !== 'object' ||
      !('path' in value) ||
      typeof value.path !== 'string' ||
      !('session' in value) ||
      value.session === null ||
      typeof value.session !== 'object' ||
      !('authenticated' in value.session) ||
      typeof value.session.authenticated !== 'boolean' ||
      !('csrf' in value.session) ||
      (value.session.csrf !== null && typeof value.session.csrf !== 'string')
    )
      return null;
    // The Rust serializer supplies the same generated DTOs used by authenticated JSON requests.
    return value as PageBootstrap;
  } catch {
    return null;
  }
}

/** Read once before stores and route components render, without persisting private metadata. */
export const bootstrap = readBootstrap();

/** Initial media data is single-use so later client-side returns fetch fresh metadata. */
let consumed = false;
/** Consume only the matching route's fresh document data without reusing it after navigation. */
export function takeInitialPage(path: string): PageBootstrap | null {
  if (consumed || bootstrap?.path !== path) return null;
  consumed = true;
  return bootstrap;
}
