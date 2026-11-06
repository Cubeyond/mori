// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Local encoding is independently bounded, cancellable, and optional on every upload path. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  OptimizationBatch,
  discardPrepared,
} from '../src/lib/features/optimization/client.ts';
/** Small controllable worker double; no encoder or filesystem implementation is mirrored. */
class LocalWorker {
  /** Live worker identities are used only to observe concurrency and explicit cancellation. */
  static live = new Set<LocalWorker>();
  /** Total starts detect accidental re-encoding after a no-gain attempt. */
  static starts = 0;
  /** Browsers may reject worker construction under a restrictive environment. */
  static blocked = false;
  /** The browser dispatches a typed message event after the optional attempt completes. */
  onmessage: ((event: MessageEvent<unknown>) => void) | null = null;
  /** Kept for the same error boundary as a native Worker. */
  onerror: (() => void) | null = null;
  /** Starting a worker consumes one externally observable encoding slot. */
  constructor() {
    if (LocalWorker.blocked) throw new Error('Worker unavailable');
    LocalWorker.live.add(this);
    LocalWorker.starts++;
  }
  /** Tests explicitly complete work instead of waiting for real codec timings. */
  postMessage(): void {
    /* Completion is controlled explicitly by the test. */
  }
  /** A complete but unprofitable attempt returns the original. */
  finish(): void {
    this.onmessage?.(
      new MessageEvent('message', { data: { type: 'done', copy: null } }),
    );
  }
  /** Pauses, failures and successful completion all release the worker. */
  terminate(): void {
    LocalWorker.live.delete(this);
  }
}
/** Advance asynchronous queue boundaries without a timing-dependent sleep. */
async function settle(): Promise<void> {
  for (let n = 0; n < 8; n++) await Promise.resolve();
}
void test('preparation bounds image workers, serializes video, cancels waiters and caches unprofitable attempts', async () => {
  const globals = new Map(
    ['navigator', 'Worker'].map((key) => [
      key,
      Object.getOwnPropertyDescriptor(globalThis, key),
    ]),
  );
  Object.defineProperty(globalThis, 'navigator', {
    configurable: true,
    value: {
      hardwareConcurrency: 8,
      storage: {
        getDirectory: () => Promise.reject(new Error('No partial file')),
      },
    },
  });
  Object.defineProperty(globalThis, 'Worker', {
    configurable: true,
    value: LocalWorker,
  });
  /** Use distinct original identities as the real editable picker does. */
  const original = () => new File([new Uint8Array(1_100_000)], 'original.mp4');
  try {
    const batch = new OptimizationBatch('balanced', ['mp4', 'webp']);
    const first = original(),
      second = original(),
      third = original();
    const abort = new AbortController();
    const jobs = [
      batch.prepare(first, 'image', new AbortController().signal),
      batch.prepare(second, 'image', new AbortController().signal),
    ];
    const waiting = batch.prepare(third, 'image', abort.signal);
    const rejected = assert.rejects(waiting, { name: 'AbortError' });
    await settle();
    assert.equal(LocalWorker.live.size, 2);
    abort.abort();
    await rejected;
    for (const worker of [...LocalWorker.live]) worker.finish();
    const results = await Promise.all(jobs);
    assert.equal(results[0]?.file, first);
    assert.equal(results[1]?.file, second);
    const starts = LocalWorker.starts;
    assert.equal(
      (await batch.prepare(first, 'image', new AbortController().signal)).file,
      first,
    );
    assert.equal(LocalWorker.starts, starts);
    const controller = new AbortController();
    const videoA = batch.prepare(original(), 'video', controller.signal),
      videoB = batch.prepare(original(), 'video', controller.signal);
    const rejectedVideos = Promise.all([
      assert.rejects(videoA, { name: 'AbortError' }),
      assert.rejects(videoB, { name: 'AbortError' }),
    ]);
    await settle();
    assert.equal(LocalWorker.live.size, 1);
    controller.abort();
    await rejectedVideos;
    assert.equal(LocalWorker.live.size, 0);
    for (const mode of ['original', 'balanced'] as const) {
      const skip = new OptimizationBatch(mode, []);
      const file = original();
      assert.equal(
        (await skip.prepare(file, 'video', new AbortController().signal)).file,
        file,
      );
    }
    assert.equal(LocalWorker.live.size, 0);
    LocalWorker.blocked = true;
    const unsupported = original();
    assert.equal(
      (
        await new OptimizationBatch('balanced', ['mp4']).prepare(
          unsupported,
          'video',
          new AbortController().signal,
        )
      ).file,
      unsupported,
    );
    Object.defineProperty(globalThis, 'navigator', {
      configurable: true,
      value: { hardwareConcurrency: 4 },
    });
    const noStorage = original();
    assert.equal(
      (
        await new OptimizationBatch('balanced', ['mp4']).prepare(
          noStorage,
          'video',
          new AbortController().signal,
        )
      ).file,
      noStorage,
    );
  } finally {
    LocalWorker.blocked = false;
    await discardPrepared();
    for (const [key, descriptor] of globals) {
      if (descriptor === undefined) Reflect.deleteProperty(globalThis, key);
      else Object.defineProperty(globalThis, key, descriptor);
    }
  }
});
