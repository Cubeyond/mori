// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Masonry regressions cover responsive bounds, uniform gutters, and stable focus order. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { arrangeGallery } from '../src/lib/features/gallery.ts';
import type { MediaItem } from '../src/lib/api/client';
import { videoFixture } from './support/media.ts';
/** Allow floating point subpixels without accepting visible gutter drift. */
function near(actual: number, expected: number): void {
  assert.ok(Math.abs(actual - expected) < 0.001);
}
void test('masonry preserves DOM order and equal gutters without intersections across viewport sizes', () => {
  const videos = Array.from({ length: 41 }, (_, i) => videoFixture(i));
  for (const width of [288, 370, 520, 760, 880, 1096, 1336]) {
    for (const compact of [false, true]) {
      const layout = arrangeGallery(videos, width, compact);
      assert.deepEqual(
        layout.items.map((item) => item.video.id),
        videos.map((video) => video.id),
      );
      for (const [i, a] of layout.items.entries()) {
        assert.ok(
          a.x >= 0 &&
            a.y >= 0 &&
            a.x + a.width <= width + 0.001 &&
            a.y + a.height <= layout.height + 0.001,
        );
        near(a.width, layout.items[0]?.width ?? 0);
        for (const b of layout.items.slice(i + 1)) {
          assert.ok(
            a.x + a.width <= b.x + 0.001 ||
              b.x + b.width <= a.x + 0.001 ||
              a.y + a.height <= b.y + 0.001 ||
              b.y + b.height <= a.y + 0.001,
          );
        }
      }
      for (let column = 0; column < layout.columns; column++) {
        const entries = layout.items.filter((item) => item.column === column);
        for (let i = 1; i < entries.length; i++) {
          const a = entries[i - 1],
            b = entries[i];
          assert.ok(a && b);
          near(b.y - a.y - a.height, 14);
        }
      }
    }
  }
});
void test('appending records keeps existing covers stationary and leaves native metadata intact', () => {
  const videos = Array.from({ length: 12 }, (_, i) => videoFixture(i));
  const original = structuredClone(videos);
  const first = arrangeGallery(videos.slice(0, 7), 1096);
  const next = arrangeGallery(videos, 1096);
  assert.deepEqual(first.items, next.items.slice(0, 7));
  assert.deepEqual(videos, original);
  assert.equal(first.columns, 4);
  assert.ok(
    new Set(first.items.map((item) => Math.round(item.height))).size >= 3,
  );
  assert.ok(arrangeGallery(videos, 1096, true).columns > next.columns);
});
void test('empty and unmeasured galleries reserve no phantom black panel', () => {
  assert.equal(arrangeGallery([], 1096).height, 0);
  for (const width of [0, -100, Number.NaN, Number.POSITIVE_INFINITY])
    assert.equal(arrangeGallery([videoFixture(1)], width).items.length, 0);
});

void test('images and videos share masonry order without changing their native metadata', () => {
  const image: MediaItem = {
    ...videoFixture(2),
    kind: 'image',
    extension: 'jpg',
    duration: 0,
    width: 800,
    height: 1200,
  };
  const video = videoFixture(3);
  const mixed = [image, video];
  const snapshot = structuredClone(mixed);
  const layout = arrangeGallery(mixed, 880);
  assert.deepEqual(
    layout.items.map((item) => item.video.kind),
    ['image', 'video'],
  );
  assert.deepEqual(mixed, snapshot);
});
