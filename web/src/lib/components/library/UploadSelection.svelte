<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: Elastic-2.0 -->

<script lang="ts">
  /** Four-item scroll pages preserve a compact upload sheet without displaying a scrollbar. */
  import { ChevronUp, ChevronDown } from '@lucide/svelte';
  import type { UploadPolicy } from '#lib/api/client';
  import { formatBytes, uploadKind } from '#lib/features/upload-selection';
  import UploadThumbnail from './UploadThumbnail.svelte';
  /** Only the transferring item is locked; other pending files can still be removed. */
  let {
    files,
    policy,
    activeFiles,
    optimizedSizes,
    completedFiles,
    onremove,
  }: {
    files: File[];
    policy: UploadPolicy;
    activeFiles: File[];
    optimizedSizes: ReadonlyMap<File, number>;
    completedFiles: ReadonlySet<File>;
    onremove: (file: File) => void;
  } = $props();
  /** Each page owns two columns and at most two rows. */
  const pages = $derived(
    Array.from({ length: Math.ceil(files.length / 4) }, (_, index) =>
      files.slice(index * 4, index * 4 + 4),
    ),
  );
  /** Page buttons complement wheel/touch scrolling while the native track remains hidden. */
  let viewport = $state<HTMLDivElement>(),
    scrollPage = $state(0);
  /** Clamp the indicator after removals without writing derived state back into an effect. */
  const current = $derived(Math.min(scrollPage, Math.max(0, pages.length - 1)));
  /** Scroll exactly one group without moving the upload form or underlying gallery. */
  function show(index: number): void {
    viewport?.scrollTo({
      top:
        Math.max(0, Math.min(pages.length - 1, index)) * viewport.clientHeight,
    });
  }
</script>

<div class="queue-heading">
  <div class="summary">
    <span>{files.length} {files.length === 1 ? 'file' : 'files'}</span><span
      class="total-size"
      >{formatBytes(
        files.reduce(
          (size, file) => size + (optimizedSizes.get(file) ?? file.size),
          0,
        ),
      )}</span
    >
  </div>
  {#if pages.length > 1}<nav aria-label="Selected file pages">
      <button
        type="button"
        aria-label="Previous four files"
        disabled={current === 0}
        onclick={() => {
          show(current - 1);
        }}><ChevronUp size={17} /></button
      >
      <span aria-live="polite">{current + 1} / {pages.length}</span>
      <button
        type="button"
        aria-label="Next four files"
        disabled={current >= pages.length - 1}
        onclick={() => {
          show(current + 1);
        }}><ChevronDown size={17} /></button
      >
    </nav>{/if}
</div>
<div
  class="selection-viewport"
  class:one-row={files.length <= 2}
  bind:this={viewport}
  role="region"
  aria-label="Selected media"
  onscroll={() => {
    if (viewport !== undefined && viewport.clientHeight > 0)
      scrollPage = Math.round(viewport.scrollTop / viewport.clientHeight);
  }}
>
  {#each pages as page, index (index)}<div
      class="selection-page"
      role="group"
      aria-label={'Files ' +
        String(index * 4 + 1) +
        ' to ' +
        String(index * 4 + page.length)}
    >
      {#each page as file (file)}<UploadThumbnail
          {file}
          preparedSize={optimizedSizes.get(file) ?? file.size}
          kind={uploadKind(file, policy)}
          root={viewport}
          locked={activeFiles.includes(file)}
          active={activeFiles.includes(file)}
          completed={completedFiles.has(file)}
          {onremove}
        />{/each}
    </div>{/each}
</div>

<style>
  .queue-heading {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 12px;
    min-height: 32px;
    margin-bottom: 12px;
  }
  .summary {
    display: flex;
    gap: 8px;
    align-items: center;
    font-size: 14px;
  }
  .total-size {
    color: var(--muted-foreground);
  }
  .total-size::before {
    content: '·';
    margin-right: 8px;
  }
  nav {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }
  nav span {
    min-width: 36px;
    text-align: center;
    font-size: 13px;
    color: var(--muted-foreground);
    font-variant-numeric: tabular-nums;
  }
  nav button {
    width: 40px;
    height: 40px;
    display: grid;
    place-items: center;
    border-radius: 5px;
    color: var(--foreground);
  }
  nav button:hover:not(:disabled) {
    background: var(--hover-surface);
  }
  nav button:disabled {
    opacity: 0.3;
  }
  .selection-viewport {
    --page-height: clamp(240px, 32dvh, 320px);
    --rows: 2;
    height: var(--page-height);
    overflow-y: auto;
    overscroll-behavior: contain;
    scroll-snap-type: y mandatory;
    scrollbar-width: none;
    margin-bottom: 18px;
  }
  .selection-viewport::-webkit-scrollbar {
    display: none;
  }
  @media (min-width: 601px) {
    .selection-viewport {
      --page-height: clamp(336px, calc(100svh - 470px), 384px);
    }
  }
  .selection-viewport.one-row {
    --rows: 1;
    height: max(128px, calc((var(--page-height) - 16px) / 2));
  }
  .selection-page {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-template-rows: repeat(var(--rows), minmax(0, 1fr));
    gap: 16px;
    height: 100%;
    scroll-snap-align: start;
    scroll-snap-stop: always;
  }
  @media (pointer: coarse) {
    nav button {
      width: 44px;
      height: 44px;
    }
  }
  @media (max-width: 480px) {
    .summary {
      flex-direction: column;
      align-items: flex-start;
      gap: 0;
      font-size: 14px;
    }
    .total-size {
      font-size: 12px;
    }
    .total-size::before {
      content: none;
    }
    .selection-page {
      gap: 12px;
    }
  }
</style>
