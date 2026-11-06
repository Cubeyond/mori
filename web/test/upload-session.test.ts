// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Stored browser data never crosses the upload interface without shape validation. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  parsePendingUpload,
  parsePendingUploads,
  rememberUpload,
  savedUploads,
  forgetUpload,
} from '../src/lib/features/upload-session.ts';

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

void test('legacy single sessions migrate and concurrent completion never clears a sibling', () => {
  const second = {
    ...pending,
    id: 'c'.repeat(32),
    fingerprint: 'd'.repeat(64),
  };
  assert.deepEqual(parsePendingUploads(pending), [pending]);
  assert.deepEqual(parsePendingUploads([pending, null, second, pending]), [
    pending,
    second,
  ]);
  const original = Object.getOwnPropertyDescriptor(globalThis, 'localStorage');
  const stored = new Map<string, string>();
  Object.defineProperty(globalThis, 'localStorage', {
    configurable: true,
    value: {
      getItem: (key: string) => stored.get(key) ?? null,
      setItem: (key: string, value: string) => {
        stored.set(key, value);
      },
      removeItem: (key: string) => {
        stored.delete(key);
      },
    },
  });
  try {
    rememberUpload(pending);
    rememberUpload(second);
    assert.deepEqual(savedUploads(), [pending, second]);
    forgetUpload(pending.id);
    assert.deepEqual(savedUploads(), [second]);
    forgetUpload(second.id);
    assert.deepEqual(savedUploads(), []);
  } finally {
    if (original !== undefined)
      Object.defineProperty(globalThis, 'localStorage', original);
    else Reflect.deleteProperty(globalThis, 'localStorage');
  }
});

void test('transformed uploads retain exact local bytes while malformed descriptors cannot resume originals', () => {
  const prepared = {
    id: 'e'.repeat(32),
    extension: 'mp4',
    size: 700,
    created_at: 123,
  };
  assert.deepEqual(parsePendingUpload({ ...pending, prepared }), {
    ...pending,
    prepared,
  });
  assert.deepEqual(parsePendingUpload(pending), pending);
  for (const value of [
    null,
    {},
    { ...prepared, id: '../unsafe' },
    { ...prepared, size: Infinity },
  ])
    assert.equal(parsePendingUpload({ ...pending, prepared: value }), null);
});
