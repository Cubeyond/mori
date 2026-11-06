// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Bounded lightbox geometry, independent of DOM events and media decoding. */
export interface Size {
  /** Measured CSS width. */
  width: number;
  /** Measured CSS height. */
  height: number;
}
/** Translation is measured from the fitted media's center in viewport pixels. */
export interface View {
  /** Magnification relative to the fitted frame. */
  scale: number;
  /** Horizontal translation. */
  x: number;
  /** Vertical translation. */
  y: number;
}
/** Small, explicit control interface exposed by the gesture viewport. */
export interface ZoomControls {
  /** Change zoom by a multiplier, centered on the visible frame. */
  change: (factor: number) => void;
  /** Restore the complete fitted frame. */
  reset: () => void;
  /** Move the visible crop without requiring a drag gesture. */
  pan: (x: number, y: number) => void;
}
/** Fit without cropping or enlarging a small original beyond native size. */
export function fitMedia(stage: Size, source: Size): Size {
  const width = Math.max(1, source.width),
    height = Math.max(1, source.height);
  const factor = Math.max(
    0,
    Math.min(1, stage.width / width, stage.height / height),
  );
  return { width: width * factor, height: height * factor };
}
/** Clamp translation so zoomed content never leaves a blank edge inside its fit frame. */
export function boundView(view: View, fitted: Size, stage: Size): View {
  const scale = Math.max(1, Math.min(8, view.scale));
  const x = Math.max(0, (fitted.width * scale - stage.width) / 2);
  const y = Math.max(0, (fitted.height * scale - stage.height) / 2);
  return {
    scale,
    x: Math.max(-x, Math.min(x, view.x)),
    y: Math.max(-y, Math.min(y, view.y)),
  };
}
/** Preserve the image point beneath a pointer or pinch midpoint as scale changes. */
export function zoomAt(
  view: View,
  scale: number,
  point: { x: number; y: number },
  fitted: Size,
  stage: Size,
): View {
  const next = Math.max(1, Math.min(8, scale)),
    ratio = next / view.scale;
  return boundView(
    {
      scale: next,
      x: point.x - (point.x - view.x) * ratio,
      y: point.y - (point.y - view.y) * ratio,
    },
    fitted,
    stage,
  );
}
/** Normalize pixel, line, and page wheels, limiting one event's zoom jump. */
export function wheelPixels(
  delta: number,
  mode: number,
  pageHeight: number,
): number {
  return Math.max(
    -120,
    Math.min(120, delta * (mode === 1 ? 16 : mode === 2 ? pageHeight : 1)),
  );
}
