/** Shared complete metadata fixtures use the generated media contract. */
import type { MediaItem } from '../../src/lib/api/client';
/** Complete metadata fixture preserves the generated contract in geometry tests. */
export function videoFixture(index: number): MediaItem {
  const id = String(index).padStart(32, '0');
  return {
    kind: 'video',
    id,
    token: id,
    extension: 'mp4',
    name: id,
    album_id: null,
    width: index % 2 === 0 ? 1920 : 540,
    height: 1080,
    visibility: 'private',
    state: 'ready',
    size: 1024,
    duration: 20,
    poster: 0,
    created_at: index,
    revision: 0,
    last_error: null,
    media_url: '/file/' + id + '.mp4',
    poster_url: null,
  };
}
