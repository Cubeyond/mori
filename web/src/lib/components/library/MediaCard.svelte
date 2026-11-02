<script lang="ts">
  import Button from '#lib/components/ui/button/button.svelte';
  /** Cropped cover with hover, keyboard, and touch actions; no video bytes are loaded. */
  import {
    Play,
    Ellipsis,
    Video as VideoIcon,
    Image as ImageIcon,
    Expand,
  } from '@lucide/svelte';
  import type { MediaItem } from '#lib/api/client';
  import { shareValue } from '#lib/features/sharing';
  import { preferences } from '#lib/features/preferences';
  import { mediaStatus } from '#lib/features/media-status';
  import CopyButton from '../CopyButton.svelte';
  /** Cover selection and administrator actions remain independent click targets. */
  let {
    video,
    onopen,
    onmanage,
  }: {
    video: MediaItem;
    onopen: () => void;
    onmanage?: ((video: MediaItem) => void) | undefined;
  } = $props();
  /** Touch reveals actions first; the next tap opens the player. */
  let revealed = $state(false),
    failedPoster = $state(false);
  /** Only completely published media offers anonymous sharing. */
  const shareable = $derived(
    video.visibility === 'public' && video.state === 'published',
  );
  /** Operational state is independent of deletion's forced-private access policy. */
  const status = $derived(mediaStatus(video));
  /** Fade only visual pixels, leaving actions, focus, and lifecycle labels fully visible. */
  const thumbnailOpacity = $derived(
    $preferences.translucentThumbnails ? $preferences.thumbnailOpacity : 1,
  );
  /** Format metadata duration without loading the original video. */
  const duration = $derived(
    String(Math.floor(video.duration / 60)).padStart(2, '0') +
      ':' +
      String(Math.floor(video.duration % 60)).padStart(2, '0'),
  );
  /** Keep the first touch available for share and management controls. */
  function open(event: MouseEvent): void {
    if ('pointerType' in event && event.pointerType === 'touch' && !revealed) {
      revealed = true;
      return;
    }
    if (status.ready) onopen();
  }
</script>

<div class="card" class:revealed role="group" aria-label={video.name}>
  <button
    class="cover"
    aria-label={'Open ' + video.kind + ' ' + video.name}
    onclick={open}
    disabled={!status.ready}
  >
    {#if video.poster_url !== null && !failedPoster && status.ready}<img
        src={video.poster_url}
        alt=""
        loading="lazy"
        decoding="async"
        style:opacity={thumbnailOpacity}
        onerror={() => (failedPoster = true)}
      />
    {:else}<span class="placeholder" style:opacity={thumbnailOpacity}>
        {#if video.kind === 'image'}<ImageIcon size={28} />{:else}<VideoIcon
            size={28}
          />{/if}
      </span>{/if}
    {#if !status.ready}<span class="processing-status" role="status"
        >{status.label}</span
      >{/if}
    <span class="play controls"
      >{#if video.kind === 'image'}<Expand size={24} />{:else}<Play
          size={24}
          fill="currentColor"
          strokeWidth={1.5}
        />{/if}</span
    >
  </button>
  <div class="actions controls">
    {#if shareable}<CopyButton
        value={shareValue(video, 'media')}
        label={video.kind === 'image' ? 'Copy image link' : 'Copy video link'}
        icon="link"
      />{#if video.kind === 'video'}<CopyButton
          value={shareValue(video, 'iframe')}
          label="Copy iframe"
          icon="code"
        />{/if}{/if}
    {#if onmanage}<Button
        variant="ghost"
        size="icon"
        type="submit"
        class="icon-button"
        aria-label="Media options"
        title="Share or edit"
        disabled={!['ready', 'published', 'failed'].includes(video.state)}
        onclick={() => {
          onmanage(video);
        }}><Ellipsis size={21} /></Button
      >{/if}
  </div>
  <div class="details controls">
    <span>{status.visibility}</span>{#if video.kind === 'video'}<span
        >{duration}</span
      >{/if}
  </div>
</div>

<style>
  .card {
    position: relative;
    height: 100%;
    overflow: hidden;
    background: transparent;
    isolation: isolate;
    border-radius: 5px;
  }
  .cover {
    display: block;
    width: 100%;
    height: 100%;
    padding: 0;
    border: 0;
    background: transparent;
    color: #f4f4f5;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .placeholder {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    height: 100%;
    color: #858593;
    background: #14151e;
    font-size: 14px;
  }
  .processing-status {
    position: absolute;
    top: calc(50% + 24px);
    inset-inline: 8px;
    font-size: 14px;
    color: #e4e4eb;
    text-align: center;
    text-shadow: 0 1px 3px #000;
    pointer-events: none;
  }
  .card::after {
    content: '';
    position: absolute;
    inset: 0;
    z-index: 2;
    border-radius: inherit;
    pointer-events: none;
  }
  .card:has(:global(:focus-visible))::after {
    box-shadow: inset 0 0 0 2px #a5b4fc;
  }
  @media (hover: hover) {
    .card:hover::after {
      box-shadow: inset 0 0 0 2px #a5b4fc;
    }
  }
  .controls {
    opacity: 0;
    transition: opacity 140ms;
    pointer-events: none;
  }
  .card:hover .controls,
  .card:focus-within .controls,
  .revealed .controls {
    opacity: 1;
    pointer-events: auto;
  }
  .actions {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    padding: 3px;
    display: flex;
    justify-content: flex-end;
    background: linear-gradient(#0009, transparent);
  }
  .play {
    display: grid;
    place-items: center;
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 48px;
    height: 48px;
    background: #0b0b1477;
    border-radius: 50%;
  }
  .details {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    display: flex;
    justify-content: space-between;
    padding: 24px 12px 10px;
    background: linear-gradient(transparent, #0009);
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  .actions :global(.icon-button) {
    color: #fff;
  }

  @media (hover: none) {
    .card:hover:not(.revealed):not(:focus-within) .controls {
      opacity: 0;
      pointer-events: none;
    }
  }
</style>
