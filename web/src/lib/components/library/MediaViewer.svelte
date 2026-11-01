<script lang="ts">
  import Button from '#lib/components/ui/button/button.svelte';
  /** Full-frame player; closing unmounts playback and restores thumbnail focus. */
  import { X, Maximize, Download } from '@lucide/svelte';
  import type { MediaItem } from '#lib/api/client';
  import { lockPageScroll } from '#lib/features/dialog';
  import Player from '../Player.svelte';
  /** Selected video and a single dismissal callback. */
  let { video, onclose }: { video: MediaItem; onclose: () => void } = $props();
  /** Native dialog owns keyboard focus and Escape behavior. */
  let dialog: HTMLDialogElement;
  /** Match the entire source frame within the available viewport height. */
  const ratio = $derived(
    video.width > 0 && video.height > 0 ? video.width / video.height : 16 / 9,
  );
  /** A drag beginning inside the player cannot dismiss it on release. */
  let backdropPress = false;
  /** Original images may require a decoder unavailable in this browser. */
  let failed = $state(false);
  /** Fullscreen belongs to the image frame independently of the video player. */
  let imageFrame = $state<HTMLDivElement>();
  /** Unsupported fullscreen APIs report feedback instead of throwing an event-handler rejection. */
  let notice = $state('');
  /** Keep browser capability differences inside the image viewer. */
  async function fullscreen(): Promise<void> {
    try {
      if (
        imageFrame === undefined ||
        typeof imageFrame.requestFullscreen !== 'function'
      ) {
        notice = 'Fullscreen is unavailable in this browser.';
        return;
      }
      await imageFrame.requestFullscreen();
    } catch {
      notice = 'Fullscreen is unavailable in this browser.';
    }
  }
  $effect(() => {
    const unlock = lockPageScroll();
    dialog.showModal();
    return () => {
      dialog.close();
      unlock();
    };
  });
</script>

<dialog
  bind:this={dialog}
  aria-label={video.kind === 'image' ? 'Image viewer' : 'Video player'}
  oncancel={(event) => {
    event.preventDefault();
    onclose();
  }}
  onpointerdown={(event) => (backdropPress = event.target === dialog)}
  onclick={(event) => {
    if (backdropPress && event.target === dialog) onclose();
    backdropPress = false;
  }}
>
  <div
    class="viewer"
    style:width={'min(1120px, calc(100vw - 32px), calc((100svh - 100px) * ' +
      String(ratio) +
      '))'}
  >
    <div class="viewer-toolbar">
      <span>{video.kind === 'image' ? 'Image' : 'Video'}</span>
      {#if video.kind === 'image'}<div class="image-actions">
          <a
            href={video.media_url}
            download={video.name + '.' + video.extension}
            data-sveltekit-reload
            rel="external"
            aria-label="Download original image"><Download size={20} /></a
          >
          <Button
            variant="ghost"
            size="icon"
            type="button"
            aria-label="View image fullscreen"
            onclick={fullscreen}><Maximize size={20} /></Button
          >
        </div>{/if}
      <Button
        variant="ghost"
        size="icon"
        type="submit"
        class="icon-button"
        aria-label={video.kind === 'image'
          ? 'Close image viewer'
          : 'Close video player'}
        onclick={onclose}><X size={20} /></Button
      >
    </div>
    {#if video.kind === 'image'}<div
        class="image-frame"
        bind:this={imageFrame}
        style:aspect-ratio={String(ratio)}
      >
        {#if failed}<div class="image-unavailable" role="status">
            <p>This browser cannot preview this image.</p>
            <a
              href={video.media_url}
              download={video.name + '.' + video.extension}
              data-sveltekit-reload
              rel="external"><Download size={18} />Download original</a
            >
          </div>
        {:else}<img
            src={video.media_url}
            alt="Shared media"
            decoding="async"
            referrerpolicy="no-referrer"
            onerror={() => {
              failed = true;
            }}
          />{/if}
      </div>{:else}<Player {video} />{/if}
    {#if notice}<p role="status">{notice}</p>{/if}
  </div>
</dialog>

<style>
  dialog {
    margin: auto;
    padding: 0;
    max-width: none;
    max-height: none;
    border: 0;
    color: #f4f4f5;
    background: transparent;
    overflow: visible;
  }
  dialog::backdrop {
    background: var(--viewer-backdrop, #05060be8);
    backdrop-filter: blur(6px);
  }
  .viewer {
    max-width: 100%;
  }
  .viewer-toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 48px;
    color: #a2a2af;
    font-size: 14px;
  }
  .image-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .image-actions a {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
  }
  .image-frame {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    max-height: calc(100svh - 100px);
    background: var(--viewer-surface, #0b0c14);
  }
  .image-frame img {
    width: 100%;
    height: 100%;
    max-height: calc(100svh - 100px);
    object-fit: contain;
  }
  .image-frame:fullscreen,
  .image-frame:fullscreen img {
    width: 100%;
    height: 100%;
    max-height: 100svh;
  }
  .image-unavailable {
    display: grid;
    justify-items: center;
    gap: 16px;
    padding: 32px;
    text-align: center;
    color: #a4a4b2;
  }
  .image-unavailable a {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: #e4e4eb;
  }
</style>
