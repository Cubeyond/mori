/** Existing preferences survive appearance upgrades; corrupted opacity never breaks rendering. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parsePreferences } from '../src/lib/features/preferences.ts';

void test('older browser settings retain choices and receive a 10 percent background default', () => {
  assert.deepEqual(
    parsePreferences({ density: 'compact', uploadVisibility: 'public' }),
    { density: 'compact', uploadVisibility: 'public', backgroundOpacity: 0.1 },
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
