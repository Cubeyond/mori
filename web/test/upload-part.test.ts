// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

/** Transport tests drive actual progress, failure, and cancellation callbacks without networking. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { putUploadPart } from '../src/lib/features/upload-part.ts';

/** Minimal browser protocol fixture records requests without replacing the transfer logic. */
class RequestFixture {
  static requests: RequestFixture[] = [];
  status = 200;
  aborted = false;
  headers = new Map<string, string>();
  method = '';
  url = '';
  body: Blob | null = null;
  upload: { onprogress: ((event: { loaded: number }) => void) | null } = {
    onprogress: null,
  };
  onload: (() => void) | null = null;
  onerror: (() => void) | null = null;
  onabort: (() => void) | null = null;
  /** Retain each instance so tests can independently complete requests. */
  constructor() {
    RequestFixture.requests.push(this);
  }
  /** Record the exact direct-upload destination and HTTP method. */
  open(method: string, url: string): void {
    this.method = method;
    this.url = url;
  }
  /** Record the integrity header required by the presigned request. */
  setRequestHeader(name: string, value: string): void {
    this.headers.set(name, value);
  }
  /** Keep the bounded Blob rather than constructing an additional full-file buffer. */
  send(body: Blob): void {
    this.body = body;
  }
  /** Model the synchronous cancellation event emitted by browser XHR. */
  abort(): void {
    this.aborted = true;
    this.onabort?.();
  }
}

void test('S3 progress arrives before completion and abort listeners are released', async (t) => {
  const original = Object.getOwnPropertyDescriptor(
    globalThis,
    'XMLHttpRequest',
  );
  Object.defineProperty(globalThis, 'XMLHttpRequest', {
    configurable: true,
    value: RequestFixture,
  });
  t.after(() => {
    if (original === undefined)
      Reflect.deleteProperty(globalThis, 'XMLHttpRequest');
    else Object.defineProperty(globalThis, 'XMLHttpRequest', original);
  });
  const controller = new AbortController();
  const progress: number[] = [];
  const blob = new Blob(['video']);
  const transfer = putUploadPart(
    'https://s3.example/part',
    blob,
    'checksum',
    controller.signal,
    (bytes) => progress.push(bytes),
  );
  const request = RequestFixture.requests.at(-1);
  assert.ok(request);
  assert.equal(request.method, 'PUT');
  assert.equal(request.body, blob);
  assert.equal(request.headers.get('x-amz-checksum-sha256'), 'checksum');
  request.upload.onprogress?.({ loaded: 2 });
  assert.deepEqual(progress, [2]);
  request.onload?.();
  await transfer;
  controller.abort();
  assert.equal(request.aborted, false);
  assert.equal(request.upload.onprogress, null);
});

void test('HTTP failure and pause reject the transfer instead of reporting success', async (t) => {
  const original = Object.getOwnPropertyDescriptor(
    globalThis,
    'XMLHttpRequest',
  );
  Object.defineProperty(globalThis, 'XMLHttpRequest', {
    configurable: true,
    value: RequestFixture,
  });
  t.after(() => {
    if (original === undefined)
      Reflect.deleteProperty(globalThis, 'XMLHttpRequest');
    else Object.defineProperty(globalThis, 'XMLHttpRequest', original);
  });
  for (const cancel of [false, true]) {
    const controller = new AbortController();
    const transfer = putUploadPart(
      'https://s3.example/part',
      new Blob(['video']),
      'checksum',
      controller.signal,
      () => {
        /* Progress is irrelevant to the failure contract. */
      },
    );
    const request = RequestFixture.requests.at(-1);
    assert.ok(request);
    if (cancel) controller.abort();
    else {
      request.status = 503;
      request.onload?.();
    }
    await assert.rejects(transfer, cancel ? { name: 'AbortError' } : /503/);
    assert.equal(request.aborted, cancel);
  }
});
