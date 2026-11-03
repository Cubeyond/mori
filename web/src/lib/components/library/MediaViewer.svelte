<script lang="ts">
  /** A native modal owns focus; only the media pixels participate in zoom and pan. */
  import {
    X,
    Maximize,
    Minimize,
    Download,
    Plus,
    Minus,
    ChevronLeft,
    ChevronRight,
    RotateCcw,
  } from '@lucide/svelte';
  import type { MediaItem } from '#lib/api/client';
  import { lockPageScroll } from '#lib/features/dialog';
  import type { View, ZoomControls } from '#lib/features/lightbox';
  import Player from '../Player.svelte';
  import ZoomViewport from './ZoomViewport.svelte';
  /** Navigation is bounded to this gallery's loaded and visible media. */
  let {
    video,
    onclose,
    onprevious,
    onnext,
    position = '',
  }: {
    video: MediaItem;
    onclose: () => void;
    onprevious?: (() => void) | undefined;
    onnext?: (() => void) | undefined;
    position?: string;
  } = $props();
  /** Fullscreen belongs to the inner container, keeping the toolbar accessible. */
  let dialog: HTMLDialogElement, frame: HTMLDivElement;
  let zoom = $state<ZoomControls>();
  /** Gesture state and decoding feedback reset independently for every source. */
  let view = $state<View>({ scale: 1, x: 0, y: 0 }),
    failed = $state(false),
    loading = $state(true),
    retry = $state(0);
  let exitedAt = Number.NEGATIVE_INFINITY;
  let notice = $state(''),
    expanded = $state(false);
  /** Fullscreen hides chrome until pointer, touch, or keyboard interaction requests it. */
  let toolbarVisible = $state(true);
  let toolbarTimer: ReturnType<typeof setTimeout> | undefined;
  /** Only one idle timer can hide the fullscreen toolbar. */
  function scheduleHide(delay = 600): void {
    if (toolbarTimer !== undefined) clearTimeout(toolbarTimer);
    toolbarTimer = setTimeout(() => {
      toolbarVisible = false;
      toolbarTimer = undefined;
    }, delay);
  }
  /** The top edge reveals controls without reserving space above the media. */
  function pointer(event: PointerEvent): void {
    if (!expanded || event.pointerType === 'touch') return;
    if (event.clientY - frame.getBoundingClientRect().top <= 72) {
      toolbarVisible = true;
      if (toolbarTimer !== undefined) {
        clearTimeout(toolbarTimer);
        toolbarTimer = undefined;
      }
    } else if (toolbarVisible && toolbarTimer === undefined) scheduleHide();
  }
  /** A stationary touch first reveals hidden chrome; drags and pinches never reach this callback. */
  function touch(event: MouseEvent): void {
    if (!expanded || !('pointerType' in event) || event.pointerType !== 'touch')
      return;
    if (!toolbarVisible) {
      event.preventDefault();
      event.stopPropagation();
    }
    toolbarVisible = true;
    scheduleHide(2500);
  }
  /** Intrinsic sizes may be corrected by the browser for legacy metadata. */
  let width = $state(16),
    height = $state(9);
  $effect(() => {
    width = video.width > 0 ? video.width : 16;
    height = video.height > 0 ? video.height : 9;
    view = { scale: 1, x: 0, y: 0 };
    failed = false;
    loading = true;
    retry = 0;
    notice = '';
  });
  /** Request or exit fullscreen without losing navigation or the close button. */
  async function fullscreen(): Promise<void> {
    try {
      if (document.fullscreenElement !== null) await document.exitFullscreen();
      else if (typeof frame.requestFullscreen === 'function') {
        await frame.requestFullscreen();
        frame.focus({ preventScroll: true });
        toolbarVisible = true;
        scheduleHide(1200);
      } else {
        const native = frame.querySelector('video');
        const safari:
          (HTMLVideoElement & { webkitEnterFullscreen?: () => void }) | null =
          native;
        if (safari?.webkitEnterFullscreen === undefined)
          throw new Error('Unsupported fullscreen');
        safari.webkitEnterFullscreen();
      }
    } catch {
      notice = 'Fullscreen is unavailable in this browser.';
    }
  }
  /** First Escape exits fullscreen; the following Escape dismisses the modal. */
  function cancel(event: Event): void {
    event.preventDefault();
    if (document.fullscreenElement !== null)
      void document.exitFullscreen().catch(() => {
        notice = 'Could not exit fullscreen.';
      });
    else if (performance.now() - exitedAt > 250) onclose();
  }
  /** Preserve native slider and button keys while offering gallery and zoom shortcuts. */
  function keyboard(event: KeyboardEvent): void {
    if (
      event.defaultPrevented ||
      event.isComposing ||
      event.ctrlKey ||
      event.metaKey ||
      event.altKey ||
      !(event.target instanceof Element) ||
      event.target.closest('input,select,textarea,.player button,a') !== null
    )
      return;
    if (event.key === '+' || event.key === '=') {
      event.preventDefault();
      zoom?.change(1.5);
    } else if (event.key === '-') {
      event.preventDefault();
      zoom?.change(1 / 1.5);
    } else if (event.key === '0') {
      event.preventDefault();
      zoom?.reset();
    } else if (
      view.scale > 1 &&
      ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)
    ) {
      event.preventDefault();
      zoom?.pan(
        event.key === 'ArrowLeft' ? 80 : event.key === 'ArrowRight' ? -80 : 0,
        event.key === 'ArrowUp' ? 80 : event.key === 'ArrowDown' ? -80 : 0,
      );
    } else if (event.key === 'ArrowLeft' && onprevious !== undefined) {
      event.preventDefault();
      onprevious();
    } else if (event.key === 'ArrowRight' && onnext !== undefined) {
      event.preventDefault();
      onnext();
    }
  }
  $effect(() => {
    const unlock = lockPageScroll();
    const opener = document.activeElement;
    /** Reflect browser-driven fullscreen exits, including Escape. */
    const changed = (): void => {
      const current = document.fullscreenElement === frame;
      if (expanded && !current) exitedAt = performance.now();
      expanded = current;
      if (!current) {
        toolbarVisible = true;
        if (toolbarTimer !== undefined) {
          clearTimeout(toolbarTimer);
          toolbarTimer = undefined;
        }
      }
    };
    dialog.showModal();
    document.addEventListener('fullscreenchange', changed);
    return () => {
      document.removeEventListener('fullscreenchange', changed);
      if (toolbarTimer !== undefined) clearTimeout(toolbarTimer);
      dialog.close();
      unlock();
      if (opener instanceof HTMLElement && opener.isConnected)
        opener.focus({ preventScroll: true });
    };
  });
</script>

<dialog
  bind:this={dialog}
  aria-label="Media viewer"
  aria-describedby="viewer-help"
  oncancel={cancel}
  onkeydown={keyboard}
>
  <div
    class="viewer"
    role="group"
    aria-label="Media preview"
    class:is-fullscreen={expanded}
    class:toolbar-visible={toolbarVisible}
    bind:this={frame}
    tabindex="-1"
    onpointermove={pointer}
  >
    <header class="viewer-toolbar">
      <div class="zoom-tools" role="group" aria-label="Zoom">
        <button
          type="button"
          aria-label="Zoom out"
          disabled={view.scale <= 1}
          onclick={() => {
            zoom?.change(1 / 1.5);
          }}><Minus size={21} /></button
        >
        <button
          type="button"
          class="fit-button"
          aria-label="Reset zoom to fit"
          onclick={() => {
            zoom?.reset();
          }}
          >{view.scale <= 1.01
            ? 'Fit'
            : String(Math.round(view.scale * 100)) + '%'}</button
        >
        <button
          type="button"
          aria-label="Zoom in"
          disabled={view.scale >= 8}
          onclick={() => {
            zoom?.change(1.5);
          }}><Plus size={21} /></button
        >
      </div>
      <span class="position" aria-live="polite">{position}</span>
      <div class="media-tools">
        <a
          href={video.media_url}
          download={video.name + '.' + video.extension}
          data-sveltekit-reload
          rel="external"
          aria-label="Download original"><Download size={21} /></a
        >
        <button
          type="button"
          aria-label={expanded ? 'Exit fullscreen' : 'Enter fullscreen'}
          onclick={fullscreen}
          >{#if expanded}<Minimize size={21} />{:else}<Maximize
              size={21}
            />{/if}</button
        >
        <button type="button" aria-label="Close media viewer" onclick={onclose}
          ><X size={23} /></button
        >
      </div>
    </header>
    <div class="media-stage">
      {#key video.id + ':' + String(retry)}
        <ZoomViewport
          {width}
          {height}
          bind:view
          bind:this={zoom}
          doubleTap={video.kind === 'image'}
          ontap={touch}
        >
          {#snippet children(transform)}
            {#if video.kind === 'image'}
              <img
                src={video.media_url}
                alt="Shared media"
                class:decoded={!loading && !failed}
                draggable="false"
                decoding="async"
                referrerpolicy="no-referrer"
                style:transform
                onload={(event) => {
                  if (event.currentTarget instanceof HTMLImageElement) {
                    width = event.currentTarget.naturalWidth;
                    height = event.currentTarget.naturalHeight;
                  }
                  loading = false;
                }}
                onerror={() => {
                  failed = true;
                  loading = false;
                }}
              />
            {:else}<Player {video} {transform} onfullscreen={fullscreen} />{/if}
          {/snippet}
        </ZoomViewport>
      {/key}
      {#if video.kind === 'image' && loading}<span class="loading" role="status"
          >Loading image…</span
        >{/if}
      {#if failed}<div class="unavailable" role="status">
          <p>This browser could not preview this image.</p>
          <button
            type="button"
            onclick={() => {
              failed = false;
              loading = true;
              retry++;
            }}><RotateCcw size={18} />Retry</button
          >
        </div>{/if}
      {#if onprevious !== undefined}<button
          type="button"
          class="navigate previous"
          aria-label="Previous media"
          onclick={onprevious}><ChevronLeft size={24} /></button
        >{/if}
      {#if onnext !== undefined}<button
          type="button"
          class="navigate next"
          aria-label="Next media"
          onclick={onnext}><ChevronRight size={24} /></button
        >{/if}
    </div>
    <p id="viewer-help" class="sr-only">
      Scroll or pinch to zoom. Drag to pan. Use plus, minus, or zero to zoom and
      fit. Arrow keys browse media at fit, or pan when zoomed. Escape closes the
      viewer.
    </p>
    {#if notice}<p class="notice" role="status">{notice}</p>{/if}
  </div>
</dialog>

<style>
  dialog {
    margin: 0;
    padding: 0;
    width: 100vw;
    height: 100dvh;
    max-width: none;
    max-height: none;
    border: 0;
    color: #f5f5f5;
    background: #101113;
    overflow: hidden;
  }
  dialog::backdrop {
    background: #101113;
  }
  .viewer {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background: #101113;
    color: #f5f5f5;
  }
  .viewer:focus {
    outline: none;
  }
  .is-fullscreen .viewer-toolbar {
    position: absolute;
    inset: 0 0 auto;
    transform: translateY(-100%);
    opacity: 0;
    pointer-events: none;
    transition:
      transform 150ms cubic-bezier(0.2, 0, 0.38, 0.9),
      opacity 150ms ease-out;
  }
  .is-fullscreen.toolbar-visible .viewer-toolbar,
  .is-fullscreen .viewer-toolbar:has(:focus-visible) {
    transform: translateY(0);
    opacity: 1;
    pointer-events: auto;
  }
  @media (prefers-reduced-motion: reduce) {
    .is-fullscreen .viewer-toolbar {
      transition: none;
    }
  }
  .viewer-toolbar {
    display: flex;
    align-items: center;
    flex: none;
    min-height: 60px;
    padding: max(6px, env(safe-area-inset-top))
      max(8px, env(safe-area-inset-right)) 6px
      max(8px, env(safe-area-inset-left));
    border-bottom: 1px solid #ffffff20;
    background: #1b1d20;
    z-index: 5;
    gap: 4px;
  }
  .zoom-tools,
  .media-tools {
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .media-tools {
    margin-left: auto;
  }
  button,
  a {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: 44px;
    height: 44px;
    border: 0;
    border-radius: 7px;
    color: #f5f5f5;
    background: transparent;
    cursor: pointer;
    text-decoration: none;
  }
  button:hover,
  a:hover {
    background: #ffffff1c;
  }
  button:focus-visible,
  a:focus-visible {
    outline: 2px solid #c9d8ed;
    outline-offset: -3px;
  }
  button:disabled {
    opacity: 0.35;
    cursor: default;
  }
  .fit-button {
    width: 58px;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  .position {
    margin-left: 16px;
    color: #ced0d5;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
  }
  .media-stage {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
    opacity: 0;
    transform-origin: center;
  }
  img.decoded {
    opacity: 1;
  }
  .navigate {
    position: absolute;
    top: calc(50% - 22px);
    background: #1b1d20;
    box-shadow: 0 1px 8px #0004;
    z-index: 3;
  }
  .previous {
    left: max(12px, env(safe-area-inset-left));
  }
  .next {
    right: max(12px, env(safe-area-inset-right));
  }
  .loading {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    color: #b8bbc2;
    pointer-events: none;
  }
  .unavailable {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 20px;
    padding: 60px;
    background: #101113;
    text-align: center;
  }
  .unavailable button {
    width: auto;
    gap: 8px;
    padding: 0 16px;
    background: #292c31;
  }
  .notice {
    position: absolute;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    background: #1b1d20;
    padding: 12px 18px;
    border-radius: 7px;
    z-index: 5;
    font-size: 14px;
  }
  @media (max-width: 480px) {
    .is-fullscreen .position {
      display: none;
    }
    .position {
      position: absolute;
      bottom: max(14px, env(safe-area-inset-bottom));
      left: 50%;
      transform: translateX(-50%);
      margin: 0;
      z-index: 4;
      background: #1b1d20;
      padding: 4px 10px;
      border-radius: 7px;
      pointer-events: none;
    }
    .navigate {
      width: 44px;
    }
    .previous {
      left: 4px;
    }
    .next {
      right: 4px;
    }
  }
</style>
