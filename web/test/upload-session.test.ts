/** Stored browser data never crosses the upload interface without shape validation. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { parsePendingUpload } from '../src/lib/features/upload-session.ts';

/** Minimal metadata shared by old and current browser resume formats. */
const pending = {
  id: 'a'.repeat(32),
  name: 'Display',
  expires_at: 2_000_000_000,
  size: 1024,
  modified: 0,
  fingerprint: 'b'.repeat(64),
};

void test('resume parsing preserves identity but discards cached multipart and storage data', () => {
  assert.deepEqual(
    parsePendingUpload({
      ...pending,
      parts: [],
      part_size: 16_777_216,
      unexpected: 'ignored',
    }),
    pending,
  );
});

void test('resume parsing rejects missing, malformed, and non-finite fields', () => {
  for (const value of [
    null,
    'value',
    {},
    { id: 'only-id', size: 1024 },
    { ...pending, id: '../another/path' },
    { ...pending, size: NaN },
    { ...pending, modified: -1 },
    { ...pending, expires_at: Infinity },
    { ...pending, fingerprint: 'invalid' },
  ]) {
    assert.equal(parsePendingUpload(value), null);
  }
});
