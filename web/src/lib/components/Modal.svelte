<script lang="ts">
  import Button from '#lib/components/ui/button/button.svelte';
  /** Shared native dialog with focus restoration and a non-shifting backdrop. */
  import type { Snippet } from 'svelte';
  import { X } from '@lucide/svelte';
  import { lockPageScroll } from '#lib/features/dialog';

  /** Controlled visibility, accessible title, and modal contents. */
  let {
    open,
    title,
    onclose,
    children,
    wide = false,
    closeLabel = 'Close dialog',
  }: {
    open: boolean;
    title: string;
    onclose: () => void;
    children: Snippet;
    wide?: boolean;
    closeLabel?: string;
  } = $props();
  /** Native top-layer element retains the original trigger for focus restoration. */
  let dialog = $state<HTMLDialogElement>();
  /** Pointer-down location prevents a drag from inside the dialog closing it. */
  let backdropPress = false;

  $effect(() => {
    const element = dialog;
    if (element === undefined || !open) return;
    const unlock = lockPageScroll();
    if (!element.open) element.showModal();
    return () => {
      element.close();
      unlock();
    };
  });

  /** Dismiss only a complete click on the backdrop rather than its contents. */
  function dismissBackdrop(event: MouseEvent): void {
    if (backdropPress && event.target === dialog) onclose();
    backdropPress = false;
  }
</script>

<dialog
  bind:this={dialog}
  aria-label={title}
  class="cn-dialog-content"
  class:wide
  oncancel={(event) => {
    event.preventDefault();
    onclose();
  }}
  onpointerdown={(event) => (backdropPress = event.target === dialog)}
  onclick={dismissBackdrop}
>
  <div class="sheet">
    <header>
      <h2>{title}</h2>
      <Button
        variant="ghost"
        size="icon"
        class="icon-button"
        type="button"
        aria-label={closeLabel}
        onclick={onclose}
      >
        <X size={20} strokeWidth={1.5} />
      </Button>
    </header>
    {@render children()}
  </div>
</dialog>

<style>
  dialog.cn-dialog-content:not([open]) {
    display: none;
  }
  dialog.cn-dialog-content {
    padding: 0;
    margin: auto;
    width: min(520px, calc(100% - 32px));
    max-width: min(520px, calc(100% - 32px));
    max-height: calc(100svh - 40px);
    overflow: auto;
    color: var(--color-paper);
    background: var(--popover);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    box-shadow: 0 24px 96px #0008;
  }
  dialog.wide {
    width: min(720px, calc(100% - 32px));
    max-width: min(720px, calc(100% - 32px));
  }
  dialog::backdrop {
    background: var(--dialog-backdrop, #070808b8);
    backdrop-filter: blur(6px);
  }
  dialog[open] {
    animation: arrive 160ms ease-out;
  }
  .sheet {
    padding: 28px;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    margin: -6px -8px 20px 0;
  }
  h2 {
    font-size: 20px;
    font-weight: 600;
    line-height: 1.4;
  }
  @keyframes arrive {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  @media (max-width: 480px) {
    .sheet {
      padding: 20px;
    }
  }
</style>
