<script lang="ts">
  /** The editable local queue survives dismissal and stays separate from bounded concurrent transfers. */
  import { onMount } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import { resolve } from '$app/paths';
  import { Images, ArrowUp, Pause, FolderCog, Upload } from '@lucide/svelte';
  import Button from '#lib/components/ui/button/button.svelte';
  import NativeSelect from '#lib/components/ui/native-select/native-select.svelte';
  import Modal from '../Modal.svelte';
  import UploadSelection from './UploadSelection.svelte';
  import { preferences } from '#lib/features/preferences';
  import type { Album, UploadPolicy } from '#lib/api/client';
  import { message } from '#lib/api/client';
  import {
    appendUploadFiles,
    uploadKind,
  } from '#lib/features/upload-selection';
  import { runUploadQueue } from '#lib/features/upload-queue';
  import {
    uploadVideo,
    savedUploads,
    cancelUpload,
    type PendingUpload,
    type TransferStatus,
  } from '#lib/features/upload';
  /** The persistent controller owns navigation, storage discovery and collection refreshes. */
  let {
    open,
    albums,
    policy,
    storageConfigured,
    initialAlbumId,
    onclose,
    oncomplete,
    onstatus,
    onnotice,
    onalbums,
  }: {
    open: boolean;
    albums: Album[];
    policy: UploadPolicy | null;
    storageConfigured: boolean | null;
    initialAlbumId: string | null;
    onclose: () => void;
    oncomplete: () => Promise<void>;
    onstatus: (status: TransferStatus | null) => void;
    onnotice: (notice: string) => void;
    onalbums: () => void;
  } = $props();
  /** Original File objects remain intact while picker additions replace only the queue array. */
  let files = $state<File[]>([]),
    activeFiles = $state<File[]>([]);
  /** The first waiting original determines whether the draft can be submitted. */
  const file = $derived(files[0] ?? null);
  /** Album and visibility are fixed while a resumable or active transfer owns them. */
  let albumId = $state(''),
    visibility = $state('private');
  /** Saved session metadata contains no media bytes or AWS credentials. */
  let pending = $state<PendingUpload[]>([]),
    busy = $state(false);
  /** Progress belongs to the modal or its background pill, never the gallery layout. */
  let progress = $state(0),
    phase = $state(''),
    error = $state(''),
    dragging = $state(false);
  /** Pausing affects transfer rather than ordinary modal dismissal. */
  let controller: AbortController | undefined;
  $effect(() => {
    if (open && file === null && pending.length === 0 && !busy) {
      albumId = initialAlbumId ?? '';
      visibility = $preferences.uploadVisibility;
    }
  });
  onMount(() => {
    pending = savedUploads();
    if (pending.length > 0)
      onstatus({ progress: 0, phase: 'Resume upload', paused: true });
    return () => controller?.abort();
  });
  /** Invalid additions preserve the existing selection instead of silently clearing it. */
  function selectFiles(selected: File[]): void {
    if (selected.length === 0) return;
    error = '';
    try {
      if (policy === null) throw new Error('Upload settings are loading.');
      files = appendUploadFiles(files, selected, policy);
    } catch (failure) {
      error = message(failure);
    }
  }
  /** Picker and drop gestures append through exactly the same validation path. */
  function dropFiles(event: DragEvent): void {
    event.preventDefault();
    dragging = false;
    selectFiles(Array.from(event.dataTransfer?.files ?? []));
  }
  /** Removing a paused original cancels only its matching reservation; active originals stay locked. */
  async function removeFile(selected: File): Promise<void> {
    if (activeFiles.includes(selected)) return;
    const position = files.indexOf(selected);
    files = files.filter((entry) => entry !== selected);
    try {
      await cancelUpload(selected);
      pending = savedUploads();
      error = '';
      if (!busy)
        onstatus(
          files.length === 0
            ? null
            : { progress: 0, phase: 'Continue uploads', paused: true },
        );
    } catch (failure) {
      files.splice(Math.max(0, position), 0, selected);
      error = message(failure);
    }
  }
  /** Upload two live entries at once with byte-weighted progress and independent resumes. */
  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (file === null || policy === null || busy) return;
    const transferPolicy = policy;
    busy = true;
    error = '';
    progress = 0;
    phase = 'Preparing uploads';
    onstatus({ progress, phase });
    controller = new AbortController();
    const signal = controller.signal;
    const transferred = new SvelteMap<File, number>();
    let completedBytes = 0;
    /** Combine progress by bytes so competing workers never overwrite the meter. */
    function report(label: string): void {
      const total =
        completedBytes + files.reduce((bytes, entry) => bytes + entry.size, 0);
      const sent =
        completedBytes +
        [...transferred].reduce(
          (bytes, [entry, value]) => bytes + (entry.size * value) / 100,
          0,
        );
      progress = total === 0 ? 0 : Math.round((sent / total) * 100);
      phase =
        activeFiles.length > 1
          ? `${String(activeFiles.length)} files · ${label}`
          : label;
      onstatus({ progress, phase });
    }
    try {
      const completed = await runUploadQueue(
        () => files,
        async (entry, transferSignal) => {
          activeFiles = [...activeFiles, entry];
          try {
            await uploadVideo(
              entry,
              albumId || null,
              visibility,
              (value, label) => {
                transferred.set(entry, value);
                report(label);
              },
              transferSignal,
              uploadKind(entry, transferPolicy),
            );
            completedBytes += entry.size;
            pending = savedUploads();
            void oncomplete().catch((failure: unknown) => {
              onnotice(message(failure));
            });
          } finally {
            transferred.delete(entry);
            activeFiles = activeFiles.filter((active) => active !== entry);
          }
        },
        signal,
      );
      onclose();
      onnotice(
        completed === 1
          ? 'Media uploaded.'
          : `${String(completed)} files uploaded.`,
      );
      await oncomplete();
    } catch (failure) {
      pending = savedUploads();
      error = signal.aborted ? 'Uploads paused.' : message(failure);
      if (!open) onnotice('Upload interrupted. Open Upload to continue.');
    } finally {
      activeFiles = [];
      busy = false;
      onstatus(
        pending.length === 0 && files.length === 0
          ? null
          : { progress, phase: 'Continue uploads', paused: true },
      );
    }
  }
  /** Discard saved multipart sessions while retaining selected originals for a fresh upload. */
  async function discard(): Promise<void> {
    try {
      await cancelUpload();
      pending = [];
      onstatus(
        files.length === 0
          ? null
          : { progress: 0, phase: 'Continue uploads', paused: true },
      );
      error = '';
      await oncomplete();
    } catch (failure) {
      error = message(failure);
    }
  }
</script>

<Modal
  {open}
  title="Upload media"
  {onclose}
  closeLabel="Close upload"
  maxWidth={600}
  hideScrollbar
>
  {#if storageConfigured === null}<p class="storage-setup muted" role="status">
      Loading upload settings…
    </p>
  {:else if !storageConfigured}<div class="storage-setup">
      <p>Connect Amazon S3 to start uploading.</p>
      <Button
        href={resolve('/settings/storage')}
        class="button primary"
        onclick={onclose}>Set up storage</Button
      >
    </div>
  {:else}<form onsubmit={submit}>
      {#if pending.length > 0 && !busy}<div class="resume">
          <p>
            {file === null ? 'Continue unfinished uploads' : 'Uploads paused'}
          </p>
          {#if file === null}<span>Choose the original files to resume.</span
            >{/if}<Button
            variant="ghost"
            type="button"
            class="text-action"
            disabled={busy}
            onclick={discard}>Discard paused uploads</Button
          >
        </div>{/if}
      {#if files.length > 0 && policy !== null}<UploadSelection
          {files}
          {policy}
          {activeFiles}
          onremove={(selected: File) => {
            void removeFile(selected);
          }}
        />{/if}
      <label class="drop-zone" class:compact={files.length > 0} class:dragging>
        <input
          type="file"
          multiple
          accept={policy?.allowed_extensions
            .map((extension) => '.' + extension)
            .join(',') ?? 'image/*,video/*'}
          aria-label="Media files"
          disabled={policy === null}
          onchange={(event) => {
            const selected = Array.from(event.currentTarget.files ?? []);
            event.currentTarget.value = '';
            selectFiles(selected);
          }}
          ondragover={(event) => {
            event.preventDefault();
            dragging = true;
          }}
          ondragleave={() => (dragging = false)}
          ondrop={dropFiles}
        />
        {#if files.length > 0}<Upload size={20} /><span
            >Drop more files here</span
          >
        {:else}<Images size={28} /><strong>Drop images or videos here</strong
          ><span>or click to choose files</span>{/if}
      </label>
      <div class="options">
        <div class="field">
          <div class="album-label">
            <label for="upload-album"
              >Album <span class="optional">optional</span></label
            ><button
              type="button"
              class="album-action"
              aria-label="Manage albums"
              title="Manage albums"
              onclick={onalbums}><FolderCog size={16} /></button
            >
          </div>
          <NativeSelect
            id="upload-album"
            bind:value={albumId}
            disabled={busy || pending.length > 0}
            ><option value="">No album</option
            >{#each albums as album (album.id)}<option value={album.id}
                >{album.name}</option
              >{/each}</NativeSelect
          >
        </div>
        <div class="field">
          <div class="album-label">
            <label for="upload-visibility">Visibility</label>
          </div>
          <NativeSelect
            id="upload-visibility"
            bind:value={visibility}
            disabled={busy || pending.length > 0}
            ><option value="private">Private</option><option value="public"
              >Public</option
            ></NativeSelect
          >
        </div>
      </div>
      <p class="limits">
        {policy === null
          ? 'Loading upload settings…'
          : 'Images ' +
            String(policy.max_image_upload_bytes / 1_048_576) +
            ' MiB · Videos ' +
            String(policy.max_upload_bytes / 1_000_000_000) +
            ' GB'}
      </p>
      {#if error}<p class="error mb-4" role="alert">{error}</p>{/if}
      {#if busy}<div class="transfer" role="status">
          <div><span>{phase}</span><span>{progress}%</span></div>
          <progress max="100" value={progress} aria-label="Upload progress"
          ></progress>
          <p>You can close this window while the upload continues.</p>
        </div>{/if}
      <footer>
        <Button
          variant="ghost"
          class="text-action"
          type="button"
          onclick={onclose}>{busy ? 'Continue in background' : 'Cancel'}</Button
        >
        {#if busy}<Button
            variant="outline"
            class="button"
            type="button"
            onclick={() => controller?.abort()}><Pause size={16} />Pause</Button
          >
        {:else}<Button
            variant="default"
            type="submit"
            class="button primary"
            disabled={file === null || policy === null}
            ><ArrowUp size={17} />{pending.length > 0
              ? 'Continue uploads'
              : files.length > 0
                ? 'Upload ' +
                  String(files.length) +
                  (files.length === 1 ? ' file' : ' files')
                : 'Upload media'}</Button
          >{/if}
      </footer>
    </form>{/if}
</Modal>

<style>
  .storage-setup {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 24px;
    padding-block: 8px;
    font-size: 16px;
  }
  .drop-zone {
    position: relative;
    min-height: 190px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    border: 1px dashed var(--input);
    background: var(--soft-surface);
    border-radius: 7px;
    text-align: center;
    padding: 24px;
    margin-bottom: 18px;
  }
  .drop-zone.compact {
    min-height: 48px;
    flex-direction: row;
    padding: 10px 16px;
  }
  .drop-zone:hover,
  .dragging,
  .drop-zone:has(input:focus-visible) {
    border-color: var(--media-ring);
  }

  .drop-zone input {
    position: absolute;
    inset: 0;
    outline: none;
    box-shadow: none;
    margin: 0;
    padding: 0;
    opacity: 0;
    cursor: pointer;
    width: 100%;
    height: 100%;
  }
  .drop-zone strong {
    color: var(--foreground);
    font-size: 15px;
    font-weight: 400;
  }
  .drop-zone span {
    font-size: 14px;
    color: var(--muted-foreground);
  }
  .options {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
  }
  .album-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 36px;
  }
  .album-action {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    margin: 0;
    color: var(--muted-foreground);
  }
  :global(#upload-album),
  :global(#upload-visibility) {
    margin: 0;
  }
  .optional {
    color: var(--muted-foreground);
    font-size: 12px;
    margin-left: 4px;
  }
  .limits {
    font-size: 12px;
    color: var(--muted-foreground);
    margin: 0 0 18px;
  }
  footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  .resume {
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: 7px;
    margin-bottom: 16px;
    font-size: 14px;
  }
  .resume span {
    display: block;
    color: var(--muted-foreground);
    margin: 5px 0;
    font-size: 13px;
  }
  .transfer {
    margin-bottom: 18px;
  }
  .transfer div {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    font-size: 13px;
    margin-bottom: 8px;
  }
  progress {
    display: block;
    width: 100%;
    height: 3px;
    accent-color: var(--foreground);
  }
  .transfer p {
    font-size: 12px;
    color: var(--muted-foreground);
    margin-top: 8px;
  }
  @media (max-width: 380px) {
    :global(#upload-album),
    :global(#upload-visibility) {
      font-size: 14px;
      padding-right: 30px;
    }
    .optional {
      display: none;
    }
  }
</style>
