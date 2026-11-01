/** Keep user-facing media operations distinct from immediate access-control withdrawal. */
import type { MediaItem } from '#lib/api/client';

/** Card state exposes a single operation label without misleading privacy badges. */
export interface MediaStatus {
  /** A complete original can be opened while publication changes are in progress. */
  ready: boolean;
  /** Incomplete or destructive operation shown independently of thumbnail appearance. */
  label: string;
  /** Stable access choice; deletion's forced-private state is not a user preference. */
  visibility: string;
}

/** Translate lifecycle state into display semantics without weakening server authorization. */
export function mediaStatus(
  media: Pick<MediaItem, 'kind' | 'state' | 'visibility'>,
): MediaStatus {
  const ready = ['ready', 'published', 'publishing', 'revoking'].includes(
    media.state,
  );
  return {
    ready,
    label: ready
      ? ''
      : media.state === 'failed'
        ? 'Upload failed'
        : media.state === 'deleting'
          ? 'Deleting ' + media.kind + '…'
          : 'Processing ' + media.kind + '…',
    visibility:
      media.state !== 'deleting' && media.visibility === 'private'
        ? 'Private'
        : '',
  };
}
