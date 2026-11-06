// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Local decode concurrency is independent of queue size and cancelled preview requests. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { previewSlot } from '../src/lib/features/preview-slots.ts';

void test('two decoders run at most, cancelled waiters leave no reservation, and releases are idempotent', async () => {
  const first = await previewSlot(new AbortController().signal);
  const second = await previewSlot(new AbortController().signal);
  const cancelled = new AbortController();
  const waiting = previewSlot(cancelled.signal);
  let acquired = false;
  const next = previewSlot(new AbortController().signal).then((release) => {
    acquired = true;
    return release;
  });
  await Promise.resolve();
  assert.equal(acquired, false);
  cancelled.abort();
  await assert.rejects(waiting, { name: 'AbortError' });
  first();
  first();
  const release = await next;
  assert.equal(acquired, true);
  second();
  release();
});
