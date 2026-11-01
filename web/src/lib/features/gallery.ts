/** Equal-width thumbnail masonry with stable row-major reading and focus order. */
import type { Video } from '#lib/api/client';
/** Cropped thumbnail geometry; original video dimensions are never modified. */
export interface VideoPlacement {
  video: Video;
  x: number;
  y: number;
  width: number;
  height: number;
  column: number;
}
/** Reserved dimensions prevent poster loading from shifting surrounding content. */
export interface GalleryLayout {
  items: VideoPlacement[];
  height: number;
  columns: number;
}
/** Cover formats reproduce the reference's square, portrait, and landscape rhythm. */
const covers = [
  1,
  2 / 3,
  0.95,
  2 / 3,
  2 / 3,
  2 / 3,
  0.8,
  0.7,
  1.45,
  1,
  0.8,
  1.5,
];
/** Distribute records across equal columns while keeping metadata and DOM order intact. */
export function arrangeGallery(
  videos: Video[],
  width: number,
  compact = false,
  spacing = 14,
): GalleryLayout {
  const items: VideoPlacement[] = [];
  if (!Number.isFinite(width) || width <= 0 || videos.length === 0)
    return { items, height: 0, columns: 0 };
  const gap = Number.isFinite(spacing)
    ? Math.max(0, Math.min(spacing, width / 8))
    : 14;
  const columns = Math.min(
    compact ? 5 : 4,
    Math.max(
      width >= 280 && width < 480 ? 2 : 1,
      Math.floor((width + gap) / ((compact ? 170 : 220) + gap)),
    ),
  );
  const cellWidth = (width - gap * (columns - 1)) / columns;
  const bottoms = Array.from({ length: columns }, () => 0);
  for (const [index, video] of videos.entries()) {
    const column = index % columns;
    const aspect = covers[index % covers.length] ?? 1;
    const height = cellWidth / aspect;
    const y = bottoms[column] ?? 0;
    items.push({
      video,
      x: column * (cellWidth + gap),
      y,
      width: cellWidth,
      height,
      column,
    });
    bottoms[column] = y + height + gap;
  }
  return { items, height: Math.max(...bottoms) - gap, columns };
}
