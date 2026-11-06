<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: Elastic-2.0 -->

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
    footer,
    maxWidth = 520,
    hideScrollbar = false,
    closeLabel = 'Close dialog',
  }: {
    open: boolean;
    title: string;
    onclose: () => void;
    children: Snippet;
    /** Optional actions remain reachable while the body scrolls on short screens. */
    footer?: Snippet;
    maxWidth?: number;
    hideScrollbar?: boolean;
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
  class:hide-scrollbar={hideScrollbar}
  style:--dialog-width={String(maxWidth) + 'px'}
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
    <div class="dialog-body">{@render children()}</div>
    {#if footer !== undefined}{@render footer()}{/if}
  </div>
</dialog>

<style>
  dialog.cn-dialog-content:not([open]) {
    display: none;
  }
  dialog.cn-dialog-content {
    padding: 0;
    margin: auto;
    width: min(var(--dialog-width), calc(100% - 32px));
    max-width: min(var(--dialog-width), calc(100% - 32px));
    max-height: calc(100dvh - 32px);
    overflow: hidden;
    color: var(--color-paper);
    background: var(--popover);
    border: 1px solid var(--border);
    border-radius: var(--radius-xl);
    box-shadow: 0 24px 96px #0008;
  }
  .hide-scrollbar {
    scrollbar-width: none;
  }
  .hide-scrollbar .dialog-body {
    scrollbar-width: none;
  }
  .hide-scrollbar .dialog-body::-webkit-scrollbar,
  .hide-scrollbar::-webkit-scrollbar {
    display: none;
  }
  dialog::backdrop {
    background: transparent;
    backdrop-filter: blur(8px);
  }
  dialog[open] {
    animation: arrive 160ms ease-out;
  }
  .sheet {
    padding: 28px;
    display: flex;
    flex-direction: column;
    max-height: inherit;
  }
  .dialog-body {
    /* Reserve the outward focus ring inside the scroll clip without shifting form alignment. */
    padding: 6px;
    margin: -6px;
    scroll-padding: 6px;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  header {
    flex: none;
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
    dialog.cn-dialog-content {
      width: calc(100% - 24px);
      max-width: calc(100% - 24px);
      max-height: calc(100dvh - 24px);
    }
    .sheet {
      padding: 20px;
    }
    .sheet :global(.form-actions) {
      display: flex;
      flex-direction: column-reverse;
      gap: 8px;
    }
    .sheet :global(.form-actions > button) {
      width: 100%;
      min-height: 44px;
    }
    .sheet :global(.form-actions > button:first-child) {
      border-color: transparent;
      background: transparent;
    }

    header {
      margin: -2px -4px 20px 0;
      gap: 12px;
    }
    header :global(.icon-button) {
      width: 44px;
      height: 44px;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    dialog[open] {
      animation: none;
    }
  }
</style>
