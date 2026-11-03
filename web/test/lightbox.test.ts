/** Lightbox geometry keeps sources intact and protects focal points across gestures. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  fitMedia,
  boundView,
  zoomAt,
  wheelPixels,
} from '../src/lib/features/lightbox.ts';

void test('fit preserves portrait and landscape ratios without upscaling small originals', () => {
  assert.deepEqual(
    fitMedia({ width: 800, height: 600 }, { width: 2000, height: 1000 }),
    { width: 800, height: 400 },
  );
  assert.deepEqual(
    fitMedia({ width: 800, height: 600 }, { width: 1000, height: 2000 }),
    { width: 300, height: 600 },
  );
  assert.deepEqual(
    fitMedia({ width: 800, height: 600 }, { width: 80, height: 60 }),
    { width: 80, height: 60 },
  );
});
void test('zoom keeps the pointer anchored until reaching a physical content boundary', () => {
  const stage = { width: 800, height: 600 },
    fit = { width: 800, height: 600 };
  const point = { x: 100, y: -80 };
  const next = zoomAt({ scale: 1, x: 0, y: 0 }, 2, point, fit, stage);
  assert.deepEqual(next, { scale: 2, x: -100, y: 80 });
  assert.equal((point.x - next.x) / next.scale, point.x);
  assert.equal((point.y - next.y) / next.scale, point.y);
  assert.deepEqual(zoomAt(next, 1, point, fit, stage), {
    scale: 1,
    x: 0,
    y: 0,
  });
});
void test('pan bounds remove blank edges, survive resize, and normalize wheel modes', () => {
  const fit = { width: 800, height: 400 },
    stage = { width: 800, height: 600 };
  assert.deepEqual(boundView({ scale: 2, x: 9999, y: -9999 }, fit, stage), {
    scale: 2,
    x: 400,
    y: -100,
  });
  assert.deepEqual(boundView({ scale: 0, x: 100, y: 100 }, fit, stage), {
    scale: 1,
    x: 0,
    y: 0,
  });
  assert.equal(boundView({ scale: 99, x: 0, y: 0 }, fit, stage).scale, 8);
  assert.equal(wheelPixels(1, 1, 600), wheelPixels(16, 0, 600));
  assert.equal(wheelPixels(-1, 2, 600), -120);
  assert.equal(wheelPixels(10000, 0, 600), 120);
});
