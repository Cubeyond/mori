<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: Elastic-2.0 -->

<script lang="ts">
  /** Canonical album editor and name-confirmed deletion with an explicit media option. */
  import Input from '#lib/components/ui/input/input.svelte';
  import AlbumFields from './AlbumFields.svelte';
  import Button from '#lib/components/ui/button/button.svelte';
  import { untrack } from 'svelte';
  import Modal from '../Modal.svelte';
  import { api, apiData, message, type Album } from '#lib/api/client';
  /** Every caller uses the same form and destructive confirmation contract. */
  let {
    album = null,
    removing = false,
    onclose,
    oncomplete,
  }: {
    album?: Album | null;
    removing?: boolean;
    onclose: () => void;
    oncomplete: () => Promise<void>;
  } = $props();
  /** Display name and address are independent editable fields. */
  let name = $state(untrack(() => album?.name ?? '')),
    slug = $state(untrack(() => album?.slug ?? '')),
    visibility = $state(untrack(() => album?.visibility ?? 'private')),
    description = $state(untrack(() => album?.description ?? ''));
  /** Retaining videos is the default; both removal modes require the current exact name. */
  let confirmation = $state(''),
    deletionMode = $state('keep');
  /** A single choice determines the existing deletion API's media flag. */
  const deleteMedia = $derived(deletionMode === 'delete');
  /** Confirmation is also independently enforced by Rust. */
  const confirmed = $derived(album !== null && confirmation === album.name);
  /** Keep failures in the form without losing any draft input. */
  let busy = $state(false),
    error = $state('');
  /** Commit one mutation before refreshing the surrounding collection. */
  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy || (removing && !confirmed)) return;
    busy = true;
    error = '';
    try {
      if (album === null) {
        await apiData(
          api.POST('/v1/admin/albums', {
            body: { name, slug, visibility, description },
          }),
        );
      } else if (removing) {
        await apiData(
          api.DELETE('/v1/admin/albums/{id}', {
            params: { path: { id: album.id } },
            body: { confirmation, delete_media: deleteMedia },
          }),
        );
      } else {
        await apiData(
          api.PATCH('/v1/admin/albums/{id}', {
            params: { path: { id: album.id } },
            body: { name, slug, visibility, description },
          }),
        );
      }
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
  title={removing
    ? 'Delete album'
    : album === null
      ? 'New album'
      : 'Edit album'}
  {onclose}
>
  <form onsubmit={save}>
    {#if removing}
      <fieldset class="deletion-options" disabled={busy}>
        <legend class="sr-only">Media handling</legend>
        <div class="deletion-choices">
          <label class="deletion-choice">
            <input
              type="radio"
              name="album-video-action"
              value="keep"
              bind:group={deletionMode}
            />
            <span>Keep in Gallery</span>
          </label>
          <label class="deletion-choice destructive">
            <input
              type="radio"
              name="album-video-action"
              value="delete"
              bind:group={deletionMode}
            />
            <span>Delete permanently</span>
          </label>
        </div>
      </fieldset>
      <div class="deletion-confirmation">
        <label for="album-delete-confirmation"
          >Type <strong translate="no">{album?.name}</strong> to confirm</label
        >
        <Input
          id="album-delete-confirmation"
          required
          bind:value={confirmation}
          autocomplete="off"
          autocapitalize="none"
          autocorrect="off"
          spellcheck={false}
          translate="no"
          disabled={busy}
        />
      </div>
    {:else}
      <AlbumFields
        bind:name
        bind:slug
        bind:visibility
        bind:description
        disabled={busy}
        suggestSlug={album === null}
      />
      {#if album !== null && slug !== album.slug}<p
          class="muted mb-6 text-sm leading-relaxed"
        >
          Changing the slug changes album and member sharing links. Previous
          album addresses will stop working.
        </p>{/if}
    {/if}
    {#if error}<p class="error mb-4" role="alert">{error}</p>{/if}
    <div class="form-actions">
      <Button variant="outline" class="button" type="button" onclick={onclose}
        >Cancel</Button
      >
      <Button
        variant={removing ? 'destructive' : 'default'}
        type="submit"
        class="button"
        disabled={busy || (removing && !confirmed)}
        >{busy
          ? removing
            ? 'Deleting…'
            : 'Saving…'
          : removing
            ? deleteMedia
              ? 'Delete album and media'
              : 'Delete album'
            : album === null
              ? 'Create album'
              : 'Save changes'}</Button
      >
    </div>
  </form>
</Modal>

<style>
  .deletion-options {
    border: 0;
    padding: 0;
    margin: 0;
    min-width: 0;
  }
  .deletion-choices {
    display: grid;
    gap: 4px;
  }
  .deletion-choice {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
    padding: 8px 0;
    font-size: 15px;
    line-height: 24px;
    cursor: pointer;
  }
  .deletion-choice.destructive:has(input:checked) {
    color: var(--destructive);
  }
  .deletion-choice input {
    width: 18px;
    height: 18px;
    margin: 0;
    accent-color: var(--foreground);
    flex-shrink: 0;
  }
  .destructive input {
    accent-color: var(--destructive);
  }
  .deletion-confirmation {
    display: grid;
    gap: 10px;
    margin-top: 28px;
  }
  .deletion-confirmation label {
    line-height: 24px;
    overflow-wrap: anywhere;
  }
  .form-actions {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 12px;
    margin-top: 28px;
  }
</style>
