<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: Elastic-2.0 -->

<script lang="ts">
  /** Background controls preview immediately while protecting the last saved image on failure. */
  import { onDestroy } from 'svelte';
  import { ImagePlus, Trash, Check } from '@lucide/svelte';
  import Button from '#lib/components/ui/button/button.svelte';
  import OpacityControl from './OpacityControl.svelte';
  import {
    background,
    saveBackground,
    removeBackground,
  } from '#lib/features/background';
  import { preferences, savePreferences } from '#lib/features/preferences';
  import { message } from '#lib/api/client';
  /** Hidden native picker preserves normal file permissions and keyboard activation. */
  let picker: HTMLInputElement;
  /** Saving an image does not disable navigation or change the current backdrop prematurely. */
  let busy = $state(false),
    error = $state(''),
    saved = $state(false);
  /** Success replaces the relevant action icon without adding a tag or shifting the row. */
  let changed = $state<'image' | 'opacity'>('image');
  /** A dismissed settings page cancels an unfinished local decode. */
  let controller: AbortController | undefined;
  /** One short acknowledgement replaces previous save feedback. */
  let timer: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => {
    controller?.abort();
    if (timer !== undefined) clearTimeout(timer);
  });
  /** Acknowledge a completed write beside these controls. */
  function acknowledge(field: 'image' | 'opacity'): void {
    changed = field;
    saved = true;
    if (timer !== undefined) clearTimeout(timer);
    timer = setTimeout(() => {
      saved = false;
    }, 2400);
  }
  /** Decode and persist only this browser's selected background. */
  async function select(
    event: Event & { currentTarget: HTMLInputElement },
  ): Promise<void> {
    const file = event.currentTarget.files?.[0];
    event.currentTarget.value = '';
    if (file === undefined) return;
    controller?.abort();
    controller = new AbortController();
    busy = true;
    saved = false;
    error = '';
    try {
      await saveBackground(file, controller.signal);
      acknowledge('image');
    } catch (failure) {
      if (!controller.signal.aborted) error = message(failure);
    } finally {
      busy = false;
    }
  }
  /** Delete both the stored image and its live local URL. */
  async function remove(): Promise<void> {
    busy = true;
    saved = false;
    error = '';
    try {
      await removeBackground();
      acknowledge('image');
    } catch (failure) {
      error = message(failure);
    } finally {
      busy = false;
    }
  }
  /** Persist opacity after a slider gesture, retaining smooth local previews during dragging. */
  function persistOpacity(): void {
    error = '';
    saved = false;
    if (savePreferences($preferences)) acknowledge('opacity');
    else
      error = 'Opacity applied for this visit. Browser storage is unavailable.';
  }
</script>

<div class="setting-row">
  <div class="setting-content">
    <span id="background-image-label" class="setting-label"
      >Background image</span
    >
    <div class="background-controls" class:has-image={$background !== null}>
      {#if $background !== null}<img
          class="thumbnail"
          src={$background}
          alt="Current background"
        />{/if}
      <input
        bind:this={picker}
        id="background-image"
        hidden
        aria-labelledby="background-image-label"
        type="file"
        accept="image/*"
        onchange={select}
        disabled={busy}
      />
      <Button
        variant="outline"
        type="button"
        disabled={busy}
        onclick={() => {
          picker.click();
        }}
        >{#if saved && changed === 'image'}<Check
            size={18}
            class="text-(--success)"
          />{:else}<ImagePlus size={18} />{/if}{busy
          ? 'Saving…'
          : $background === null
            ? 'Choose image'
            : 'Change'}</Button
      >
      {#if $background !== null}<Button
          variant="ghost"
          size="icon"
          type="button"
          aria-label="Remove background image"
          disabled={busy}
          onclick={remove}><Trash size={19} /></Button
        >{/if}
      <span class="sr-only" role="status" aria-live="polite"
        >{saved ? 'Saved' : ''}</span
      >
    </div>
  </div>
</div>
{#if $background !== null}<div class="setting-row">
    <div class="setting-content">
      <span class="setting-label">Background opacity</span>
      <OpacityControl
        id="background-opacity"
        label="Background opacity"
        value={$preferences.backgroundOpacity}
        defaultValue={0.1}
        saved={saved && changed === 'opacity'}
        oninput={(backgroundOpacity: number) => {
          preferences.update((value) => ({ ...value, backgroundOpacity }));
        }}
        oncommit={persistOpacity}
      />
    </div>
  </div>{/if}
{#if error}<p class="background-error" role="alert">{error}</p>{/if}

<style>
  .background-controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 12px;
    margin-top: 0;
  }
  .thumbnail {
    width: 80px;
    height: 52px;
    object-fit: cover;
    border: 1px solid var(--border);
    border-radius: 6px;
  }
  .background-error {
    color: var(--destructive);
    font-size: 15px;
    margin-block: 16px;
  }
  @media (max-width: 600px) {
    .background-controls.has-image {
      display: grid;
      grid-template-columns: 56px minmax(0, 1fr) 40px;
      gap: 8px;
    }
    .thumbnail {
      width: 56px;
      height: 44px;
    }
  }
</style>
