<script lang="ts">
  import Button from '#lib/components/ui/button/button.svelte';
  /** Full-frame player; closing unmounts playback and restores thumbnail focus. */
  import { X } from '@lucide/svelte';
  import type { Video } from '#lib/api/client';
  import { lockPageScroll } from '#lib/features/dialog';
  import Player from '../Player.svelte';
  /** Selected video and a single dismissal callback. */
  let { video, onclose }: { video: Video; onclose: () => void } = $props();
  /** Native dialog owns keyboard focus and Escape behavior. */
  let dialog: HTMLDialogElement;
  /** Match the entire source frame within the available viewport height. */
  const ratio = $derived(
    video.width > 0 && video.height > 0 ? video.width / video.height : 16 / 9,
  );
  /** A drag beginning inside the player cannot dismiss it on release. */
  let backdropPress = false;
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
  aria-label="Video player"
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
      <span>Video</span><Button
        variant="ghost"
        size="icon"
        type="submit"
        class="icon-button"
        aria-label="Close video player"
        onclick={onclose}><X size={20} /></Button
      >
    </div>
    <Player {video} />
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
    background: #05060be8;
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
</style>
