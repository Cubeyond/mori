<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: Elastic-2.0 -->

<script lang="ts">
  /** Existing membership, atomic creation-and-move, and permanent deletion share one compact sheet. */
  import { Plus, Folder, Search, ArrowLeft } from '@lucide/svelte';
  import Modal from '../Modal.svelte';
  import Input from '#lib/components/ui/input/input.svelte';
  import Button from '#lib/components/ui/button/button.svelte';
  import AlbumFields from './AlbumFields.svelte';
  import {
    api,
    apiData,
    message,
    type Album,
    type MediaItem,
  } from '#lib/api/client';
  import type { BatchAction } from '#lib/api/client';
  /** The frozen selection is independent of background collection polling. */
  let {
    items,
    albums,
    mode,
    onclose,
    oncomplete,
  }: {
    items: MediaItem[];
    albums: Album[];
    mode: 'move' | 'delete';
    onclose: () => void;
    oncomplete: () => Promise<void>;
  } = $props();
  /** Searching never changes the current target or loses the new album draft. */
  let query = $state(''),
    target = $state(''),
    creating = $state(false),
    name = $state(''),
    slug = $state(''),
    visibility = $state('private'),
    description = $state(''),
    busy = $state(false),
    error = $state('');
  /** Natural labels remain correct for a single selected original. */
  const countLabel = $derived(
    String(items.length) + (items.length === 1 ? ' item' : ' items'),
  );
  /** Keep list order and the exact requested selection stable while submitting. */
  const matches = $derived(
    albums.filter((album) =>
      album.name.toLowerCase().includes(query.trim().toLowerCase()),
    ),
  );
  /** Apply one server transaction before clearing selection or refreshing parent metadata. */
  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy) return;
    busy = true;
    error = '';
    try {
      const action: BatchAction =
        mode === 'delete'
          ? { type: 'delete' }
          : creating
            ? {
                type: 'create_album',
                album: { name, slug, visibility, description },
              }
            : { type: 'move', album_id: target };
      await apiData(
        api.POST('/v1/admin/videos/batch', {
          body: { ids: items.map((item) => item.id), action },
        }),
      );
      await oncomplete();
      onclose();
    } catch (failure) {
      error = message(failure);
    } finally {
      busy = false;
    }
  }
</script>

<Modal
  open={true}
  title={mode === 'delete'
    ? 'Delete ' + countLabel + '?'
    : creating
      ? 'Create album and move'
      : 'Move to album'}
  onclose={() => {
    if (!busy) onclose();
  }}
>
  <form onsubmit={submit}>
    {#if mode === 'delete'}<p class="description">
        These files will be permanently deleted.
      </p>
      <div class="previews" aria-label="Selected media previews">
        {#each items.slice(0, 4) as item (item.id)}{#if item.poster_url !== null}<img
              src={item.poster_url}
              alt={item.name}
            />{/if}{/each}{#if items.length > 4}<span>+{items.length - 4}</span
          >{/if}
      </div>
    {:else if creating}<AlbumFields
        bind:name
        bind:slug
        bind:visibility
        bind:description
        prefix="batch-album"
        suggestSlug
        disabled={busy}
      />
    {:else}<label class="album-search"
        ><Search size={17} /><span class="sr-only">Search albums</span><Input
          type="search"
          bind:value={query}
          placeholder="Search albums…"
          disabled={busy}
        /></label
      >
      <div class="album-list" role="group" aria-label="Destination album">
        {#each matches as album (album.id)}<label
            class="album-row"
            class:chosen={target === album.id}
            ><input
              type="radio"
              name="batch-destination"
              value={album.id}
              bind:group={target}
              disabled={busy}
            /><Folder size={19} /><span>{album.name}</span></label
          >{/each}{#if matches.length === 0}<p class="muted">
            {albums.length === 0 ? 'No albums yet' : 'No matching albums'}
          </p>{/if}
      </div>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    <footer>
      {#if mode === 'move'}<Button
          variant="ghost"
          type="button"
          disabled={busy}
          onclick={() => {
            creating = !creating;
            error = '';
          }}
          >{#if creating}<ArrowLeft size={16} />Existing albums{:else}<Plus
              size={16}
            />Create album{/if}</Button
        >{:else}<span></span>{/if}
      <div class="submit-actions">
        <Button
          variant="outline"
          type="button"
          onclick={onclose}
          disabled={busy}>Cancel</Button
        ><Button
          variant={mode === 'delete' ? 'destructive' : 'default'}
          type="submit"
          disabled={busy || (mode === 'move' && !creating && target === '')}
          >{busy
            ? 'Working…'
            : mode === 'delete'
              ? 'Delete ' + countLabel
              : creating
                ? 'Create and move ' + countLabel
                : 'Move ' + countLabel}</Button
        >
      </div>
    </footer>
  </form>
</Modal>

<style>
  .description {
    color: var(--muted-foreground);
    margin-bottom: 20px;
  }
  .previews {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 24px;
  }
  .previews img {
    width: calc((100% - 30px) / 4);
    aspect-ratio: 1;
    object-fit: cover;
    border-radius: 7px;
  }
  .previews span {
    color: var(--muted-foreground);
    font-size: 14px;
  }
  .album-search {
    display: flex;
    align-items: center;
    position: relative;
  }
  .album-search :global(input) {
    margin: 0;
    padding-left: 38px;
  }
  .album-search :global(svg) {
    position: absolute;
    left: 12px;
    pointer-events: none;
    color: var(--muted-foreground);
  }
  .album-list {
    display: grid;
    gap: 4px;
    max-height: 280px;
    overflow: auto;
    margin: 16px 0;
    scrollbar-width: thin;
  }
  .album-list p {
    padding: 16px 4px;
  }
  .album-row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px;
    border: 1px solid transparent;
    border-radius: 7px;
    cursor: pointer;
    min-width: 0;
  }
  .album-row span {
    overflow-wrap: anywhere;
  }
  .album-row input {
    margin: 0;
    accent-color: var(--selection);
  }
  .album-row:hover {
    background: var(--hover-surface);
  }
  .chosen {
    border-color: var(--selection);
    background: var(--accent);
  }
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 24px;
    flex-wrap: wrap;
  }
  .submit-actions {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-left: auto;
    flex-wrap: wrap;
    justify-content: flex-end;
  }
  @media (max-width: 480px) {
    footer {
      flex-direction: column-reverse;
      align-items: stretch;
      gap: 8px;
    }
    .submit-actions {
      display: flex;
      flex-direction: column-reverse;
      align-items: stretch;
      width: 100%;
      margin: 0;
      gap: 8px;
    }
    .submit-actions :global(button),
    footer > :global(button) {
      width: 100%;
      min-height: 44px;
    }
    .submit-actions :global(button:first-child) {
      background: transparent;
      border-color: transparent;
    }
  }
</style>
