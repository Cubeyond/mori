<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: AGPL-3.0-or-later -->

<script lang="ts">
  /** Decode only visible local previews and release every decoder and URL on removal. */
  import { onMount } from 'svelte';
  import {
    X,
    Play,
    Image as ImageIcon,
    Video as VideoIcon,
  } from '@lucide/svelte';
  import { prepareUploadPreview } from '#lib/features/upload-preview';
  import { previewSlot } from '#lib/features/preview-slots';
  import { formatBytes } from '#lib/features/upload-selection';
  /** The selected original remains untouched; optional preparation has a separate size. */
  let {
    file,
    kind,
    preparedSize,
    root,
    locked,
    active,
    completed,
    onremove,
  }: {
    file: File;
    kind: 'image' | 'video';
    preparedSize: number;
    root: HTMLElement | undefined;
    locked: boolean;
    active: boolean;
    completed: boolean;
    onremove: (file: File) => void;
  } = $props();
  /** One bounded JPEG replaces the full local source once the browser finishes decoding. */
  let url = $state(''),
    duration = $state(0),
    unavailable = $state(false);
  /** Intersection tracking is clipped by the four-item scrolling viewport. */
  let element: HTMLDivElement;
  /** Leaving a page cancels pending work rather than queuing every selected large file. */
  let controller: AbortController | undefined;
  /** Settle a local preview without exposing decoder errors as failed uploads. */
  async function decode(signal: AbortSignal): Promise<void> {
    let release: (() => void) | undefined;
    let preview: ReturnType<typeof prepareUploadPreview> | undefined;
    try {
      release = await previewSlot(signal);
      signal.throwIfAborted();
      preview = prepareUploadPreview(file, signal, kind, 640);
      const [metadata, poster] = await Promise.all([
        preview.metadata,
        preview.poster,
      ]);
      if (signal.aborted) return;
      duration = metadata.duration;
      if (poster === null) unavailable = true;
      else url = URL.createObjectURL(poster);
    } catch {
      if (!signal.aborted) unavailable = true;
    } finally {
      preview?.dispose();
      release?.();
    }
  }
  onMount(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) {
          controller?.abort();
          controller = undefined;
        } else if (url === '' && !unavailable && controller === undefined) {
          controller = new AbortController();
          void decode(controller.signal);
        }
      },
      { root: root ?? null },
    );
    observer.observe(element);
    return () => {
      observer.disconnect();
      controller?.abort();
      if (url !== '') URL.revokeObjectURL(url);
    };
  });
</script>

<div class="file-card" bind:this={element} role="group" aria-label={file.name}>
  <div class="visual">
    {#if url !== ''}<img src={url} alt={'Preview of ' + file.name} />
    {:else}<span class="placeholder" aria-hidden="true"
        >{#if kind === 'image'}<ImageIcon size={24} />{:else}<VideoIcon
            size={24}
          />{/if}</span
      >{/if}
    <button
      type="button"
      class="remove-file"
      aria-label={(completed ? 'Dismiss ' : 'Remove ') + file.name}
      disabled={locked}
      onclick={() => {
        onremove(file);
      }}><X size={17} /></button
    >
    {#if kind === 'video' && duration > 0}<span class="duration"
        ><Play size={11} fill="currentColor" />{String(
          Math.floor(duration / 60),
        ).padStart(2, '0')}:{String(Math.floor(duration % 60)).padStart(
          2,
          '0',
        )}</span
      >{/if}
    {#if active}<span class="active">Uploading</span>{/if}
  </div>
  <div class="file-info">
    <span class="filename" title={file.name}>{file.name}</span>
    <span class="metadata"
      >{formatBytes(preparedSize)} · {preparedSize < file.size
        ? String(Math.round((1 - preparedSize / file.size) * 100)) + '% smaller'
        : completed
          ? 'Original'
          : kind === 'image'
            ? 'Image'
            : 'Video'}</span
    >
  </div>
</div>

<style>
  .file-card {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .visual {
    position: relative;
    flex: 1;
    min-height: 0;
    overflow: hidden;
    border-radius: 7px;
    background: var(--card);
  }
  img,
  .placeholder {
    width: 100%;
    height: 100%;
    display: block;
  }
  img {
    object-fit: cover;
    object-position: center;
  }
  .placeholder {
    display: grid;
    place-items: center;
    color: var(--muted-foreground);
  }
  .remove-file {
    position: absolute;
    top: 4px;
    right: 4px;
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border-radius: 7px;
    color: #f4f4f5;
    background: #0b0c14d9;
  }
  .remove-file:hover {
    background: #252630;
  }
  .remove-file:disabled {
    opacity: 0.45;
  }
  .duration,
  .active {
    position: absolute;
    bottom: 7px;
    left: 7px;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    border-radius: 4px;
    background: #0b0c14c9;
    color: #f4f4f5;
    padding: 3px 6px;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .active {
    left: auto;
    right: 7px;
  }
  .file-info {
    display: grid;
    gap: 1px;
    min-width: 0;
    height: 38px;
  }
  .filename {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    font-size: 14px;
    line-height: 20px;
  }
  .metadata {
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--muted-foreground);
    font-size: 12px;
    line-height: 17px;
    white-space: nowrap;
  }
  @media (max-width: 480px) {
    .file-info {
      height: 38px;
    }
    .filename {
      font-size: 14px;
      line-height: 20px;
    }
    .metadata {
      font-size: 12px;
    }
  }
  @media (pointer: coarse) {
    .remove-file {
      width: 44px;
      height: 44px;
    }
  }
</style>
