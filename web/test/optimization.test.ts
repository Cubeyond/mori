// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

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

/** A normal PNG cHRM block uses the standard sRGB white point and RGB primaries. */
function chromaticPng(values: readonly number[]): Uint8Array {
  const bytes = new Uint8Array(89),
    view = new DataView(bytes.buffer);
  bytes.set(png().subarray(0, 33));
  view.setUint32(33, 32);
  bytes.set(new TextEncoder().encode('cHRM'), 37);
  values.forEach((value, index) => {
    view.setUint32(41 + index * 4, value);
  });
  bytes.set(new TextEncoder().encode('IDAT'), 81);
  return bytes;
}
void test('standard sRGB chromaticities remain eligible while wide-gamut and malformed headers stay original', () => {
  const standard = [31270, 32900, 64000, 33000, 30000, 60000, 15000, 6000];
  assert.deepEqual(inspectImage(chromaticPng(standard)), {
    width: 800,
    height: 600,
    png: true,
  });
  assert.equal(
    inspectImage(
      chromaticPng([31270, 32900, 68000, 32000, 26500, 69000, 15000, 6000]),
    ),
    null,
  );
  const broken = chromaticPng(standard);
  new DataView(broken.buffer).setUint32(33, 28);
  assert.equal(inspectImage(broken), null);
  assert.equal(inspectImage(chromaticPng(standard).subarray(0, 65)), null);
});
