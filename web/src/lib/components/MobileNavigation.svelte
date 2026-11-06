<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: AGPL-3.0-or-later -->

<script lang="ts">
  /** Full-page mobile navigation keeps the destination list clear and the underlying collection stable. */
  import { onMount } from 'svelte';
  import { X } from '@lucide/svelte';
  import Button from './ui/button/button.svelte';
  import Wordmark from './Wordmark.svelte';
  import LibraryNavigation from './LibraryNavigation.svelte';
  import { lockPageScroll } from '#lib/features/dialog';
  /** The root layout owns visibility and closes the drawer on navigation. */
  let {
    open,
    path,
    onclose,
  }: { open: boolean; path: string; onclose: () => void } = $props();
  /** A native top-layer dialog provides focus trapping, inert background, and focus return. */
  let drawer = $state<HTMLDialogElement>();
  $effect(() => {
    const element = drawer;
    if (element === undefined || !open) return;
    const unlock = lockPageScroll();
    element.showModal();
    element.querySelector<HTMLButtonElement>('button')?.focus();
    return () => {
      element.close();
      unlock();
    };
  });
  onMount(() => {
    const desktop = window.matchMedia('(min-width: 768px)');
    /** Restore the desktop layout without leaving modal scroll or focus state behind. */
    function resized(): void {
      if (desktop.matches) onclose();
    }
    desktop.addEventListener('change', resized);
    return () => {
      desktop.removeEventListener('change', resized);
    };
  });
  /** Wrap ordinary Tab navigation at both boundaries while preserving browser shortcuts. */
  function keyboard(event: KeyboardEvent): void {
    if (event.key !== 'Tab' || event.altKey || event.ctrlKey || event.metaKey)
      return;
    const targets = drawer?.querySelectorAll<HTMLElement>(
      'a[href], button:not(:disabled)',
    );
    if (targets === undefined) return;
    const first = targets[0],
      last = targets[targets.length - 1];
    if (first === undefined || last === undefined) return;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  }
</script>

<dialog
  bind:this={drawer}
  id="mobile-navigation"
  aria-label="Navigation"
  class="navigation-drawer"
  oncancel={(event) => {
    event.preventDefault();
    onclose();
  }}
  onkeydown={keyboard}
>
  <header>
    <Wordmark />
    <Button
      variant="ghost"
      size="icon"
      class="icon-button navigation-close"
      aria-label="Close navigation"
      onclick={onclose}><X class="size-6" size={24} /></Button
    >
  </header>
  <div class="drawer-content">
    <LibraryNavigation {path} variant="drawer" onfollow={onclose} />
  </div>
</dialog>

<style>
  dialog:not([open]) {
    display: none;
  }
  .navigation-drawer {
    position: fixed;
    inset: 0;
    margin: 0;
    padding: 0;
    width: 100%;
    max-width: none;
    height: 100vh;
    height: 100dvh;
    max-height: 100dvh;
    overflow-y: auto;
    overscroll-behavior: contain;
    border: 0;
    border-radius: 0;
    background: var(--navigation-surface, #101117);
    color: var(--foreground);
  }
  header {
    height: 56px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px 0 20px;
    border-bottom: 1px solid var(--border);
  }
  header :global(.navigation-close) {
    width: 48px;
    height: 48px;
  }
  .drawer-content {
    padding: 28px 20px max(28px, env(safe-area-inset-bottom));
    max-width: 560px;
    margin-inline: auto;
  }
  dialog::backdrop {
    background: transparent;
    backdrop-filter: blur(3px);
  }
  dialog[open] {
    animation: navigation-in 140ms ease-out;
  }
  @keyframes navigation-in {
    from {
      opacity: 0;
      transform: translateY(-8px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    dialog[open] {
      animation: none;
    }
  }
</style>
