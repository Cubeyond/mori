<script lang="ts">
  import { pageTitle } from '#lib/features/page-title';
  import Button from '#lib/components/ui/button/button.svelte';
  /** Minimal public collection; the API authorizes album and member visibility independently. */
  import { resolve } from '$app/paths';
  import { untrack } from 'svelte';
  import { page } from '$app/state';
  import { takeInitialPage } from '#lib/api/bootstrap';
  import {
    api,
    apiData,
    session,
    type Video,
    type Album,
  } from '#lib/api/client';
  import Gallery from '#lib/components/Gallery.svelte';
  /** Public collection data is visibility-filtered by Rust before the first render. */
  const initial = untrack(
    () => takeInitialPage(page.url.pathname)?.album_page ?? null,
  );
  /** Seed authorization must match the current session before it can be reused. */
  const initialAuthenticated = untrack(() => $session.authenticated);
  /** Re-entering a collection later must use a fresh request. */
  let seeded = false;
  /** Publicly readable metadata only, cleared on route changes or failed access checks. */
  let videos = $state<Video[]>(initial?.videos ?? []),
    album = $state<Album | null>(initial?.album ?? null);
  /** Pagination and request state never expose a private album's identity. */
  let more = $state(initial?.has_more ?? false),
    busy = $state(initial === null),
    failed = $state(false);
  /** Each route owns its request lifecycle, including subsequent pagination. */
  let controller: AbortController | undefined;
  /** Fetch public members while ignoring obsolete or aborted responses. */
  async function load(slug: string, signal: AbortSignal): Promise<void> {
    busy = true;
    try {
      const data = await apiData(
        api.GET('/v1/albums/{slug}', {
          params: { path: { slug }, query: { offset: videos.length } },
          signal,
        }),
      );
      if (signal.aborted) return;
      album = data.album;
      videos = [...videos, ...data.videos];
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
    if (!busy && controller !== undefined)
      void load(page.params.slug ?? '', controller.signal);
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
        return;
      }
      videos = [];
      album = null;
      failed = false;
      more = false;
      if (slug !== undefined) void load(slug, lifecycle.signal);
    });
    return () => {
      lifecycle.abort();
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
    {#if videos.length > 0}<Gallery {videos} />{:else if !busy}<div
        class="empty-state"
      >
        <h2>No public media yet</h2>
        <p>Shared images and videos will appear here.</p>
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
    color: #8e8e9a;
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
