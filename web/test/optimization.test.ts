/** Preparation must never guess fidelity, persist unsafe paths, or accept pointless conversions. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { inspectImage } from '../src/lib/features/optimization/image.ts';
import {
  optimizationConcurrency,
  parsePreparedCopy,
  worthwhile,
} from '../src/lib/features/optimization/model.ts';
import { parsePreferences } from '../src/lib/features/preferences.ts';
/** Construct only enough PNG structure to exercise bounded preflight, without invoking a decoder. */
function png(depth = 8, extra = ''): Uint8Array {
  const bytes = new Uint8Array(extra === '' ? 45 : 57),
    view = new DataView(bytes.buffer);
  bytes.set([137, 80, 78, 71, 13, 10, 26, 10]);
  view.setUint32(8, 13);
  bytes.set(new TextEncoder().encode('IHDR'), 12);
  view.setUint32(16, 800);
  view.setUint32(20, 600);
  bytes[24] = depth;
  if (extra !== '') bytes.set(new TextEncoder().encode(extra), 37);
  bytes.set(new TextEncoder().encode('IDAT'), extra === '' ? 37 : 49);
  return bytes;
}
void test('known still image headers pass; animation, color profiles and 16-bit inputs stay original', () => {
  assert.deepEqual(inspectImage(png()), { width: 800, height: 600, png: true });
  for (const marker of ['acTL', 'iCCP', 'cICP'])
    assert.equal(inspectImage(png(8, marker)), null);
  assert.equal(inspectImage(png(16)), null);
  for (let n = 0; n < 33; n++)
    assert.equal(inspectImage(png().slice(0, n)), null);
  assert.equal(inspectImage(new Uint8Array(60)), null);
});
void test('prepared references reject arbitrary paths and corrupted or imprecise byte counts', () => {
  const copy = {
    id: 'a'.repeat(32),
    extension: 'webp',
    size: 100,
    created_at: 123,
  };
  assert.deepEqual(parsePreparedCopy(copy), copy);
  for (const value of [
    null,
    {},
    { ...copy, id: '../file' },
    { ...copy, extension: 'svg' },
    { ...copy, size: NaN },
    { ...copy, size: 15 },
    { ...copy, size: 2 ** 54 },
    { ...copy, created_at: -1 },
  ])
    assert.equal(parsePreparedCopy(value), null);
  assert.equal(worthwhile(1000, 950), true);
  assert.equal(worthwhile(1000, 951), false);
  assert.equal(worthwhile(1000, 1100), false);
});
void test('balanced defaults migrate safely and worker concurrency always leaves spare cores', () => {
  assert.equal(parsePreferences(null).uploadOptimization, 'balanced');
  for (const mode of ['original', 'balanced', 'quality'])
    assert.equal(
      parsePreferences({ uploadOptimization: mode }).uploadOptimization,
      mode,
    );
  assert.equal(
    parsePreferences({ uploadOptimization: 'unbounded' }).uploadOptimization,
    'balanced',
  );
  for (const [cores, expected] of [
    [1, 1],
    [4, 1],
    [8, 2],
    [64, 2],
    [NaN, 1],
    [-1, 1],
  ])
    assert.equal(optimizationConcurrency(cores ?? 1), expected);
});
