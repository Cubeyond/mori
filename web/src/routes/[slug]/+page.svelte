<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: Elastic-2.0 -->

<script lang="ts">
  import { pageTitle } from '#lib/features/page-title';
  import Button from '#lib/components/ui/button/button.svelte';
  /** Minimal public collection; the API authorizes album and member visibility independently. */
  import { resolve } from '$app/paths';
  import { onMount, untrack } from 'svelte';
  import { page } from '$app/state';
  import { takeInitialPage } from '#lib/api/bootstrap';
  import {
    api,
    apiData,
    session,
    type MediaItem,
    type Album,
  } from '#lib/api/client';
  import { CollectionSearch } from '#lib/features/collection-search';
  import CollectionToolbar from '#lib/components/library/CollectionToolbar.svelte';
  import Gallery from '#lib/components/Gallery.svelte';
  /** Public collection data is visibility-filtered by Rust before the first render. */
  const initial = untrack(
    () => takeInitialPage(page.url.pathname)?.album_page ?? null,
  );
  /** This instance contains only metadata authorized for the active public route/session. */
  const search = new CollectionSearch((offset, limit, q, signal) => {
    const slug = page.params.slug ?? '';
    return apiData(
      api.GET('/v1/albums/{slug}', {
        params: { path: { slug }, query: { offset, limit, q } },
        signal,
      }),
    );
  });
  /** Seed authorization must match the current session before it can be reused. */
  const initialAuthenticated = untrack(() => $session.authenticated);
  /** Re-entering a collection later must use a fresh request. */
  let seeded = false;
  /** Publicly readable metadata only, cleared on route changes or failed access checks. */
  let videos = $state<MediaItem[]>(initial?.videos ?? []),
    album = $state<Album | null>(initial?.album ?? null);
  /** Pagination and request state never expose a private album's identity. */
  let more = $state(initial?.has_more ?? false),
    busy = $state(initial === null),
    failed = $state(false);
  /** Search and counts are scoped by the same server authorization as media. */
  let query = $state(''),
    total = $state(initial?.total ?? 0);
  let searched = '',
    request: AbortController | undefined;
  /** Each route owns its request lifecycle, including subsequent pagination. */
  let controller: AbortController | undefined;
  /** Fetch public members while ignoring obsolete or aborted responses. */
  async function load(
    routeSignal: AbortSignal,
    append = false,
    authoritative = false,
  ): Promise<void> {
    request?.abort();
    request = new AbortController();
    const signal = AbortSignal.any([routeSignal, request.signal]),
      q = query.trim();
    searched = q;
    busy = true;
    try {
      const data = await search.read(
        q,
        append ? videos.length : 0,
        30,
        signal,
        authoritative,
      );
      if (signal.aborted) return;
      if (data.album !== undefined) album = data.album;
      videos = append ? [...videos, ...data.videos] : data.videos;
      total = data.total;
      more = data.has_more;
    } catch {
      if (!signal.aborted) {
        failed = true;
        album = null;
        videos = [];
      }
    } finally {
      if (!signal.aborted) busy = false;
    }
  }
  /** Continue only the active collection's bounded result set. */
  function next(): void {
    if (!busy && controller !== undefined) void load(controller.signal, true);
  }
  $effect(() => {
    const slug = page.params.slug;
    // Session changes must discard owner-only metadata, including after logout.
    const authenticated = $session.authenticated;
    const lifecycle = new AbortController();
    controller = lifecycle;
    untrack(() => {
      if (
        !seeded &&
        initial !== null &&
        initial.album.slug === slug &&
        authenticated === initialAuthenticated
      ) {
        seeded = true;
        search.seed(initial);
        return;
      }
      query = '';
      searched = '';
      total = 0;
      videos = [];
      album = null;
      failed = false;
      more = false;
      if (slug !== undefined) void load(lifecycle.signal);
    });
    return () => {
      lifecycle.abort();
      request?.abort();
      search.clear();
    };
  });
  $effect(() => {
    const q = query.trim();
    if (album === null || q === untrack(() => searched)) return;
    if (untrack(() => search.complete || search.cached(q) !== null)) {
      untrack(() => {
        if (controller !== undefined) void load(controller.signal);
      });
      return;
    }
    const timer = setTimeout(() => {
      if (controller !== undefined) void load(controller.signal);
    }, 80);
    return () => {
      clearTimeout(timer);
    };
  });
  onMount(() => {
    const timer = setInterval(() => {
      if (
        !busy &&
        !document.hidden &&
        controller !== undefined &&
        album !== null
      )
        void load(controller.signal, false, true);
    }, 15000);
    return () => {
      clearInterval(timer);
    };
  });
</script>

<svelte:head><title>{pageTitle(album?.name)}</title></svelte:head>
<main class="public-album">
  {#if failed}<div class="empty-state">
      <h1>Album unavailable</h1>
      <p>This album is private, removed, or does not exist.</p>
      <Button variant="outline" class="button" href={resolve('/')}
        >Return home</Button
      >
    </div>
  {:else if album !== null}<header class="album-heading">
      <div>
        <p class="eyebrow">Shared album</p>
        <h1>{album.name}</h1>
      </div>
    </header>
    <CollectionToolbar bind:query count={total} />
    {#if videos.length > 0}<Gallery {videos} />{:else if !busy}<div
        class="empty-state"
      >
        <h2>{query.trim() ? 'No matching media' : 'No media yet'}</h2>
        <p>
          {query.trim()
            ? 'Try another name.'
            : 'Shared images and videos will appear here.'}
        </p>
      </div>{/if}
    {#if busy}<p class="muted py-6 text-center text-sm" role="status">
        Loading media…
      </p>{:else if more}<div class="py-6 text-center">
        <Button variant="outline" type="submit" class="button" onclick={next}
          >Load more</Button
        >
      </div>{/if}
  {:else}<p class="empty-state muted" role="status">Opening album…</p>{/if}
</main>

<style>
  .album-heading {
    margin: 12px 0 32px;
  }
  .eyebrow {
    font-size: 14px;
    color: var(--muted-foreground);
    margin-bottom: 10px;
    letter-spacing: 0.05em;
  }
  h1 {
    font-family: 'Mori Serif', Georgia, serif;
    font-size: clamp(36px, 4vw, 48px);
    font-weight: 400;
    line-height: 1.1;
    letter-spacing: -0.025em;
    overflow-wrap: anywhere;
  }
</style>
