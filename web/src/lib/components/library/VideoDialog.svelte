<script lang="ts">
  import Button from '#lib/components/ui/button/button.svelte';
  import Input from '#lib/components/ui/input/input.svelte';
  import NativeSelect from '#lib/components/ui/native-select/native-select.svelte';
  /** The video's own menu contains sharing, metadata, privacy, and deletion. */
  import { onMount } from 'svelte';
  import { Globe } from '@lucide/svelte';
  import Modal from '../Modal.svelte';
  import SharePanel from './SharePanel.svelte';
  import {
    api,
    apiData,
    message,
    type Video,
    type Album,
  } from '#lib/api/client';
  import { waitForJob } from '#lib/features/jobs';

  /** One selected video, explicit metadata refreshes, and its original gallery context. */
  let {
    video,
    albums,
    onclose,
    oncomplete,
    onnotice,
    onupdate,
  }: {
    video: Video;
    albums: Album[];
    onclose: () => void;
    oncomplete: () => Promise<void>;
    onnotice: (notice: string) => void;
    onupdate: (video: Video) => void;
  } = $props();
  /** Sharing is the primary action; editing remains one tab away in the same menu. */
  let tab = $state<'share' | 'edit'>('share');
  /** Draft metadata is reset only after the server supplies a new selected video. */
  let name = $state(''),
    albumId = $state(''),
    visibility = $state('private');
  /** A destructive operation has a separate confirmation view. */
  let saving = $state(false),
    error = $state(''),
    deleting = $state(false),
    publishing = $state(false);
  /** Only fully published public content exposes downloadable links and iframe markup. */
  const shareable = $derived(
    video.visibility === 'public' && video.state === 'published',
  );
  /** Closing the menu stops browser polling without cancelling durable publication. */
  const controller = new AbortController();
  onMount(() => () => {
    controller.abort();
  });
  $effect(() => {
    name = video.name;
    albumId = video.album_id ?? '';
    visibility = video.visibility;
  });

  /** Save edited metadata without changing any canonical playback URL. */
  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    saving = true;
    error = '';
    try {
      await apiData(
        api.PATCH('/v1/admin/videos/{id}', {
          params: { path: { id: video.id } },
          body: { name, album_id: albumId || null, visibility },
        }),
      );
      onclose();
      onnotice('Changes saved.');
      await oncomplete();
    } catch (failure) {
      error = message(failure);
    } finally {
      saving = false;
    }
  }
  /** Publish a private video and reveal sharing only after its durable copy job completes. */
  async function publish(): Promise<void> {
    publishing = true;
    error = '';
    try {
      const accepted = await apiData(
        api.PATCH('/v1/admin/videos/{id}', {
          params: { path: { id: video.id } },
          signal: controller.signal,
          body: {
            name: video.name,
            album_id: video.album_id,
            visibility: 'public',
          },
        }),
      );
      if (accepted.job_id !== null)
        await waitForJob(accepted.job_id, controller.signal);
      const updated = await apiData(
        api.GET('/v1/videos/{token}', {
          params: { path: { token: video.token } },
          signal: controller.signal,
        }),
      );
      onupdate(updated);
      await oncomplete();
    } catch (failure) {
      if (!controller.signal.aborted) error = message(failure);
    } finally {
      publishing = false;
    }
  }
  /** Remove source media and withdraw public copies after an explicit confirmation. */
  async function remove(): Promise<void> {
    saving = true;
    error = '';
    try {
      await apiData(
        api.DELETE('/v1/admin/videos/{id}', {
          params: { path: { id: video.id } },
        }),
      );
      onclose();
      onnotice('Video deletion started.');
      await oncomplete();
    } catch (failure) {
      error = message(failure);
    } finally {
      saving = false;
    }
  }
</script>

<Modal
  open
  title={deleting ? 'Delete this video?' : 'Video options'}
  {onclose}
  closeLabel="Close video options"
>
  {#if deleting}
    <p class="delete-name">{video.name}</p>
    <p class="hint mb-6">
      The video and its public copies will be removed. This cannot be undone.
    </p>
    {#if error}<p class="error mb-4" role="alert">{error}</p>{/if}
    <footer>
      <Button
        variant="ghost"
        type="submit"
        class="text-action"
        onclick={() => (deleting = false)}>Keep video</Button
      ><Button
        variant="destructive"
        type="submit"
        class="button danger"
        disabled={saving}
        onclick={remove}>Delete video</Button
      >
    </footer>
  {:else}
    <div class="tabs" aria-label="Video actions">
      <button
        aria-pressed={tab === 'share'}
        class:active={tab === 'share'}
        onclick={() => {
          tab = 'share';
          error = '';
        }}
        disabled={publishing}>Share</button
      >
      <button
        aria-pressed={tab === 'edit'}
        class:active={tab === 'edit'}
        onclick={() => {
          tab = 'edit';
          error = '';
        }}
        disabled={publishing}>Edit</button
      >
    </div>
    {#if tab === 'share'}
      {#if shareable}<SharePanel {video} />
      {:else}<div class="private-message">
          <Globe size={26} strokeWidth={1.25} />
          <h3 class="serif">
            {publishing || video.state === 'publishing'
              ? 'Preparing your links…'
              : 'Share this video'}
          </h3>
          <p class="hint">
            {publishing || video.state === 'publishing'
              ? 'Publication continues in the background if you close this window.'
              : 'This video is private. Make it public to share a direct video link or iframe.'}
          </p>
          <Button
            variant="default"
            type="button"
            class="button primary"
            onclick={publish}
            disabled={publishing || video.state !== 'ready'}
            >{publishing ? 'Publishing…' : 'Make public'}</Button
          >
        </div>{/if}
    {:else}
      {#if video.last_error !== null}<p class="error mb-4" role="alert">
          {video.last_error}
        </p>{/if}
      <form onsubmit={save}>
        <label class="field"
          >Display name<Input
            required
            maxlength={200}
            bind:value={name}
          /></label
        >
        <div class="options">
          <label class="field"
            >Album<NativeSelect bind:value={albumId}
              ><option value="">No album</option
              >{#each albums as album (album.id)}<option value={album.id}
                  >{album.name}</option
                >{/each}</NativeSelect
            ></label
          >
          <label class="field"
            >Visibility<NativeSelect bind:value={visibility}
              ><option value="private">Private</option><option value="public"
                >Public</option
              ></NativeSelect
            ></label
          >
        </div>

        <footer>
          <Button
            variant="destructive"
            class="text-action danger-text"
            type="button"
            onclick={() => {
              deleting = true;
              error = '';
            }}>Delete video</Button
          ><Button
            variant="default"
            type="submit"
            class="button primary"
            disabled={saving || video.state === 'failed'}>Save changes</Button
          >
        </footer>
      </form>
    {/if}
    {#if error}<p class="error mt-4" role="alert">{error}</p>{/if}
  {/if}
</Modal>

<style>
  .tabs {
    display: flex;
    gap: 24px;
    border-bottom: 1px solid #ffffff1f;
    margin-bottom: 18px;
  }
  .tabs button {
    min-height: 44px;
    color: #96988f;
    font-size: 15px;
    border-bottom: 1px solid transparent;
    margin-bottom: -1px;
    padding: 0 0 12px;
  }
  .tabs .active {
    color: #ebe8df;
    border-bottom-color: #d3cbb9;
  }
  .options {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
  }
  .hint {
    color: #a0a397;
    font-size: 13px;
    line-height: 1.7;
    margin-bottom: 24px;
  }
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }
  .delete-name {
    font-size: 15px;
    color: #e1ded4;
    overflow-wrap: anywhere;
    margin-bottom: 12px;
  }
  :global(.danger-text) {
    color: #d2a39a;
  }
  .private-message {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 24px 4px 10px;
  }
  .private-message :global(svg) {
    color: #a6aa9b;
  }
  .private-message h3 {
    font-size: 26px;
    margin: 14px 0 10px;
  }
  @media (max-width: 380px) {
    .options {
      grid-template-columns: 1fr;
      gap: 0;
    }
  }
</style>
