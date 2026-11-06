// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

/** Existing preferences survive appearance upgrades; corrupted opacity never breaks rendering. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parsePreferences } from '../src/lib/features/preferences.ts';

void test('older browser settings retain choices and receive a 10 percent background default', () => {
  assert.deepEqual(
    parsePreferences({ density: 'compact', uploadVisibility: 'public' }),
    {
      theme: 'mori',
      accent: 'green',
      density: 'compact',
      uploadVisibility: 'public',
      uploadOptimization: 'balanced',
      backgroundOpacity: 0.1,
      translucentThumbnails: false,
      thumbnailOpacity: 0.92,
    },
  );
});

void test('opacity accepts the full slider range and rejects malformed persisted values', () => {
  for (const value of [0, 0.1, 0.37, 1])
    assert.equal(
      parsePreferences({ backgroundOpacity: value }).backgroundOpacity,
      value,
    );
  for (const value of [-1, 2, NaN, Infinity, '50', null])
    assert.equal(
      parsePreferences({ backgroundOpacity: value }).backgroundOpacity,
      0.1,
    );
  assert.equal(parsePreferences(null).backgroundOpacity, 0.1);
});

void test('thumbnail translucency requires explicit opt-in and preserves independent alpha', () => {
  for (const value of [null, 'true', 1, false])
    assert.equal(
      parsePreferences({ translucentThumbnails: value }).translucentThumbnails,
      false,
    );
  const selected = parsePreferences({
    translucentThumbnails: true,
    thumbnailOpacity: 0.63,
    backgroundOpacity: 0.2,
  });
  assert.equal(selected.translucentThumbnails, true);
  assert.equal(selected.thumbnailOpacity, 0.63);
  assert.equal(selected.backgroundOpacity, 0.2);
  for (const value of [0, 0.92, 1])
    assert.equal(
      parsePreferences({ thumbnailOpacity: value }).thumbnailOpacity,
      value,
    );
  for (const value of [-1, 2, NaN, Infinity, '50', null])
    assert.equal(
      parsePreferences({ thumbnailOpacity: value }).thumbnailOpacity,
      0.92,
    );
});
