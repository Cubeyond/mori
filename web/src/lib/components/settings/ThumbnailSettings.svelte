<script lang="ts">
  /** Browser-local thumbnail translucency is independent of wallpaper opacity and original viewing. */
  import { onDestroy } from 'svelte';
  import { Check } from '@lucide/svelte';
  import {
    preferences,
    savePreferences,
    DEFAULT_THUMBNAIL_OPACITY,
  } from '#lib/features/preferences';
  import OpacityControl from './OpacityControl.svelte';
  /** Feedback follows the edited control and never changes the settings layout. */
  let saved = $state(false),
    error = $state(''),
    changed = $state<'enabled' | 'opacity'>('enabled');
  /** One acknowledgement timer covers repeated keyboard or pointer adjustments. */
  let timer: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => {
    if (timer !== undefined) clearTimeout(timer);
  });
  /** Persist the latest local preview and report storage failure without losing the live choice. */
  function persist(field: 'enabled' | 'opacity'): void {
    error = '';
    changed = field;
    saved = savePreferences($preferences);
    if (timer !== undefined) clearTimeout(timer);
    if (saved) timer = setTimeout(() => (saved = false), 2400);
    else error = 'Applied for this visit. Browser storage is unavailable.';
  }
</script>

<div class="setting-row">
  <div class="setting-content">
    <span id="thumbnail-translucency-label" class="setting-label"
      >Translucent thumbnails</span
    >
    <div class="toggle-control">
      <div class="switch">
        <input
          id="thumbnail-translucency"
          type="checkbox"
          role="switch"
          aria-labelledby="thumbnail-translucency-label"
          checked={$preferences.translucentThumbnails}
          onchange={(event) => {
            preferences.update((value) => ({
              ...value,
              translucentThumbnails: event.currentTarget.checked,
            }));
            persist('enabled');
          }}
        />
        <span class="track" aria-hidden="true"></span>
      </div>
      <span class="feedback" aria-hidden="true">
        {#if saved && changed === 'enabled'}<Check size={20} />{/if}
      </span>
    </div>
  </div>
</div>
{#if $preferences.translucentThumbnails}<div
    class="setting-row"
    id="thumbnail-opacity-row"
  >
    <div class="setting-content">
      <span class="setting-label">Thumbnail opacity</span>
      <OpacityControl
        id="thumbnail-opacity"
        label="Thumbnail opacity"
        value={$preferences.thumbnailOpacity}
        defaultValue={DEFAULT_THUMBNAIL_OPACITY}
        saved={saved && changed === 'opacity'}
        oninput={(thumbnailOpacity: number) => {
          preferences.update((value) => ({ ...value, thumbnailOpacity }));
        }}
        oncommit={() => {
          persist('opacity');
        }}
      />
    </div>
  </div>{/if}
<span class="sr-only" role="status" aria-live="polite"
  >{saved ? 'Saved' : ''}</span
>
{#if error}<p class="storage-error" role="alert">{error}</p>{/if}

<style>
  .toggle-control {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 44px;
  }
  .switch {
    position: relative;
    display: grid;
    align-items: center;
    width: 44px;
    height: 44px;
  }
  input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }
  .track {
    width: 42px;
    height: 24px;
    padding: 3px;
    border-radius: 14px;
    background: var(--input);
    pointer-events: none;
  }
  .track::after {
    content: '';
    display: block;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--foreground);
  }
  input:checked + .track {
    background: var(--selection);
  }
  input:checked + .track::after {
    transform: translateX(18px);
    background: var(--selection-foreground);
  }
  input:focus-visible + .track {
    outline: 2px solid var(--ring);
    outline-offset: 4px;
  }
  .feedback {
    width: 24px;
    color: var(--success);
  }
  .storage-error {
    margin: 0;
    padding: 0 24px 20px;
    color: var(--warning);
    font-size: 14px;
  }
</style>
