<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: Elastic-2.0 -->

<script lang="ts">
  /** Gallery and album pages share the same bounded selection and floating action surface. */
  import { SvelteSet } from 'svelte/reactivity';
  import {
    FolderInput,
    FolderMinus,
    Trash,
    X,
    CheckCheck,
  } from '@lucide/svelte';
  import Button from '#lib/components/ui/button/button.svelte';
  import Gallery from '../Gallery.svelte';
  import BatchDialog from './BatchDialog.svelte';
  import {
    api,
    apiData,
    message,
    type Album,
    type MediaItem,
  } from '#lib/api/client';
  /** The parent places the mode toggle beside its existing upload action. */
  let {
    videos,
    albums,
    selecting = $bindable(false),
    onmanage,
    oncomplete,
  }: {
    videos: MediaItem[];
    albums: Album[];
    selecting?: boolean;
    onmanage: (item: MediaItem) => void;
    oncomplete: () => Promise<void>;
  } = $props();
  /** Selection identities survive metadata refreshes while only visible, ready items can be acted on. */
  const ids = new SvelteSet<string>();
  const available = $derived(
    videos.filter((item) => ['ready', 'published'].includes(item.state)),
  );
  const selected = $derived(available.filter((item) => ids.has(item.id)));
  /** A frozen dialog selection cannot silently expand when more media is loaded. */
  let dialog = $state<'move' | 'delete' | null>(null),
    snapshot = $state<MediaItem[]>([]),
    busy = $state(false),
    error = $state('');
  $effect(() => {
    if (!selecting) ids.clear();
  });
  /** Escape leaves selection mode without interfering with open dialogs or ordinary text editing. */
  function keyboard(event: KeyboardEvent): void {
    if (!selecting || dialog !== null || busy) return;
    if (event.key === 'Escape') {
      selecting = false;
      error = '';
    }
  }
  /** Toggle by identifier; queued and processing records cannot be selected. */
  function toggle(id: string): void {
    if (ids.has(id)) {
      ids.delete(id);
      return;
    }
    if (selected.length >= 100) {
      error = 'Select up to 100 items at a time.';
      return;
    }
    ids.add(id);
    error = '';
  }
  /** Preserve the exact confirmation scope if polling updates the underlying gallery. */
  function open(mode: 'move' | 'delete'): void {
    snapshot = [...selected];
    dialog = mode;
    error = '';
  }
  /** Successful operations clear selected identities and reload current membership and URLs. */
  async function complete(): Promise<void> {
    ids.clear();
    selecting = false;
    await oncomplete();
  }
  /** Detaching is reversible and never changes visibility or deletes original media. */
  async function detach(): Promise<void> {
    busy = true;
    error = '';
    try {
      await apiData(
        api.POST('/v1/admin/videos/batch', {
          body: {
            ids: selected.map((item) => item.id),
            action: { type: 'move', album_id: null },
          },
        }),
      );
      await complete();
    } catch (failure) {
      error = message(failure);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:window onkeydown={keyboard} />
<div class="collection" class:selecting>
  <Gallery
    {videos}
    {onmanage}
    selection={selecting ? ids : undefined}
    onselect={toggle}
  />
</div>
{#if selecting}<div
    class="selection-bar"
    role="group"
    aria-label="Selected media actions"
    aria-busy={busy}
  >
    <span class="count" aria-live="polite">{selected.length} selected</span>
    <Button
      variant="ghost"
      type="button"
      disabled={busy || available.length === 0}
      aria-label="Select all loaded media"
      onclick={() => {
        ids.clear();
        for (const item of available.slice(0, 100)) ids.add(item.id);
      }}><CheckCheck size={18} /><span>Select all</span></Button
    >
    <span class="separator"></span>
    <Button
      variant="ghost"
      type="button"
      disabled={busy || selected.length === 0}
      onclick={() => {
        open('move');
      }}><FolderInput size={18} /><span>Move to album</span></Button
    >
    <Button
      variant="ghost"
      type="button"
      disabled={busy || !selected.some((item) => item.album_id !== null)}
      onclick={detach}
      ><FolderMinus size={18} /><span>Remove from album</span></Button
    >
    <Button
      variant="destructive"
      type="button"
      disabled={busy || selected.length === 0}
      onclick={() => {
        open('delete');
      }}><Trash size={18} /><span>Delete</span></Button
    >
    <span class="separator"></span>
    <Button
      variant="ghost"
      size="icon"
      type="button"
      aria-label="Clear selection"
      disabled={busy}
      onclick={() => {
        selecting = false;
        error = '';
      }}><X size={18} /></Button
    >
  </div>{/if}
{#if error}<p class="selection-error error" role="alert">
    {error}<button
      type="button"
      aria-label="Dismiss selection error"
      onclick={() => {
        error = '';
      }}><X size={17} /></button
    >
  </p>{/if}
{#if dialog !== null}<BatchDialog
    items={snapshot}
    {albums}
    mode={dialog}
    onclose={() => {
      dialog = null;
    }}
    oncomplete={complete}
  />{/if}

<style>
  .collection.selecting {
    padding-bottom: 96px;
  }
  @media (max-width: 767px) {
    .collection.selecting {
      padding-bottom: 152px;
    }
  }
  .selection-bar {
    position: fixed;
    z-index: 20;
    bottom: max(20px, env(safe-area-inset-bottom));
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 4px;
    width: max-content;
    max-width: calc(100vw - 32px);
    padding: 8px;
    border: 1px solid var(--input);
    border-radius: 10px;
    background: var(--popover);
    box-shadow: 0 8px 36px #0005;
  }
  .count {
    font-size: 14px;
    padding: 0 12px;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
  .separator {
    height: 24px;
    width: 1px;
    background: var(--border);
    margin: 0 4px;
  }
  .selection-error {
    position: fixed;
    z-index: 21;
    bottom: 88px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 12px;
    background: var(--popover);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 12px 16px;
    max-width: calc(100vw - 32px);
  }
  .selection-error button {
    display: grid;
    place-items: center;
    min-width: 32px;
    min-height: 32px;
  }
  @media (max-width: 767px) {
    .selection-bar {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      width: calc(100vw - 24px);
      gap: 4px;
      padding: 8px;
    }
    .count {
      grid-column: 1 / 4;
    }
    .separator {
      display: none;
    }
    .selection-bar :global(button) {
      height: auto;
      min-height: 44px;
      flex-direction: column;
      gap: 4px;
      padding: 8px 4px;
      font-size: 11px;
      line-height: 15px;
    }
    .selection-bar :global(button:last-child) {
      grid-column: 4;
      grid-row: 1;
      min-height: 36px;
      justify-self: end;
    }
    .selection-error {
      bottom: 152px;
    }
  }
</style>
