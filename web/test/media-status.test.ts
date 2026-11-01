/** Operational deletion state must not masquerade as an administrator's privacy choice. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mediaStatus } from '../src/lib/features/media-status.ts';

void test('public and private deletions show their operation without a Private badge', () => {
  for (const kind of ['image', 'video'] as const) {
    for (const visibility of ['public', 'private'] as const) {
      const status = mediaStatus({ kind, visibility, state: 'deleting' });
      assert.equal(status.label, `Deleting ${kind}…`);
      assert.equal(status.visibility, '');
      assert.equal(status.ready, false);
    }
  }
});

void test('ready private media retains its real privacy label', () => {
  assert.deepEqual(
    mediaStatus({ kind: 'image', visibility: 'private', state: 'ready' }),
    { ready: true, label: '', visibility: 'Private' },
  );
  assert.equal(
    mediaStatus({ kind: 'video', visibility: 'public', state: 'published' })
      .visibility,
    '',
  );
});

void test('image processing and failed uploads retain accurate status labels', () => {
  assert.equal(
    mediaStatus({ kind: 'image', visibility: 'private', state: 'verifying' })
      .label,
    'Processing image…',
  );
  assert.equal(
    mediaStatus({ kind: 'video', visibility: 'private', state: 'failed' })
      .label,
    'Upload failed',
  );
});
