// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

/** File-picker additions must preserve the original queue, limits and local File identities. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  appendUploadFiles,
  uploadKind,
} from '../src/lib/features/upload-selection.ts';
import type { UploadPolicy } from '../src/lib/api/client';

/** Small independent limits exercise category boundaries without allocating huge fixtures. */
const policy: UploadPolicy = {
  max_upload_bytes: 128,
  max_image_upload_bytes: 64,
  allowed_extensions: ['jpg', 'heic', 'webm'],
  image_extensions: ['jpg', 'heic'],
};

void test('later additions append in order and selecting the same file again does not duplicate it', () => {
  const first = new File([new Uint8Array(32)], 'first.jpg', {
    lastModified: 1,
  });
  const second = new File([new Uint8Array(64)], 'second.webm', {
    lastModified: 2,
  });
  const current = [first];
  const result = appendUploadFiles(current, [second, first], policy);
  assert.deepEqual(result, [first, second]);
  assert.equal(result[0], first);
  assert.deepEqual(current, [first]);
  assert.equal(uploadKind(first, policy), 'image');
  assert.equal(uploadKind(second, policy), 'video');
});

void test('invalid or oversized additions leave previously selected originals untouched', () => {
  const original = new File([new Uint8Array(32)], 'original.jpg');
  const current = [original];
  for (const [name, size] of [
    ['large.jpg', 65],
    ['large.webm', 129],
    ['unsafe.exe', 32],
  ] as const) {
    assert.throws(() =>
      appendUploadFiles(
        current,
        [new File([new Uint8Array(size)], name)],
        policy,
      ),
    );
    assert.deepEqual(current, [original]);
  }
  const raw = new File([new Uint8Array(64)], 'original.HEIC');
  assert.equal(uploadKind(raw, policy), 'image');
  assert.deepEqual(appendUploadFiles(current, [raw], policy), [original, raw]);
});
