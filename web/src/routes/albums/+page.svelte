<script lang="ts">
  import { pageTitle } from '#lib/features/page-title';
  import Button from '#lib/components/ui/button/button.svelte';
  import Breadcrumbs from '#lib/components/Breadcrumbs.svelte';
  /** Explicit album administration without exposing private library metadata to visitors. */
  import { takeInitialPage } from '#lib/api/bootstrap';
  import { onMount, untrack } from 'svelte';
  import { resolve } from '$app/paths';
  import { Plus, Library, Pencil, Trash } from '@lucide/svelte';
  import AccessGate from '#lib/components/AccessGate.svelte';
  import CopyButton from '#lib/components/CopyButton.svelte';
  import AlbumEditor from '#lib/components/library/AlbumEditor.svelte';
  import { api, apiData, session, message, type Album } from '#lib/api/client';
  /** The document seed is consumed once; returning from another page fetches fresh data. */
  const initial = untrack(() => takeInitialPage('/albums')?.albums ?? null);
  /** Server-backed album list and safe feedback for loading or clipboard errors. */
  let albums = $state<Album[]>(initial ?? []),
    loaded = $state(initial !== null),
    error = $state('');
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
      if ($session.authenticated && editor === null) void refresh();
    }, 15000);
    return () => {
      clearInterval(timer);
    };
  });
</script>

<svelte:head><title>{pageTitle('Albums')}</title></svelte:head>
<main>
  <Breadcrumbs items={[{ label: 'Albums', href: resolve('/albums') }]} />
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
    {#if error}<p class="error mb-4" role="alert">{error}</p>{/if}
    {#if !loaded}<p class="muted" role="status">Loading albums…</p>
    {:else if albums.length === 0}<div class="empty-state">
        <Library size={28} />
        <h2>No albums yet</h2>
        <p>
          Create a collection when you need one. Videos can stay outside albums.
        </p>
        <Button
          variant="outline"
          type="submit"
          class="button"
          onclick={() => (editor = { album: null, removing: false })}
          >Create your first album</Button
        >
      </div>
    {:else}<div class="album-list">
        {#each albums as album (album.id)}<article class="album-row">
            <div class="album-icon"><Library size={23} /></div>
            <div class="album-metadata">
              <a
                class="album-name"
                href={resolve('/albums/[id]', { id: album.id })}>{album.name}</a
              ><span
                class="album-visibility"
                class:public={album.visibility === 'public'}
                >{album.visibility === 'public' ? 'Public' : 'Private'}</span
              ><span class="album-address"
                >{window.location.origin}/{album.slug}</span
              >
            </div>
            <div class="album-actions">
              <CopyButton
                label={'Copy album link ' + album.name}
                value={new URL('/' + album.slug, window.location.origin).href}
                icon="link"
                disabled={album.visibility !== 'public'}
              /><Button
                variant="ghost"
                size="icon"
                type="submit"
                class="icon-button"
                aria-label={'Edit album ' + album.name}
                onclick={() => (editor = { album, removing: false })}
                ><Pencil size={16} /></Button
              ><Button
                variant="ghost"
                size="icon"
                type="submit"
                class="icon-button"
                aria-label={'Delete album ' + album.name}
                onclick={() => (editor = { album, removing: true })}
                ><Trash size={16} /></Button
              >
            </div>
          </article>{/each}
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
    border-top: 1px solid #ffffff12;
  }
  .album-row {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 18px 4px;
    border-bottom: 1px solid #ffffff12;
  }
  .album-icon {
    width: 48px;
    height: 48px;
    display: grid;
    place-items: center;
    border: 1px solid #ffffff12;
    border-radius: 6px;
    color: #aaaab8;
    background: #ffffff03;
    flex-shrink: 0;
  }
  .album-metadata {
    flex: 1;
    min-width: 0;
  }
  .album-name {
    display: flex;
    align-items: center;
    gap: 6px;
    width: fit-content;
    max-width: 100%;
    font-size: 16px;
    font-weight: 550;
    overflow-wrap: anywhere;
  }
  .album-visibility {
    display: inline-block;
    margin-top: 6px;
    padding: 2px 7px;
    border: 1px solid #ffffff18;
    border-radius: 4px;
    font-size: 12px;
    color: #9999aa;
  }
  .album-visibility.public {
    color: #a8c7b5;
    border-color: #a8c7b530;
  }
  .album-address {
    display: block;
    color: #858593;
    font-size: 14px;
    margin-top: 6px;
    user-select: text;
    overflow-wrap: anywhere;
  }
  .album-actions {
    display: flex;
    color: #aaaab8;
  }
  @media (max-width: 480px) {
    .album-row {
      gap: 10px;
      flex-wrap: wrap;
    }
    .album-actions {
      margin-left: auto;
    }
  }
</style>
