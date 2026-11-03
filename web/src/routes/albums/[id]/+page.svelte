<script lang="ts">
  import { pageTitle } from '#lib/features/page-title';
  import Button from '#lib/components/ui/button/button.svelte';
  /** Authenticated album detail includes private videos without changing the public share view. */
  import { onMount, untrack } from 'svelte';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { Pencil, Plus, SquareDashedMousePointer } from '@lucide/svelte';
  import {
    api,
    apiData,
    session,
    message,
    type Album,
    type Video,
  } from '#lib/api/client';
  import AccessGate from '#lib/components/AccessGate.svelte';
  import Breadcrumbs from '#lib/components/Breadcrumbs.svelte';
  import CollectionToolbar from '#lib/components/library/CollectionToolbar.svelte';
  import MediaSelection from '#lib/components/library/MediaSelection.svelte';
  import AlbumEditor from '#lib/components/library/AlbumEditor.svelte';
  import MediaDialog from '#lib/components/library/MediaDialog.svelte';
  import { takeInitialPage } from '#lib/api/bootstrap';
  import { openUploader } from '#lib/features/upload';
  /** The document already contains fresh metadata for this exact initial route. */
  const initialPage = untrack(() => takeInitialPage(page.url.pathname));
  /** Only this initial route may consume its collection data. */
  const initial = initialPage?.album_page ?? null;
  /** Active collection, member metadata, and album choices for video editing. */
  let album = $state<Album | null>(initial?.album ?? null),
    albums = $state<Album[]>(initialPage?.albums ?? []),
    videos = $state<Video[]>(initial?.videos ?? []);
  /** Search and exact counts retain the album heading during requests. */
  let query = $state(''),
    total = $state(initial?.total ?? 0);
  /** A replacement search aborts its predecessor without restarting the route. */
  let searched = '',
    request: AbortController | undefined;
  /** Use the seed only once; later route changes fetch their own current metadata. */
  let seeded = false;
  /** This album uses the shared batch selection surface. */
  let selecting = $state(false);
  /** Background refreshes never replace an already rendered collection with loading text. */
  let loaded = $state(initial !== null);
  /** Bounded pagination, loading, and feedback remain independent of floating editors. */
  let more = $state(initial?.has_more ?? false),
    busy = $state(false),
    error = $state(''),
    notice = $state('');
  /** At most one metadata editor is active. */
  let editingAlbum = $state(false),
    editingVideo = $state<Video | null>(null);
  /** Equivalent session responses must not restart the collection route lifecycle. */
  const isAuthenticated = $derived($session.authenticated);
  /** Ignore responses from a previous album route. */
  let lifecycle: AbortController | undefined;
  /** Refresh the loaded metadata depth or append one bounded page. */
  async function refresh(append = false, reset = false): Promise<void> {
    const id = page.params.id,
      routeSignal = lifecycle?.signal;
    if (
      id === undefined ||
      routeSignal === undefined ||
      routeSignal.aborted ||
      (busy && !reset)
    )
      return;
    request?.abort();
    request = new AbortController();
    const signal = AbortSignal.any([routeSignal, request.signal]);
    const q = query.trim();
    searched = q;
    /** Read abort state at each asynchronous boundary without retaining stale narrowing. */
    const active = () => !signal.aborted;
    busy = true;
    error = '';
    try {
      const rows: Video[] = [];
      const target = append || reset ? 30 : Math.max(30, videos.length);
      let offset = append ? videos.length : 0,
        hasMore = false;
      do {
        const result = await apiData(
          api.GET('/v1/admin/albums/{id}', {
            params: {
              path: { id },
              query: { offset, limit: Math.min(60, target - rows.length), q },
            },
            signal,
          }),
        );
        if (!active()) return;
        album = result.album;
        total = result.total;
        rows.push(...result.videos);
        offset += result.videos.length;
        hasMore = result.has_more;
      } while (hasMore && rows.length < target);
      if (!active()) return;
      videos = append ? [...videos, ...rows] : rows;
      more = hasMore;
      loaded = true;
      albums = await apiData(api.GET('/v1/admin/albums', { signal }));
    } catch (failure) {
      if (active()) error = message(failure);
    } finally {
      if (active()) busy = false;
    }
  }
  /** Refresh in response to a completed application-wide upload. */
  function changed(): void {
    void refresh();
  }
  $effect(() => {
    const id = page.params.id,
      authenticated = isAuthenticated;
    const controller = new AbortController();
    lifecycle = controller;
    untrack(() => {
      if (!seeded && initial !== null && initial.album.id === id) {
        seeded = true;
        return;
      }
      query = '';
      searched = '';
      total = 0;
      selecting = false;
      videos = [];
      album = null;
      loaded = false;
      error = '';
      busy = false;
      if (authenticated && id !== undefined) void refresh();
    });
    return () => {
      controller.abort();
      request?.abort();
    };
  });
  $effect(() => {
    const q = query.trim();
    if (!loaded || !isAuthenticated || q === untrack(() => searched)) return;
    const timer = setTimeout(() => {
      void refresh(false, true);
    }, 180);
    return () => {
      clearTimeout(timer);
    };
  });
  onMount(() => {
    window.addEventListener('mori:library-changed', changed);
    const timer = setInterval(() => {
      if ($session.authenticated && !editingAlbum && editingVideo === null)
        void refresh();
    }, 10000);
    return () => {
      window.removeEventListener('mori:library-changed', changed);
      clearInterval(timer);
    };
  });
</script>

<svelte:head><title>{pageTitle(album?.name)}</title></svelte:head>
<main>
  <Breadcrumbs
    items={[
      { label: 'Albums', href: resolve('/albums') },
      {
        label: album?.name ?? '',
        href: resolve('/albums/[id]', { id: page.params.id ?? '' }),
      },
    ]}
  />
  <AccessGate>
    <div class="page-heading album-heading">
      <div>
        <h1>
          {#if album !== null && album.visibility === 'public'}<a
              class="album-title-link"
              href={resolve('/[slug]', { slug: album.slug })}
              target="_blank"
              rel="noopener noreferrer"
              aria-label={album.name + ' (opens public album in a new tab)'}
              >{album.name}</a
            >
          {:else}{album?.name ?? ''}{/if}
        </h1>
      </div>
      {#if album !== null}<div
          class="album-actions flex flex-wrap justify-end gap-2"
        >
          <Button
            variant="outline"
            type="button"
            aria-pressed={selecting}
            disabled={videos.length === 0}
            onclick={() => {
              selecting = !selecting;
            }}
            ><SquareDashedMousePointer size={17} />{selecting
              ? 'Done'
              : 'Select'}</Button
          >
          <Button
            variant="outline"
            type="submit"
            class="button"
            onclick={() => (editingAlbum = true)}
            ><Pencil size={15} />Edit album</Button
          >
          <Button
            variant="default"
            type="button"
            class="button primary"
            onclick={() => {
              openUploader(album?.id ?? null);
            }}><Plus size={16} />Upload media</Button
          >
        </div>{/if}
    </div>
    {#if error}<p class="error mb-4" role="alert">{error}</p>{/if}
    {#if notice}<p class="muted mb-4 text-sm" role="status">{notice}</p>{/if}
    {#if album !== null}<CollectionToolbar bind:query count={total} />{/if}
    {#if videos.length > 0}<MediaSelection
        {videos}
        {albums}
        bind:selecting
        oncomplete={refresh}
        onmanage={(video: Video) => (editingVideo = video)}
      />{:else if busy && !loaded}<p class="empty-state muted" role="status">
        Loading album…
      </p>{:else if query.trim()}<div class="empty-state">
        <h2>No matching media</h2>
        <p>Try another name.</p>
      </div>{:else if album !== null}<div class="empty-state">
        <h2>This album is empty</h2>
        <p>Upload an image or video to get started.</p>
      </div>{/if}
    {#if more}<div class="py-6 text-center">
        <Button
          variant="outline"
          type="submit"
          class="button"
          disabled={busy}
          onclick={() => refresh(true)}>Load more</Button
        >
      </div>{/if}
  </AccessGate>
</main>
{#if editingAlbum && album !== null}<AlbumEditor
    {album}
    onclose={() => (editingAlbum = false)}
    oncomplete={refresh}
  />{/if}
{#if editingVideo !== null}<MediaDialog
    video={editingVideo}
    {albums}
    onclose={() => (editingVideo = null)}
    oncomplete={refresh}
    onnotice={(value: string) => (notice = value)}
    onupdate={(video: Video) => (editingVideo = video)}
  />{/if}

<style>
  .album-title-link:hover {
    text-decoration: underline;
    text-decoration-thickness: 1px;
    text-underline-offset: 6px;
    text-decoration-color: #ffffff60;
  }
  @media (max-width: 640px) {
    .album-heading {
      align-items: flex-start;
      flex-direction: column;
    }
    .album-actions {
      justify-content: flex-start;
    }
  }
</style>
