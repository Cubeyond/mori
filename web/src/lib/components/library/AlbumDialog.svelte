<script lang="ts">
  /** Quick album navigation reuses the canonical editor and deletion confirmation. */
  import Button from '#lib/components/ui/button/button.svelte';
  import { Folder, Plus, Pencil, Trash } from '@lucide/svelte';
  import { resolve } from '$app/paths';
  import type { Album } from '#lib/api/client';
  import Modal from '../Modal.svelte';
  import AlbumEditor from './AlbumEditor.svelte';
  /** The upload controller owns transfer lifetime and refreshes collection choices. */
  let {
    open,
    albums,
    onclose,
    onrefresh,
  }: {
    open: boolean;
    albums: Album[];
    onclose: () => void;
    onrefresh: () => Promise<void>;
  } = $props();
  /** The same editor handles creation, editable slugs, and confirmed deletion everywhere. */
  let editor = $state<{ album: Album | null; removing: boolean } | null>(null);
  $effect(() => {
    if (!open) editor = null;
  });
</script>

<Modal
  open={open && editor === null}
  title="Albums"
  {onclose}
  closeLabel="Back to upload"
>
  <div class="album-heading">
    <p class="muted text-sm">Organize your videos.</p>
    <Button
      variant="ghost"
      onclick={() => (editor = { album: null, removing: false })}
      ><Plus size={18} />New album</Button
    >
  </div>
  <div class="list" aria-label="Albums">
    {#each albums as album (album.id)}<div class="album-row">
        <Folder size={22} />
        <a class="metadata" href={resolve('/albums/[id]', { id: album.id })}
          ><span class="name">{album.name}</span><span class="details"
            >/{album.slug}</span
          ></a
        >
        <Button
          variant="ghost"
          size="icon"
          class="icon-button"
          aria-label={`Edit album ${album.name}`}
          onclick={() => (editor = { album, removing: false })}
          ><Pencil size={18} /></Button
        >
        <Button
          variant="ghost"
          size="icon"
          class="icon-button"
          aria-label={`Delete album ${album.name}`}
          onclick={() => (editor = { album, removing: true })}
          ><Trash size={18} /></Button
        >
      </div>{:else}<div class="empty-state">
        <p>No albums yet.</p>
        <p>Videos can be uploaded without an album.</p>
      </div>{/each}
  </div>
</Modal>
{#if open && editor !== null}<AlbumEditor
    album={editor.album}
    removing={editor.removing}
    onclose={() => (editor = null)}
    oncomplete={onrefresh}
  />{/if}

<style>
  .album-heading,
  .album-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .album-heading {
    justify-content: space-between;
    margin-bottom: 16px;
  }
  .list {
    max-height: 54svh;
    overflow-y: auto;
  }
  .album-row {
    min-height: 76px;
    border-bottom: 1px solid var(--border);
  }
  .metadata {
    flex: 1;
    min-width: 0;
  }
  .name {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 15px;
    line-height: 20px;
  }
  .details {
    display: block;
    color: var(--muted-foreground);
    font-size: 14px;
    line-height: 20px;
    margin-top: 4px;
    overflow-wrap: anywhere;
  }
</style>
