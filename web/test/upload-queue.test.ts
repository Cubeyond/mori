// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Concurrent queue invariants cover live edits, resource bounds, cancellation and partial success. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { runUploadQueue } from '../src/lib/features/upload-queue.ts';

void test('two workers upload concurrently, accept waiting edits and never duplicate an original', async () => {
  const [first, second, removed, added] = [
    'first',
    'second',
    'removed',
    'added',
  ].map((name) => new File(['media'], name));
  assert.ok(first && second && removed && added);
  let files = [first, second, removed];
  const started = Promise.withResolvers<undefined>();
  const finish = Promise.withResolvers<undefined>();
  const transferred: string[] = [];
  let active = 0,
    peak = 0;
  const pending = runUploadQueue(
    () => files,
    async (file) => {
      peak = Math.max(peak, ++active);
      transferred.push(file.name);
      if (active === 2) started.resolve(undefined);
      await finish.promise;
      active--;
    },
    new AbortController().signal,
  );
  await started.promise;
  assert.equal(active, 2);
  files = [first, second, added];
  finish.resolve(undefined);
  assert.equal(await pending, 3);
  assert.equal(peak, 2);
  assert.deepEqual(transferred, ['first', 'second', 'added']);
  assert.deepEqual(files, []);
});

void test('failure aborts and settles the sibling before returning, preserving both resumable originals', async () => {
  const files = ['failed', 'interrupted', 'waiting'].map(
    (name) => new File(['media'], name),
  );
  const sibling = Promise.withResolvers<undefined>();
  let settled = false;
  const started: string[] = [];
  await assert.rejects(
    runUploadQueue(
      () => files,
      async (file, signal) => {
        started.push(file.name);
        if (file.name === 'failed') {
          await sibling.promise;
          throw new Error('Disconnected');
        }
        sibling.resolve(undefined);
        await new Promise<void>((resolve) => {
          signal.addEventListener(
            'abort',
            () => {
              resolve();
            },
            { once: true },
          );
        });
        await Promise.resolve(undefined);
        settled = true;
        signal.throwIfAborted();
      },
      new AbortController().signal,
    ),
    /Disconnected/,
  );
  assert.equal(settled, true);
  assert.deepEqual(started, ['failed', 'interrupted']);
  assert.equal(files.length, 3);
  assert.equal(
    await runUploadQueue(
      () => files,
      () => Promise.resolve(),
      new AbortController().signal,
    ),
    3,
  );
  assert.deepEqual(files, []);
});

void test('pausing retains both active originals and never starts waiting files', async () => {
  const files = ['first', 'second', 'third'].map(
    (name) => new File(['media'], name),
  );
  const controller = new AbortController();
  const ready = Promise.withResolvers<undefined>();
  let active = 0;
  const pending = runUploadQueue(
    () => files,
    async (_, signal) => {
      if (++active === 2) ready.resolve(undefined);
      await new Promise<void>((resolve) => {
        signal.addEventListener(
          'abort',
          () => {
            resolve();
          },
          { once: true },
        );
      });
      signal.throwIfAborted();
    },
    controller.signal,
  );
  await ready.promise;
  controller.abort();
  await assert.rejects(pending, { name: 'AbortError' });
  assert.equal(active, 2);
  assert.equal(files.length, 3);
});

void test('a successful sibling is removed even when another transfer fails', async () => {
  const files = ['success', 'failure'].map((name) => new File(['media'], name));
  const completed = Promise.withResolvers<undefined>();
  await assert.rejects(
    runUploadQueue(
      () => files,
      async (file) => {
        if (file.name === 'success') {
          completed.resolve(undefined);
          return;
        }
        await completed.promise;
        await Promise.resolve(undefined);
        throw new Error('Disconnected');
      },
      new AbortController().signal,
    ),
    /Disconnected/,
  );
  assert.deepEqual(
    files.map((file) => file.name),
    ['failure'],
  );
});

void test('retained thumbnails survive partial completion and retries claim only waiting originals', async () => {
  const files = ['uploaded', 'retry'].map((name) => new File(['media'], name));
  const completed = new Set<File>();
  const firstDone = Promise.withResolvers<undefined>();
  /** Retain original selection order independently from mutable transfer eligibility. */
  const waiting = () => files.filter((file) => !completed.has(file));
  /** Completion marks a thumbnail without removing it from the selected display list. */
  const finish = (file: File): void => {
    completed.add(file);
    firstDone.resolve(undefined);
  };
  await assert.rejects(
    runUploadQueue(
      waiting,
      async (file) => {
        if (file.name === 'uploaded') return;
        await firstDone.promise;
        throw new Error('Disconnected');
      },
      new AbortController().signal,
      finish,
    ),
    /Disconnected/,
  );
  assert.deepEqual(
    files.map((file) => file.name),
    ['uploaded', 'retry'],
  );
  assert.deepEqual(
    waiting().map((file) => file.name),
    ['retry'],
  );
  const retried: string[] = [];
  assert.equal(
    await runUploadQueue(
      waiting,
      (file) => {
        retried.push(file.name);
        return Promise.resolve();
      },
      new AbortController().signal,
      finish,
    ),
    1,
  );
  assert.deepEqual(retried, ['retry']);
  assert.deepEqual(waiting(), []);
  assert.equal(files.length, 2);
});
