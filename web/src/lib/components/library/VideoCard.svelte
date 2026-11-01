<script lang="ts">
  import Button from '#lib/components/ui/button/button.svelte';
  /** Cropped cover with hover, keyboard, and touch actions; no video bytes are loaded. */
  import { Play, Ellipsis, Video as VideoIcon } from '@lucide/svelte';
  import type { Video } from '#lib/api/client';
  import { shareValue } from '#lib/features/sharing';
  import CopyButton from '../CopyButton.svelte';
  /** Cover selection and administrator actions remain independent click targets. */
  let {
    video,
    onopen,
    onmanage,
  }: {
    video: Video;
    onopen: () => void;
    onmanage?: ((video: Video) => void) | undefined;
  } = $props();
  /** Touch reveals actions first; the next tap opens the player. */
  let revealed = $state(false),
    failedPoster = $state(false);
  /** Only completely published media offers anonymous sharing. */
  const shareable = $derived(
    video.visibility === 'public' && video.state === 'published',
  );
  /** Incomplete uploads expose status rather than opening an unplayable screen. */
  const ready = $derived(
    ['ready', 'published', 'publishing', 'revoking'].includes(video.state),
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
    if (ready) onopen();
  }
</script>

<div class="card" class:revealed role="group" aria-label={video.name}>
  <button
    class="cover"
    aria-label={'Open video ' + video.name}
    onclick={open}
    disabled={!ready}
  >
    {#if video.poster_url !== null && !failedPoster && ready}<img
        src={video.poster_url}
        alt=""
        loading="lazy"
        decoding="async"
        onerror={() => (failedPoster = true)}
      />
    {:else}<span class="placeholder"
        ><VideoIcon size={28} />{#if !ready}<span
            >{video.state === 'failed'
              ? 'Upload failed'
              : 'Processing video…'}</span
          >{/if}</span
      >{/if}
    <span class="play controls"
      ><Play size={24} fill="currentColor" strokeWidth={1.5} /></span
    >
  </button>
  <div class="actions controls">
    {#if shareable}<CopyButton
        value={shareValue(video, 'media')}
        label="Copy video link"
        icon="link"
      /><CopyButton
        value={shareValue(video, 'iframe')}
        label="Copy iframe"
        icon="code"
      />{/if}
    {#if onmanage}<Button
        variant="ghost"
        size="icon"
        type="submit"
        class="icon-button"
        aria-label="Video options"
        title="Share or edit video"
        disabled={!['ready', 'published', 'failed'].includes(video.state)}
        onclick={() => {
          onmanage(video);
        }}><Ellipsis size={21} /></Button
      >{/if}
  </div>
  <div class="details controls">
    <span>{video.visibility === 'private' ? 'Private' : ''}</span><span
      >{duration}</span
    >
  </div>
</div>

<style>
  .card {
    position: relative;
    height: 100%;
    overflow: hidden;
    background: #14151e;
    isolation: isolate;
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
    font-size: 14px;
  }
  .card:focus-within {
    outline: 2px solid #a5b4fc;
    outline-offset: 3px;
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
