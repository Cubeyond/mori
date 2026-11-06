// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

/** Generated transport contracts cover request policy and observable error behavior. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { apiData, createApi } from '../src/lib/api/transport.ts';

void test('generated operations encode paths, serialize JSON, and read current CSRF state', async () => {
  const requests: Request[] = [];
  let csrf = 'first-token';
  const api = createApi(
    'https://mori.test',
    () => csrf,
    (input) => {
      assert.ok(input instanceof Request);
      requests.push(input);
      return Promise.resolve(Response.json({ job_id: null }));
    },
  );
  const result = await apiData(
    api.PATCH('/v1/admin/videos/{id}', {
      params: { path: { id: 'with/slash and space' } },
      body: { name: 'Renamed', visibility: 'private', album_id: null },
    }),
  );
  assert.deepEqual(result, { job_id: null });
  const mutation = requests[0];
  assert.ok(mutation);
  assert.equal(
    mutation.url,
    'https://mori.test/v1/admin/videos/with%2Fslash%20and%20space',
  );
  assert.equal(mutation.credentials, 'same-origin');
  assert.equal(mutation.headers.get('x-csrf-token'), 'first-token');
  assert.deepEqual(await mutation.json(), {
    name: 'Renamed',
    visibility: 'private',
    album_id: null,
  });
  csrf = 'second-token';
  await apiData(api.POST('/v1/auth/logout'));
  assert.equal(requests[1]?.headers.get('x-csrf-token'), 'second-token');
  await apiData(api.GET('/v1/admin/albums'));
  assert.equal(requests[2]?.headers.has('x-csrf-token'), false);
});

void test('binary poster operations preserve bytes and content type', async () => {
  const poster = new Blob([new Uint8Array([255, 216, 255, 217])], {
    type: 'image/jpeg',
  });
  const api = createApi(
    'https://mori.test',
    () => null,
    async (input) => {
      assert.ok(input instanceof Request);
      assert.equal(input.headers.get('content-type'), 'image/jpeg');
      assert.deepEqual(
        new Uint8Array(await input.arrayBuffer()),
        new Uint8Array(await poster.arrayBuffer()),
      );
      return new Response(null, { status: 204 });
    },
  );
  await apiData(
    api.PUT('/v1/admin/videos/{id}/poster', {
      params: { path: { id: 'video' } },
      headers: { 'content-type': 'image/jpeg' },
      body: poster,
      bodySerializer: (body) => body,
    }),
  );
});

void test('JSON application errors and non-JSON gateway errors remain actionable', async () => {
  for (const [response, expected] of [
    [
      Response.json({ error: 'Authentication required' }, { status: 401 }),
      /Authentication required/,
    ],
    [
      new Response('<html>Bad gateway</html>', {
        status: 502,
        headers: { 'content-type': 'text/html' },
      }),
      /Request failed \(502\)/,
    ],
  ] as const) {
    const api = createApi(
      'https://mori.test',
      () => null,
      () => Promise.resolve(response),
    );
    await assert.rejects(apiData(api.GET('/v1/admin/albums')), expected);
  }
});
