<script lang="ts">
  import { pageTitle } from '#lib/features/page-title';
  import Button from '#lib/components/ui/button/button.svelte';
  /** Explicit album administration without exposing private library metadata to visitors. */
  import { takeInitialPage } from '#lib/api/bootstrap';
  import { onMount, untrack } from 'svelte';
  import { Plus, Library } from '@lucide/svelte';
  import AccessGate from '#lib/components/AccessGate.svelte';
  import CollectionToolbar from '#lib/components/library/CollectionToolbar.svelte';
  import AlbumCard from '#lib/components/library/AlbumCard.svelte';
  import AlbumEditor from '#lib/components/library/AlbumEditor.svelte';
  import {
    api,
    apiData,
    session,
    message,
    type Album,
    type AlbumSummary,
  } from '#lib/api/client';
  /** The document seed is consumed once; returning from another page fetches fresh data. */
  const initial = untrack(() => takeInitialPage('/albums')?.albums ?? null);
  /** Server-backed album list and safe feedback for loading or clipboard errors. */
  let albums = $state<AlbumSummary[]>(initial ?? []),
    loaded = $state(initial !== null),
    error = $state('');
  /** Album metadata is fully loaded, so search includes every collection. */
  let query = $state('');
  const visible = $derived(
    albums.filter((album) =>
      (album.name + ' ' + album.description)
        .toLocaleLowerCase()
        .includes(query.trim().toLocaleLowerCase()),
    ),
  );
  /** Each editor instance owns one album draft or destructive confirmation. */
  let editor = $state<{ album: Album | null; removing: boolean } | null>(null);
  /** Fetch metadata only after the administrator session has been established. */
  async function refresh(): Promise<void> {
    error = '';
    try {
      albums = await apiData(api.GET('/v1/admin/albums'));
      loaded = true;
    } catch (failure) {
      error = message(failure);
    }
  }
  $effect(() => {
    if ($session.authenticated && !loaded) void refresh();
  });
  onMount(() => {
    const timer = setInterval(() => {
      if (!document.hidden && $session.authenticated && editor === null)
        void refresh();
    }, 15000);
    return () => {
      clearInterval(timer);
    };
  });
</script>

<svelte:head><title>{pageTitle('Albums')}</title></svelte:head>
<main>
  <div class="page-heading">
    <h1>Albums</h1>
    {#if $session.authenticated}<Button
        variant="default"
        type="submit"
        class="button primary"
        onclick={() => (editor = { album: null, removing: false })}
        ><Plus size={16} />New album</Button
      >{/if}
  </div>
  <AccessGate>
    {#if loaded}<CollectionToolbar
        bind:query
        count={visible.length}
        noun="album"
        label="Search albums"
      />{/if}
    {#if error}<p class="error mb-4" role="alert">{error}</p>{/if}
    {#if !loaded}<p class="muted" role="status">Loading albums…</p>
    {:else if albums.length === 0}<div class="empty-state">
        <Library size={28} />
        <h2>No albums yet</h2>
        <p>Create a collection for your images and videos.</p>
        <Button
          variant="outline"
          type="submit"
          class="button"
          onclick={() => (editor = { album: null, removing: false })}
          >Create your first album</Button
        >
      </div>
    {:else if visible.length === 0}<div class="empty-state">
        <h2>No matching albums</h2>
        <p>Try another name.</p>
      </div>
    {:else}<div class="album-list">
        {#each visible as album (album.id)}<AlbumCard
            {album}
            onedit={(value: Album) =>
              (editor = { album: value, removing: false })}
            ondelete={(value: Album) =>
              (editor = { album: value, removing: true })}
          />{/each}
      </div>{/if}
  </AccessGate>
</main>
{#if editor !== null}<AlbumEditor
    album={editor.album}
    removing={editor.removing}
    onclose={() => (editor = null)}
    oncomplete={refresh}
  />{/if}

<style>
  .album-list {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(100%, 440px), 1fr));
    gap: 16px;
  }
</style>
