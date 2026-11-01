/** Batch uploads retain their exact pending position after failure and cancellation. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { runUploadQueue } from '../src/lib/features/upload-queue.ts';

void test('a batch is sequential and resumes at the failed file', async () => {
  const files = ['first', 'second', 'third'].map(
    (name) => new File(['video'], name),
  );
  const transferred: string[] = [];
  let active = 0;
  await assert.rejects(
    runUploadQueue(
      files,
      async (file) => {
        assert.equal(++active, 1);
        await Promise.resolve();
        active--;
        if (file.name === 'second') throw new Error('Disconnected');
        transferred.push(file.name);
      },
      new AbortController().signal,
    ),
    /Disconnected/,
  );
  assert.deepEqual(
    files.map((file) => file.name),
    ['second', 'third'],
  );
  await runUploadQueue(
    files,
    (file) => {
      transferred.push(file.name);
      return Promise.resolve();
    },
    new AbortController().signal,
  );
  assert.deepEqual(transferred, ['first', 'second', 'third']);
  assert.equal(files.length, 0);
});

void test('pausing a batch does not start or discard the next file', async () => {
  const files = ['first', 'second'].map((name) => new File(['video'], name));
  const controller = new AbortController();
  await assert.rejects(
    runUploadQueue(
      files,
      () => {
        controller.abort();
        return Promise.resolve();
      },
      controller.signal,
    ),
    { name: 'AbortError' },
  );
  assert.deepEqual(
    files.map((file) => file.name),
    ['second'],
  );
});
