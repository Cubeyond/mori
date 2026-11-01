/** Shared public URL and iframe formatting for gallery controls and the sharing panel. */
import type { Video } from '#lib/api/client';

/** The only public sharing formats are a direct media resource and iframe markup. */
export type ShareKind = 'media' | 'iframe';
/** Escape an attribute in embeddable HTML without changing its underlying sharing URL. */
function attribute(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('"', '&quot;')
    .replaceAll('<', '&lt;');
}
/** Produce sharing markup that delegates media controls through cross-origin S3 redirects. */
export function shareValue(video: Video, kind: ShareKind): string {
  if (kind === 'media') return video.media_url;
  const ratio =
    video.width > 0 && video.height > 0
      ? `${String(video.width)} / ${String(video.height)}`
      : '16 / 9';
  const sandbox =
    video.kind === 'image'
      ? ' sandbox="allow-same-origin allow-downloads"'
      : '';
  return `<iframe src="${attribute(video.media_url)}" title="${attribute(video.name)}" style="width:100%;height:auto;aspect-ratio:${ratio};border:0"${sandbox} allow="fullscreen *; picture-in-picture *" allowfullscreen loading="lazy" referrerpolicy="no-referrer"></iframe>`;
}
