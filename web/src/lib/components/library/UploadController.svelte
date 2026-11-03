<script lang="ts">
  import Button from '#lib/components/ui/button/button.svelte';
  /** Persistent transfer owner lives above sidebar routes so navigation cannot interrupt uploads. */
  import { onMount } from 'svelte';
  import { X, ArrowUpRight } from '@lucide/svelte';
  import {
    api,
    apiData,
    message,
    type Album,
    type UploadPolicy,
    type UnfinishedUpload,
  } from '#lib/api/client';
  import type { TransferStatus } from '#lib/features/upload';
  import UploadDialog from './UploadDialog.svelte';
  import AlbumDialog from './AlbumDialog.svelte';
  /** One operation is visible while its persistent uploader remains mounted. */
  let open = $state(false),
    albumOpen = $state(false);
  /** Current album metadata is refreshed whenever the picker opens. */
  let albums = $state<Album[]>([]);
  /** Server reservations remain discoverable across reloads and browsers. */
  let unfinished = $state<UnfinishedUpload[]>([]);
  /** Album context applies only to fresh drafts; active transfers retain their selection. */
  let initialAlbumId = $state<string | null>(null);
  /** Server settings are authoritative for limits and accepted extensions. */
  let policy = $state<UploadPolicy | null>(null);
  /** Fresh installations lead to Storage setup before accepting a local file. */
  let storageConfigured = $state<boolean | null>(null);
  /** Background progress and safe transient feedback never push the page downward. */
  let transfer = $state<TransferStatus | null>(null),
    notice = $state(''),
    error = $state('');
  /** Late discovery responses cannot restore reservations after cancellation has cleared them. */
  let refreshVersion = 0;
  /** Refresh only the optional album selector, without touching the active file draft. */
  async function refreshAlbums(): Promise<void> {
    const version = ++refreshVersion;
    try {
      const [loadedAlbums, loadedPolicy, storage, reservations] =
        await Promise.all([
          apiData(api.GET('/v1/admin/albums')),
          apiData(api.GET('/v1/admin/upload-policy')),
          apiData(api.GET('/v1/admin/storage')),
          apiData(api.GET('/v1/admin/uploads')),
        ]);
      if (version !== refreshVersion) return;
      albums = loadedAlbums;
      unfinished = reservations;
      policy = loadedPolicy;
      storageConfigured = storage.configured;
    } catch (failure) {
      if (version === refreshVersion) error = message(failure);
    }
  }
  /** Open the existing transfer without creating a second uploader. */
  function show(event?: Event): void {
    const detail: unknown = event instanceof CustomEvent ? event.detail : null;
    initialAlbumId = typeof detail === 'string' ? detail : null;
    albumOpen = false;
    open = true;
    void refreshAlbums();
  }
  /** Notify any mounted collection that upload or album metadata has changed. */
  async function complete(): Promise<void> {
    await refreshAlbums();
    window.dispatchEvent(new Event('mori:library-changed'));
  }
  /** Replace only the visible sheet while preserving the upload draft. */
  function showAlbums(): void {
    open = false;
    albumOpen = true;
  }
  /** Return from quick album administration to the original upload. */
  function closeAlbums(): void {
    albumOpen = false;
    open = true;
  }
  onMount(() => {
    window.addEventListener('mori:upload', show);
    return () => {
      window.removeEventListener('mori:upload', show);
    };
  });
  $effect(() => {
    if (!notice) return;
    const timer = setTimeout(() => (notice = ''), 6000);
    return () => {
      clearTimeout(timer);
    };
  });
</script>

<UploadDialog
  {open}
  {albums}
  {unfinished}
  {policy}
  {storageConfigured}
  {initialAlbumId}
  onclose={() => (open = false)}
  oncomplete={complete}
  onstatus={(value: TransferStatus | null) => (transfer = value)}
  onnotice={(value: string) => (notice = value)}
  onalbums={showAlbums}
/>
<AlbumDialog
  open={albumOpen}
  {albums}
  onclose={closeAlbums}
  onrefresh={complete}
/>
<div class="upload-feedback">
  {#if transfer !== null && !open}<button
      class="transfer-pill"
      onclick={show}
      aria-label="Open upload progress"
      ><span class="dot"></span><span>{transfer.phase}</span><span
        class="percentage">{transfer.progress}%</span
      ><ArrowUpRight size={16} /></button
    >{/if}
  {#if notice}<div class="toast">
      <p role="status">{notice}</p>
      <Button
        variant="ghost"
        size="icon"
        type="submit"
        class="icon-button"
        aria-label="Dismiss upload notification"
        onclick={() => (notice = '')}><X size={16} /></Button
      >
    </div>{/if}
  {#if error}<div class="toast error">
      <p role="alert">{error}</p>
      <Button
        variant="ghost"
        size="icon"
        type="submit"
        class="icon-button"
        aria-label="Dismiss upload error"
        onclick={() => (error = '')}><X size={16} /></Button
      >
    </div>{/if}
</div>

<style>
  .upload-feedback {
    position: fixed;
    z-index: 20;
    bottom: 22px;
    right: 22px;
    display: grid;
    gap: 10px;
    max-width: min(420px, calc(100% - 32px));
  }
  .toast,
  .transfer-pill {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 2px 5px 2px 16px;
    border: 1px solid var(--input);
    background: var(--popover);
    border-radius: 6px;
    box-shadow: 0 6px 30px #0004;
    font-size: 13px;
  }
  .toast p {
    flex: 1;
    line-height: 1.6;
  }
  .transfer-pill {
    min-height: 48px;
    padding: 10px 16px;
    text-align: left;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--media-ring);
  }
  .percentage {
    color: var(--muted-foreground);
    font-variant-numeric: tabular-nums;
    margin-left: auto;
  }
  @media (max-width: 480px) {
    .upload-feedback {
      right: 16px;
      bottom: 16px;
    }
  }
</style>
