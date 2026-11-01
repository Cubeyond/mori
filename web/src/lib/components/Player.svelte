<script lang="ts">
  import Button from '#lib/components/ui/button/button.svelte';
  /** Accessible, lazy-loaded player with hover, focus, and touch controls. */

  import { onMount } from 'svelte';
  import {
    Play,
    Pause,
    Maximize,
    Volume2,
    VolumeX,
    Ellipsis,
    Download,
  } from '@lucide/svelte';
  import type { Video } from '#lib/api/client';
  import { buttonVariants } from '#lib/components/ui/button/variants';
  import { shareValue } from '#lib/features/sharing';
  import CopyButton from './CopyButton.svelte';
  /** Video metadata and optional administrator inspector callback. */
  let {
    video,
    onmanage,
  }: { video: Video; onmanage?: ((video: Video) => void) | undefined } =
    $props();
  /** Bound native video element; sources are attached only after explicit playback intent. */
  let media: HTMLVideoElement;
  /** Player surface used for fullscreen and focus containment. */
  let frame: HTMLDivElement;
  /** Whether an explicit playback action has authorized loading media bytes. */
  let activated = $state(false);
  /** Whether touch interaction has revealed the player controls. */
  let opened = $state(false);
  /** Playback state synchronized with the native video element. */
  let paused = $state(true);
  /** Current playhead position in seconds. */
  let time = $state(0);
  /** Duration reported by the browser after media loading. */
  let duration = $state(0);
  /** Audio mute state synchronized with the video element. */
  let muted = $state(false);
  /** Accessible feedback for sharing or administrator actions. */
  let notice = $state('');
  /** Playback failure presented without exposing private storage details. */
  let failure = $state('');
  /** Sharing controls are available only for fully published public videos. */
  const shareable = $derived(
    video.visibility === 'public' && video.state === 'published',
  );
  /** Format a finite media timestamp as a compact minute-and-second label. */
  const clock = (n: number) =>
    `${String(Math.floor((Number.isFinite(n) ? n : 0) / 60)).padStart(2, '0')}:${String(Math.floor((Number.isFinite(n) ? n : 0) % 60)).padStart(2, '0')}`;
  /** Start or pause explicit playback and coordinate the single-active-video policy. */
  async function toggle() {
    failure = '';
    activated = true;
    opened = true;
    await new Promise<void>((resolve) =>
      requestAnimationFrame(() => {
        resolve();
      }),
    );
    try {
      if (media.paused) {
        window.dispatchEvent(
          new CustomEvent('mori:play', { detail: video.id }),
        );
        await media.play();
      } else media.pause();
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') return;
      failure =
        'This video could not be played. Check your connection and browser codec support.';
    }
  }
  onMount(() => {
    window.addEventListener('mori:play', coordinate);
    return () => {
      window.removeEventListener('mori:play', coordinate);
    };
  });
  /** Pause this player when another video begins explicit playback. */
  function coordinate(event: Event) {
    if ((event as CustomEvent<string>).detail !== video.id) media.pause();
  }
  /** Request standards-based fullscreen with a native Safari fallback. */
  async function fullscreen() {
    try {
      const supported: Partial<Pick<HTMLDivElement, 'requestFullscreen'>> =
        frame;
      const enter = supported.requestFullscreen;
      if (enter !== undefined) await enter.call(frame);
      else {
        const native = media as HTMLVideoElement & {
          webkitEnterFullscreen?: () => void;
        };
        native.webkitEnterFullscreen?.();
      }
    } catch {
      notice = 'Fullscreen is unavailable in this browser.';
    }
  }
  /** Reveal controls on touch before starting playback, while preserving keyboard activation. */
  function surface(event: MouseEvent) {
    if (
      event.detail === 0 ||
      !('pointerType' in event) ||
      event.pointerType !== 'touch'
    ) {
      void toggle();
      return;
    }
    if (!opened) {
      opened = true;
    } else {
      void toggle();
    }
  }
  /** Open metadata administration without rendering a display name over the picture. */
  function manage(): void {
    onmanage?.(video);
  }
</script>

<div
  bind:this={frame}
  class:opened
  class:playing={activated && !paused}
  class="player"
  role="group"
  aria-label={video.name}
  onfocusout={(e) => {
    if (!frame.contains(e.relatedTarget as Node)) opened = false;
  }}
  style:aspect-ratio={`${String(video.width > 0 ? video.width : 16)} / ${String(video.height > 0 ? video.height : 9)}`}
>
  <video
    bind:this={media}
    src={activated ? video.media_url : undefined}
    poster={['ready', 'published', 'publishing', 'revoking'].includes(
      video.state,
    )
      ? (video.poster_url ?? undefined)
      : undefined}
    preload="none"
    playsinline
    bind:paused
    bind:currentTime={time}
    bind:duration
    bind:muted
    onerror={() =>
      (failure =
        'Could not load this video. Please retry or check browser support.')}
    aria-label={video.name}><track kind="captions" /></video
  >
  {#if !['ready', 'published', 'publishing', 'revoking'].includes(video.state)}<div
      class="pending"
      role="status"
    >
      {video.state === 'failed'
        ? 'Upload could not be verified'
        : video.state === 'uploading'
          ? 'Upload unfinished'
          : video.state === 'deleting'
            ? 'Withdrawing video…'
            : 'Verifying video…'}
    </div>{/if}
  <button
    class="surface"
    aria-label={opened
      ? paused
        ? `Play ${video.name}`
        : `Pause ${video.name}`
      : `Show controls for ${video.name}`}
    onclick={surface}
  ></button>
  <div class="top controls">
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
        onclick={manage}><Ellipsis size={22} strokeWidth={1.5} /></Button
      >{/if}
  </div>
  <div class="bottom controls">
    <Button
      variant="ghost"
      size="icon"
      type="submit"
      class="icon-button shrink-0"
      aria-label={paused ? 'Play video' : 'Pause video'}
      onclick={toggle}
      >{#if paused}<Play
          size={21}
          fill="currentColor"
          strokeWidth={1}
        />{:else}<Pause
          size={21}
          fill="currentColor"
          strokeWidth={1}
        />{/if}</Button
    >
    <span class="timestamp"
      ><span>{clock(time)}</span><span class="time-divider" aria-hidden="true"
        >/</span
      ><span class="duration"
        >{clock(duration > 0 ? duration : video.duration)}</span
      ></span
    ><input
      style:--progress={`${String(duration > 0 ? Math.min(100, Math.max(0, (time / duration) * 100)) : 0)}%`}
      aria-label="Seek video"
      class="min-w-0 flex-1 accent-[#f0eee6]"
      disabled={!activated}
      type="range"
      min="0"
      max={Number.isFinite(duration) ? duration : 0}
      step="0.1"
      value={time}
      oninput={(e) => {
        if (activated) media.currentTime = Number(e.currentTarget.value);
      }}
    /><Button
      variant="ghost"
      size="icon"
      type="submit"
      class="icon-button shrink-0"
      aria-label={muted ? 'Unmute video' : 'Mute video'}
      onclick={() => (muted = !muted)}
      >{#if muted}<VolumeX size={20} strokeWidth={1.5} />{:else}<Volume2
          size={20}
          strokeWidth={1.5}
        />{/if}</Button
    ><Button
      variant="ghost"
      size="icon"
      type="submit"
      class="icon-button shrink-0"
      aria-label="Enter fullscreen"
      onclick={fullscreen}><Maximize size={20} strokeWidth={1.5} /></Button
    >
  </div>
  {#if failure}<div class="failure" role="alert">
      <p>{failure}</p>
      <p class="mt-2 text-xs muted">
        Download the original if this browser cannot play its format.
      </p>
      <a
        class={buttonVariants({ variant: 'outline', class: 'mt-3' })}
        href={video.media_url}
        rel="external"
        data-sveltekit-reload
        download={video.name + '.' + video.extension}
        ><Download size={16} />Download original</a
      >
      <Button
        variant="outline"
        type="submit"
        class="button mt-3"
        onclick={toggle}>Retry playback</Button
      >
    </div>{/if}
  <span class="sr-only" role="status">{notice}</span>
  {#if notice}<button
      class="notice"
      onclick={() => (notice = '')}
      aria-label="Dismiss playback notification">{notice}</button
    >{/if}
</div>

<style>
  .player {
    position: relative;
    width: 100%;
    overflow: hidden;
    background: linear-gradient(135deg, #242626, #151616);
    isolation: isolate;
    container-type: inline-size;
  }
  .pending {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: #aaa99f;
    font-size: 14px;
  }
  .surface {
    position: absolute;
    inset: 0;
    border: 0;
    background: transparent;
    width: 100%;
    height: 100%;
  }
  video {
    pointer-events: none;
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
  }
  .controls {
    z-index: 1;
    position: absolute;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    color: #f4f1eb;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.18s ease;
  }
  .top {
    top: 0;
    justify-content: flex-end;
    gap: 0;
    padding: 2px 6px 24px 18px;
    background: linear-gradient(#0008, transparent);
  }
  .bottom input[type='range'] {
    margin-left: 11px;
    margin-right: 7px;
    min-height: 44px;
    cursor: pointer;
    appearance: none;
    background: transparent;
  }
  .bottom input[type='range']::-webkit-slider-runnable-track {
    height: 2px;
    background: linear-gradient(
      to right,
      #f1eee7 var(--progress),
      #ddd7 var(--progress)
    );
  }
  .bottom input[type='range']::-webkit-slider-thumb {
    appearance: none;
    width: 8px;
    height: 8px;
    margin-top: -3px;
    border-radius: 50%;
    background: #f1eee7;
  }
  .bottom input[type='range']::-moz-range-track {
    height: 2px;
    background: #ddd7;
  }
  .bottom input[type='range']::-moz-range-progress {
    height: 2px;
    background: #f1eee7;
  }
  .bottom input[type='range']::-moz-range-thumb {
    width: 8px;
    height: 8px;
    border: 0;
    border-radius: 50%;
    background: #f1eee7;
  }
  .bottom {
    bottom: 0;
    padding: 16px 6px 4px 12px;
    gap: 6px;
    background: linear-gradient(transparent, #000b);
  }
  .timestamp {
    display: inline-flex;
    justify-content: space-between;
    gap: 8px;
    min-width: 90px;
    margin-left: 2px;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.035em;
    white-space: nowrap;
    flex-shrink: 0;
  }
  .time-divider {
    opacity: 0.65;
  }
  @container (max-width: 400px) {
    .bottom {
      padding-inline: 6px;
      gap: 4px;
    }
    .timestamp {
      min-width: 80px;
      font-size: 12px;
      margin-left: 0;
      gap: 5px;
    }
    .bottom input[type='range'] {
      margin-inline: 0;
    }
  }
  @container (max-width: 280px) {
    .timestamp {
      min-width: 32px;
    }
    .duration,
    .time-divider {
      display: none;
    }
  }
  .player:hover .controls,
  .player:focus-within .controls,
  .opened .controls {
    opacity: 1;
    pointer-events: auto;
  }
  .failure {
    z-index: 3;
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 20px;
    background: #101210df;
    font-size: 15px;
    text-align: center;
  }
  .notice {
    z-index: 2;
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 65px;
    padding: 10px;
    background: #20251ef0;
    border: 1px solid #ffffff30;
    font-size: 13px;
  }
  .player:fullscreen {
    width: 100vw;
    height: 100vh;
    aspect-ratio: auto !important;
    background: #000;
  }
  @media (hover: none) {
    .player:hover:not(.opened):not(:focus-within) .controls {
      opacity: 0;
      pointer-events: none;
    }
  }
</style>
