<script lang="ts">
  import { pageTitle } from '#lib/features/page-title';
  import Button from '#lib/components/ui/button/button.svelte';
  import CollectionToolbar from './CollectionToolbar.svelte';
  import Breadcrumbs from '#lib/components/Breadcrumbs.svelte';
  /** Authenticated gallery orchestration; administration never enters its document flow. */
  import { resolve } from '$app/paths';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { onMount, untrack } from 'svelte';
  import {
    X,
    ArrowUpRight,
    Plus,
    SquareDashedMousePointer,
  } from '@lucide/svelte';
  import {
    api,
    apiData,
    message,
    loadSession,
    session,
    type MediaItem,
    type Album,
  } from '#lib/api/client';
  import MediaSelection from './MediaSelection.svelte';
  import { openUploader } from '#lib/features/upload';
  import MediaDialog from '#lib/components/library/MediaDialog.svelte';

  /** Search applies to the entire authorized collection, not just loaded pages. */
  let query = $state('');
  /** Selection mode leaves individual playback and sharing intact when dismissed. */
  let selecting = $state(false);
  /** Metadata only; the gallery loads no video source before playback intent. */
  let videos = $state<MediaItem[]>([]),
    albums = $state<Album[]>([]);
  /** Exact server count after the current search filter. */
  let total = $state(0);
  /** The last issued query and cancellation token reject obsolete responses. */
  let searched = '',
    request: AbortController | undefined;
  /** Session discovery, pagination, and scoped floating feedback. */
  let loaded = $state(false),
    more = $state(false),
    error = $state(''),
    notice = $state('');
  /** A single top-layer operation can be active while the gallery remains mounted. */
  let editing = $state<MediaItem | null>(null);
  /** Avoid overlapping polling and user-triggered refresh responses. */
  let refreshing = false;

  /** Refresh the visible metadata window, retaining already loaded pagination depth. */
  async function refresh(append = false, reset = false): Promise<void> {
    if (refreshing && !reset) return;
    request?.abort();
    const controller = new AbortController(),
      signal = controller.signal;
    request = controller;
    const q = query.trim();
    searched = q;
    refreshing = true;
    try {
      const target = append || reset ? 30 : Math.max(30, videos.length);
      const rows: MediaItem[] = [];
      let offset = append ? videos.length : 0;
      let hasMore = false;
      do {
        const limit = Math.min(60, target - rows.length);
        const list = await apiData(
          api.GET('/v1/admin/videos', {
            params: { query: { offset, limit, q } },
            signal,
          }),
        );
        if (signal.aborted) return;
        total = list.total;
        rows.push(...list.videos);
        offset += list.videos.length;
        hasMore = list.has_more;
      } while (hasMore && rows.length < target);
      videos = append ? [...videos, ...rows] : rows;
      more = hasMore;
      albums = await apiData(api.GET('/v1/admin/albums', { signal }));
    } catch (failure) {
      if (!signal.aborted) error = message(failure);
    } finally {
      if (!signal.aborted) refreshing = false;
    }
  }
  /** Refresh mounted metadata after a background upload or album mutation. */
  function syncLibrary(): void {
    void refresh();
  }
  /** The video's own menu is the sole entry point for editing and sharing. */
  function edit(video: MediaItem): void {
    editing = video;
  }
  /** Keep selected metadata current after publication completes inside the menu. */
  function updateSelected(video: MediaItem): void {
    editing = video;
  }
  /** Keep successful action feedback outside the gallery's layout. */
  function notify(value: string): void {
    notice = value;
  }

  $effect(() => {
    const q = query.trim();
    if (!loaded || !$session.authenticated || q === untrack(() => searched))
      return;
    const timer = setTimeout(() => {
      void refresh(false, true);
    }, 180);
    return () => {
      clearTimeout(timer);
    };
  });
  $effect(() => {
    if (!notice) return;
    const timer = setTimeout(() => (notice = ''), 6000);
    return () => {
      clearTimeout(timer);
    };
  });
  onMount(() => {
    const lifecycle = new AbortController();
    void (async () => {
      try {
        await loadSession();
        if (!lifecycle.signal.aborted && $session.authenticated) {
          await refresh();
          if (page.url.searchParams.has('upload')) openUploader();

          if (
            page.url.searchParams.has('upload') ||
            page.url.searchParams.has('manage')
          )
            await goto(resolve('/gallery'), {
              state: page.state,
              shallow: true,
              replace: true,
            });
        }
      } catch (failure) {
        error = message(failure);
      } finally {
        loaded = true;
      }
    })();
    window.addEventListener('mori:library-changed', syncLibrary);

    const timer = setInterval(() => {
      if ($session.authenticated && editing === null) void refresh();
    }, 10000);
    return () => {
      lifecycle.abort();
      request?.abort();
      clearInterval(timer);
      window.removeEventListener('mori:library-changed', syncLibrary);
    };
  });
</script>

<svelte:head><title>{pageTitle('Gallery')}</title></svelte:head>
<main>
  <Breadcrumbs items={[{ label: 'Gallery', href: resolve('/gallery') }]} />
  <div class="page-heading">
    <h1>Gallery</h1>
    {#if loaded && $session.authenticated}<div class="flex items-center gap-2">
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
        ><Button
          variant="default"
          type="submit"
          class="button primary"
          onclick={() => {
            openUploader();
          }}><Plus size={16} />Upload media</Button
        >
      </div>{/if}
  </div>
  {#if loaded && $session.authenticated}<CollectionToolbar
      bind:query
      count={total}
      label="Search gallery"
    />{/if}
  {#if !loaded}<p class="empty muted" role="status">Opening your library…</p>
  {:else if !$session.authenticated}<div class="empty">
      <h2>Your private library.</h2>
      <Button variant="ghost" class="text-action" href={resolve('/login')}
        >Log in to continue<ArrowUpRight size={17} /></Button
      >
    </div>
  {:else if videos.length > 0}<MediaSelection
      {videos}
      {albums}
      bind:selecting
      onmanage={edit}
      oncomplete={refresh}
    />
    {#if more}<div class="more">
        <Button
          variant="ghost"
          type="submit"
          class="text-action"
          onclick={() => refresh(true)}>Load more</Button
        >
      </div>{/if}
  {:else if query.trim()}<div class="empty-state">
      <h2>No matching media</h2>
      <p>Try another name.</p>
    </div>
  {:else}<div class="empty">
      <h2>Your library is empty.</h2>
      <p>Upload an image or video to get started. Albums are optional.</p>
    </div>{/if}
</main>

{#if loaded && $session.authenticated}
  {#if editing !== null}<MediaDialog
      video={editing}
      {albums}
      onclose={() => (editing = null)}
      oncomplete={refresh}
      onnotice={notify}
      onupdate={updateSelected}
    />{/if}
{/if}

<div class="feedback">
  {#if notice}<div class="toast">
      <p role="status">{notice}</p>
      <Button
        variant="ghost"
        size="icon"
        type="submit"
        class="icon-button"
        aria-label="Dismiss notification"
        onclick={() => (notice = '')}><X size={16} /></Button
      >
    </div>{/if}
  {#if error}<div class="toast failure">
      <p role="alert">{error}</p>
      <Button
        variant="ghost"
        size="icon"
        type="submit"
        class="icon-button"
        aria-label="Dismiss error"
        onclick={() => (error = '')}><X size={16} /></Button
      >
    </div>{/if}
</div>

<style>
  .empty {
    min-height: 55svh;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 12px;
    padding: 32px 20px;
    color: var(--muted-foreground);
    text-align: center;
    font-size: 14px;
  }
  .empty h2 {
    color: var(--foreground);
    font-size: 24px;
  }
  .more {
    display: flex;
    justify-content: center;
    padding: 20px;
  }
  .feedback {
    position: fixed;
    z-index: 10;
    bottom: 22px;
    right: 22px;
    max-width: min(420px, calc(100% - 32px));
    display: grid;
    gap: 10px;
  }
  .toast {
    display: flex;
    align-items: center;
    gap: 14px;
    background: var(--popover);
    border: 1px solid var(--input);
    box-shadow: 0 6px 30px #0004;
    border-radius: 6px;
    padding: 2px 5px 2px 16px;
    color: var(--foreground);
    font-size: 13px;
  }
  .toast p {
    flex: 1;
    line-height: 1.6;
  }
  .toast.failure {
    color: var(--destructive);
  }
  @media (max-width: 480px) {
    .feedback {
      right: 16px;
      bottom: 16px;
    }
  }
</style>
