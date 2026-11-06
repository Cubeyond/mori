// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

/** Metadata readiness never waits for cover encoding or unsupported-codec timeout. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { prepareUploadPreview } from '../src/lib/features/upload-preview.ts';

void test('metadata resolves before a delayed JPEG, with a nearby seek and decoder cleanup', async (t) => {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, 'document');
  let encode: ((poster: Blob | null) => void) | undefined;
  let released = false;
  const handlers: Record<
    'onloadedmetadata' | 'onloadeddata' | 'onseeked' | 'onerror',
    (() => void) | null
  > = {
    onloadedmetadata: null,
    onloadeddata: null,
    onseeked: null,
    onerror: null,
  };
  const video = {
    ...handlers,
    src: '',
    muted: false,
    preload: '',
    currentTime: 0,
    readyState: 2,
    videoWidth: 1920,
    videoHeight: 1080,
    duration: 60,
    pause: () => {
      /* No decoder runs in this fixture. */
    },
    load: () => {
      /* Local metadata is emitted explicitly. */
    },
    removeAttribute: () => {
      released = true;
    },
  };
  const canvas = {
    width: 0,
    height: 0,
    getContext: () => ({
      drawImage: () => {
        /* Pixel encoding is completed independently below. */
      },
    }),
    toBlob: (callback: (poster: Blob | null) => void) => {
      encode = callback;
    },
  };
  Object.defineProperty(globalThis, 'document', {
    configurable: true,
    value: {
      createElement: (tag: string) => (tag === 'video' ? video : canvas),
    },
  });
  t.after(() => {
    if (descriptor === undefined)
      Reflect.deleteProperty(globalThis, 'document');
    else Object.defineProperty(globalThis, 'document', descriptor);
  });
  const controller = new AbortController();
  const preview = prepareUploadPreview(
    new File(['video'], 'sample.webm'),
    controller.signal,
  );
  t.after(preview.dispose);
  video.onloadedmetadata?.();
  assert.deepEqual(await preview.metadata, {
    width: 1920,
    height: 1080,
    duration: 60,
  });
  assert.equal(video.currentTime, 0.1);
  let covered = false;
  void preview.poster.then(() => {
    covered = true;
  });
  await Promise.resolve();
  assert.equal(covered, false);
  video.onloadeddata?.();
  assert.ok(encode);
  const poster = new Blob(['jpeg'], { type: 'image/jpeg' });
  encode(poster);
  assert.equal(await preview.poster, poster);
  assert.equal(released, true);
  assert.equal(canvas.width, 1280);
});
