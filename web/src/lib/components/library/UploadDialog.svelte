<script lang="ts">
  /** The editable local queue survives dismissal and stays separate from one active transfer. */
  import { onMount } from 'svelte';
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
    savedUpload,
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
    activeFile = $state<File | null>(null);
  /** Resume always targets the first remaining original file. */
  const file = $derived(files[0] ?? null);
  /** Album and visibility are fixed while a resumable or active transfer owns them. */
  let albumId = $state(''),
    visibility = $state('private');
  /** Saved session metadata contains no media bytes or AWS credentials. */
  let pending = $state<PendingUpload | null>(null),
    busy = $state(false);
  /** Progress belongs to the modal or its background pill, never the gallery layout. */
  let progress = $state(0),
    phase = $state(''),
    error = $state(''),
    dragging = $state(false);
  /** Pausing affects transfer rather than ordinary modal dismissal. */
  let controller: AbortController | undefined;
  $effect(() => {
    if (open && file === null && pending === null && !busy) {
      albumId = initialAlbumId ?? '';
      visibility = $preferences.uploadVisibility;
    }
  });
  onMount(() => {
    pending = savedUpload();
    if (pending !== null)
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
  /** Removing a paused first file first cancels its multipart reservation; successors stay intact. */
  async function removeFile(selected: File): Promise<void> {
    if (selected === activeFile) return;
    try {
      if (selected === files[0] && pending !== null) {
        await cancelUpload();
        pending = null;
      }
      files = files.filter((entry) => entry !== selected);
      error = '';
      if (!busy)
        onstatus(
          files.length === 0
            ? null
            : { progress: 0, phase: 'Continue uploads', paused: true },
        );
    } catch (failure) {
      error = message(failure);
    }
  }
  /** Consume a live queue so appending or removing waiting files cannot shift the active upload. */
  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (file === null || policy === null || busy) return;
    const transferPolicy = policy;
    busy = true;
    error = '';
    progress = 0;
    phase = 'Preparing upload';
    onstatus({ progress, phase });
    controller = new AbortController();
    const signal = controller.signal;
    try {
      const completed = await runUploadQueue(
        () => files,
        async (entry, position) => {
          activeFile = entry;
          await uploadVideo(
            entry,
            albumId || null,
            visibility,
            (value, label) => {
              progress = value;
              const total = position - 1 + files.length;
              phase =
                total > 1
                  ? `${String(position)} of ${String(total)} · ${label}`
                  : label;
              onstatus({ progress, phase });
            },
            signal,
            pending,
            uploadKind(entry, transferPolicy),
          );
          pending = null;
          void oncomplete().catch((failure: unknown) => {
            onnotice(message(failure));
          });
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
      error = signal.aborted
        ? savedUpload() === null
          ? 'Upload queue paused.'
          : 'Upload paused. Choose the same file to continue.'
        : message(failure);
      pending = savedUpload();
      if (!open) onnotice('Upload interrupted. Open Upload to continue.');
    } finally {
      activeFile = null;
      busy = false;
      onstatus(
        pending === null && files.length === 0
          ? null
          : {
              progress,
              phase: pending === null ? 'Continue uploads' : 'Resume upload',
              paused: true,
            },
      );
    }
  }
  /** Discard the paused multipart session without discarding unrelated queued originals. */
  async function discard(): Promise<void> {
    try {
      await cancelUpload();
      pending = null;
      files.shift();
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
      {#if pending !== null && !busy}<div class="resume">
          <p>
            {file === null
              ? 'Continue your unfinished upload'
              : 'Upload paused'}
          </p>
          {#if file === null}<span
              >Choose the same original file to resume.</span
            >{/if}<Button
            variant="ghost"
            type="button"
            class="text-action"
            disabled={busy}
            onclick={discard}>Discard upload</Button
          >
        </div>{/if}
      {#if files.length > 0 && policy !== null}<UploadSelection
          {files}
          {policy}
          {activeFile}
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
            disabled={busy || pending !== null}
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
            disabled={busy || pending !== null}
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
            ><ArrowUp size={17} />{pending !== null
              ? 'Resume upload'
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
    border: 1px dashed #ffffff38;
    background: #0c0d1755;
    border-radius: 5px;
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
  .drop-zone:focus-within {
    border-color: #a5b4fc;
    background: #18192699;
  }
  .drop-zone:focus-within {
    outline: 2px solid #a5b4fc;
    outline-offset: 3px;
  }
  .drop-zone input {
    position: absolute;
    inset: 0;
    margin: 0;
    padding: 0;
    opacity: 0;
    cursor: pointer;
    width: 100%;
    height: 100%;
  }
  .drop-zone strong {
    color: #ebe8df;
    font-size: 15px;
    font-weight: 400;
  }
  .drop-zone span {
    font-size: 14px;
    color: #a7a7b2;
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
    color: #a9ad9e;
  }
  :global(#upload-album),
  :global(#upload-visibility) {
    margin: 0;
  }
  .optional {
    color: #858593;
    font-size: 12px;
    margin-left: 4px;
  }
  .limits {
    font-size: 12px;
    color: #92929e;
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
    border: 1px solid #ffffff1f;
    border-radius: 5px;
    margin-bottom: 16px;
    font-size: 14px;
  }
  .resume span {
    display: block;
    color: #a7a79e;
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
    accent-color: #d4ccba;
  }
  .transfer p {
    font-size: 12px;
    color: #a3a3b0;
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
