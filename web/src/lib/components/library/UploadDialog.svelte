<script lang="ts">
  import Button from '#lib/components/ui/button/button.svelte';
  import NativeSelect from '#lib/components/ui/native-select/native-select.svelte';
  /** Persistent upload controller; closing its dialog does not interrupt a transfer. */
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { preferences } from '#lib/features/preferences';
  import { Video, Check, ArrowUp, Pause, FolderCog } from '@lucide/svelte';
  import Modal from '../Modal.svelte';
  import type { Album, UploadPolicy } from '#lib/api/client';
  import { message } from '#lib/api/client';
  import { runUploadQueue } from '#lib/features/upload-queue';
  import {
    uploadVideo,
    savedUpload,
    cancelUpload,
    type PendingUpload,
    type TransferStatus,
  } from '#lib/features/upload';

  /** Controlled dialog state and page callbacks; authorization remains server-enforced. */
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
    /** Unknown during setup discovery; false leads to the server storage settings. */
    storageConfigured: boolean | null;
    /** Fresh album-page uploads start in this collection without replacing an existing draft. */
    initialAlbumId: string | null;
    onclose: () => void;
    oncomplete: () => Promise<void>;
    onstatus: (status: TransferStatus | null) => void;
    onnotice: (notice: string) => void;
    onalbums: () => void;
  } = $props();
  /** Local file selection is never used as the video's server-generated display name. */
  let files = $state<File[]>([]);
  /** The first remaining entry is also the file used when resuming a paused session. */
  const file = $derived(files[0] ?? null);
  /** Optional explicit album membership and safe default access policy. */
  let albumId = $state(''),
    visibility = $state('private');
  $effect(() => {
    if (open && file === null && pending === null && !busy) {
      albumId = initialAlbumId ?? '';
      visibility = $preferences.uploadVisibility;
    }
  });
  /** Current resumable session and transfer state survive dialog dismissal. */
  let pending = $state<PendingUpload | null>(null),
    busy = $state(false);
  /** Progress and errors belong to this transfer rather than the gallery's flow. */
  let progress = $state(0),
    phase = $state(''),
    error = $state('');
  /** Cancellation is explicit; ordinary dialog dismissal leaves this controller running. */
  let controller: AbortController | undefined;
  /** File field is reset after successful transfer so the same local file can be selected again. */
  let input = $state<HTMLInputElement>();
  /** Drag appearance is limited to the native file input's drop target. */
  let dragging = $state(false);
  onMount(() => {
    pending = savedUpload();
    if (pending !== null)
      onstatus({ progress: 0, phase: 'Resume upload', paused: true });
    return () => controller?.abort();
  });

  /** Validate the selected local file before opening a multipart upload session. */
  function selectFiles(selected: File[]): void {
    error = '';
    if (
      selected.length > 0 &&
      (policy === null ||
        selected.some(
          (file) =>
            file.size < 16 ||
            file.size > policy.max_upload_bytes ||
            !policy.allowed_extensions.includes(
              file.name.split('.').at(-1)?.toLowerCase() ?? '',
            ),
        ))
    ) {
      error =
        policy === null
          ? 'Upload settings are loading.'
          : 'Choose an enabled video format within the upload limit.';
      files = [];
      if (input !== undefined) input.value = '';
      return;
    }
    files = selected;
  }
  /** Accept a dropped file using the same validation and transfer path as the file picker. */
  function dropFile(event: DragEvent): void {
    event.preventDefault();
    dragging = false;
    if (!busy) selectFiles(Array.from(event.dataTransfer?.files ?? []));
  }
  /** Upload checksummed parts and keep the transfer alive when the modal closes. */
  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (file === null || policy === null || busy) return;
    busy = true;
    error = '';
    progress = 0;
    phase = 'Preparing upload';
    onstatus({ progress, phase });
    controller = new AbortController();
    const signal = controller.signal;
    const count = files.length;
    try {
      await runUploadQueue(
        files,
        async (entry, position, total) => {
          await uploadVideo(
            entry,
            albumId || null,
            visibility,
            (value, label) => {
              progress = value;
              phase =
                total > 1
                  ? `${String(position)} of ${String(total)} · ${label}`
                  : label;
              onstatus({ progress: value, phase });
            },
            signal,
            pending,
          );
          pending = null;
          void oncomplete().catch((failure: unknown) => {
            onnotice(message(failure));
          });
        },
        signal,
      );
      if (input !== undefined) input.value = '';
      onclose();
      onnotice(
        count === 1 ? 'Video uploaded.' : `${String(count)} videos uploaded.`,
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
  /** Explicitly discard the saved session rather than conflating close with cancel. */
  async function discard(): Promise<void> {
    try {
      await cancelUpload();
      pending = null;
      files.shift();
      onstatus(null);
      error = '';
      await oncomplete();
    } catch (failure) {
      error = message(failure);
    }
  }
</script>

<Modal {open} title="Upload videos" {onclose} closeLabel="Close upload">
  {#if storageConfigured === null}
    <p class="storage-setup muted" role="status">Loading upload settings…</p>
  {:else if !storageConfigured}
    <div class="storage-setup">
      <p>Connect Amazon S3 to start uploading.</p>
      <Button
        href={resolve('/settings/storage')}
        class="button primary"
        onclick={onclose}>Set up storage</Button
      >
    </div>
  {:else}
    <form onsubmit={submit}>
      {#if pending !== null}<div class="resume">
          <p>Continue your unfinished upload</p>
          <span>Choose the same original file to resume.</span>
          <Button
            variant="ghost"
            type="button"
            class="text-action"
            disabled={busy}
            onclick={discard}>Discard upload</Button
          >
        </div>{/if}
      <label class="drop-zone" class:dragging class:selected={file !== null}>
        <input
          bind:this={input}
          type="file"
          multiple
          accept={policy?.allowed_extensions
            .map((extension) => '.' + extension)
            .join(',') ?? 'video/*'}
          aria-label="Video files"
          disabled={busy || policy === null}
          onchange={(event) => {
            selectFiles(Array.from(event.currentTarget.files ?? []));
          }}
          ondragover={(event) => {
            event.preventDefault();
            dragging = true;
          }}
          ondragleave={() => (dragging = false)}
          ondrop={dropFile}
        />
        {#if file !== null}<Check size={25} strokeWidth={1.25} />{:else}<Video
            size={28}
            strokeWidth={1.25}
          />{/if}
        <strong
          >{busy
            ? file?.name
            : files.length > 1
              ? `${String(files.length)} videos selected`
              : (file?.name ?? 'Drop videos here')}</strong
        >
        <span
          >{file !== null
            ? `${(files.reduce((bytes, file) => bytes + file.size, 0) / 1_048_576).toFixed(1)} MiB · ${busy ? 'Uploading in order' : 'Click to change selection'}`
            : 'or click to choose files'}</span
        >
        <small
          >{policy === null
            ? 'Loading upload settings…'
            : 'Video · up to ' +
              String(policy.max_upload_bytes / 1_000_000_000) +
              ' GB'}</small
        >
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
              : files.length > 1
                ? 'Upload videos'
                : 'Upload video'}</Button
          >{/if}
      </footer>
    </form>
  {/if}
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
    background: #0c0d17;
    border-radius: 5px;
    text-align: center;
    padding: 24px;
    margin-bottom: 24px;
  }
  .drop-zone:hover,
  .dragging,
  .drop-zone:focus-within {
    border-color: #a5b4fc;
    background: #181926;
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
    overflow-wrap: anywhere;
  }
  .drop-zone span,
  .drop-zone small {
    font-size: 13px;
    color: #a7a79e;
  }
  .drop-zone small {
    margin-top: 5px;
    font-size: 12px;
    color: #777a72;
  }
  .options {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  .album-label {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 40px;
  }
  .album-action {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    margin: 0;
    color: #a9ad9e;
  }
  :global(#upload-album),
  :global(#upload-visibility) {
    margin: 0;
  }
  .optional {
    color: #85877f;
    font-size: 12px;
    margin-left: 4px;
  }

  footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
  }
  .resume {
    padding: 14px;
    border: 1px solid #ffffff1f;
    border-radius: 4px;
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
    margin-bottom: 22px;
  }
  .transfer div {
    display: flex;
    justify-content: space-between;
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
    color: #a3a59b;
    margin-top: 10px;
  }
  @media (max-width: 380px) {
    .options {
      grid-template-columns: 1fr;
      gap: 0;
    }
  }
</style>
