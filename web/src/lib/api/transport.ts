// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** OpenAPI transport owns HTTP policy; callers inherit paths, payloads, and response types. */
import createClient, { type Client } from 'openapi-fetch';
import type { paths } from './generated/schema';

/** The successful or failed result produced by the generated client. */
type ApiResult<T> =
  | { data: T; error?: never; response: Response }
  | { data?: never; error: unknown; response: Response };

/** Create an isolated client with injectable browser transport and current CSRF state. */
export function createApi(
  origin: string,
  csrf: () => string | null,
  fetcher: typeof fetch = fetch,
): Client<paths> {
  const client = createClient<paths>({
    baseUrl: origin,
    credentials: 'same-origin',
    fetch: fetcher,
  });
  client.use({
    /** Apply CSRF to Mori mutations while keeping GET requests free of mutation credentials. */
    onRequest({ request }): Request {
      if (!['GET', 'HEAD'].includes(request.method)) {
        const token = csrf();
        if (token !== null) request.headers.set('x-csrf-token', token);
      }
      return request;
    },
    /** Normalize proxy failures so an HTML gateway response does not become a JSON parse error. */
    async onResponse({ response }): Promise<Response | undefined> {
      if (
        !response.ok &&
        !(response.headers.get('content-type') ?? '').includes('json')
      ) {
        await response.body?.cancel();
        return Response.json(
          { error: `Request failed (${String(response.status)})` },
          { status: response.status },
        );
      }
      return undefined;
    },
  });
  return client;
}

/** Decode one generated result without caller-authored assertions or duplicate response types. */
export async function apiData<T>(pending: Promise<ApiResult<T>>): Promise<T> {
  const result = await pending;
  if ('error' in result) {
    const error = result.error;
    const message =
      error !== null && typeof error === 'object' && 'error' in error
        ? String(error.error)
        : `Request failed (${String(result.response.status)})`;
    throw new Error(message);
  }
  return result.data;
}
