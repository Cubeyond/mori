<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: AGPL-3.0-or-later -->

<script lang="ts">
  /** Fixed title and visibility rows frame an optional description beside up to three cropped posters. */
  import { resolve } from '$app/paths';
  import {
    Globe,
    LockKeyhole,
    Ellipsis,
    Pencil,
    Trash,
    Play,
  } from '@lucide/svelte';
  import Button from '#lib/components/ui/button/button.svelte';
  import CopyButton from '../CopyButton.svelte';
  import type { Album, AlbumSummary } from '#lib/api/client';
  /** The parent owns mutation dialogs so one canonical editor serves the entire catalog. */
  let {
    album,
    onedit,
    ondelete,
  }: {
    album: AlbumSummary;
    onedit: (album: Album) => void;
    ondelete: (album: Album) => void;
  } = $props();
  /** Native popovers provide Escape, outside dismissal and focus return without a document listener. */
  let menu: HTMLDivElement | undefined;
  /** DOM identifiers are stable across metadata polling and scoped to this card. */
  const menuId = $derived('album-actions-' + album.id);
  const triggerId = $derived('album-trigger-' + album.id);
  /** Keep the unboxed disclosure inside the viewport, including cards near the bottom edge. */
  function position(event: ToggleEvent): void {
    if (event.newState !== 'open' || menu === undefined) return;
    const trigger = document.getElementById(triggerId);
    if (trigger === null) return;
    const bounds = trigger.getBoundingClientRect();
    menu.style.right =
      String(Math.max(12, window.innerWidth - bounds.right)) + 'px';
    const below = window.innerHeight - bounds.bottom;
    menu.style.top = below >= 160 ? String(bounds.bottom + 6) + 'px' : 'auto';
    menu.style.bottom =
      below >= 160
        ? 'auto'
        : String(window.innerHeight - bounds.top + 6) + 'px';
  }
  /** Close the disclosure before opening the native dialog for the chosen operation. */
  function act(action: (value: Album) => void): void {
    menu?.hidePopover();
    action(album);
  }
</script>

<article class="album-card" aria-label={album.name}>
  <a
    class="album-content"
    class:empty={album.previews.length === 0}
    href={resolve('/albums/[id]', { id: album.id })}
  >
    <div class="metadata">
      <h2>{album.name}</h2>
      {#if album.description !== ''}<p>{album.description}</p>{/if}
    </div>
    <span class="visibility">
      {#if album.visibility === 'public'}<Globe
          size={18}
          aria-hidden="true"
        />{:else}<LockKeyhole size={18} aria-hidden="true" />{/if}
      {album.visibility === 'public' ? 'Public' : 'Private'}
    </span>
    {#if album.previews.length > 0}<div
        class="preview-rail"
        class:full={album.previews.length === 3}
        aria-hidden="true"
      >
        {#each album.previews as preview (preview.id)}<div class="preview">
            <img
              src={preview.poster_url}
              alt=""
              loading="lazy"
              decoding="async"
            />
            {#if preview.kind === 'video'}<Play
                class="video-mark"
                size={14}
                fill="currentColor"
              />{/if}
          </div>{/each}
      </div>{/if}
  </a>
  <div class="actions" class:empty={album.previews.length === 0}>
    <Button
      id={triggerId}
      variant="ghost"
      size="icon-sm"
      aria-label={'Actions for ' + album.name}
      popovertarget={menuId}><Ellipsis size={20} /></Button
    >
  </div>
</article>
<div
  id={menuId}
  bind:this={menu}
  class="album-menu"
  popover="auto"
  onbeforetoggle={position}
  role="group"
  aria-label={'Actions for ' + album.name}
>
  <CopyButton
    label="Copy album link"
    text="Copy album link"
    value={new URL('/' + album.slug, window.location.origin).href}
    disabled={album.visibility !== 'public'}
    icon="link"
    class="album-menu-action"
  />
  <Button
    variant="ghost"
    onclick={() => {
      act(onedit);
    }}><Pencil size={17} />Edit album</Button
  >
  <Button
    variant="ghost"
    class="delete-action"
    onclick={() => {
      act(ondelete);
    }}><Trash size={17} />Delete album</Button
  >
</div>

<style>
  .album-card {
    --preview-width: 174px;
    --preview-gap: 20px;
    --card-padding: 16px;
    position: relative;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--card);
  }
  .album-card:has(.album-content:hover),
  .album-card:focus-within {
    border-color: var(--muted-foreground);
  }
  .album-content {
    display: grid;
    grid-template-columns: minmax(0, 1fr) var(--preview-width);
    grid-template-rows: auto minmax(0, 1fr) auto;
    grid-template-areas:
      'title previews'
      'description previews'
      'visibility previews';
    gap: 8px var(--preview-gap);
    block-size: 226px;
    padding: var(--card-padding) var(--card-padding) var(--card-padding) 24px;
    border-radius: inherit;
  }
  .album-content.empty {
    grid-template-columns: minmax(0, 1fr);
    grid-template-areas: 'title' 'description' 'visibility';
  }
  .metadata {
    display: contents;
  }
  h2,
  p {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
    overflow-wrap: anywhere;
  }
  h2 {
    grid-area: title;
    padding-inline-end: 44px;
    font-size: 26px;
    line-height: 34px;
    font-weight: 550;
    letter-spacing: -0.025em;
  }
  p {
    grid-area: description;
    align-self: start;
    margin: 0;
    font-size: 1.125rem;
    line-height: 28px;
    color: var(--muted-foreground);
    white-space: pre-line;
  }
  .preview-rail {
    grid-area: previews;
    display: grid;
    align-content: start;
    grid-auto-rows: max-content;
    gap: 6px;
    padding-inline-start: 18px;
    border-inline-start: 1px solid var(--border);
  }
  .preview-rail.full {
    grid-template-rows: repeat(3, minmax(0, 1fr));
    align-content: stretch;
  }
  .preview {
    position: relative;
    aspect-ratio: 1.8;
    min-height: 0;
    overflow: hidden;
    border-radius: 3px;
  }
  .full .preview {
    aspect-ratio: auto;
  }
  .preview img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .preview :global(.video-mark) {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    color: #fff;
    filter: drop-shadow(0 1px 2px #000);
  }
  .actions {
    position: absolute;
    top: 14px;
    right: calc(
      var(--preview-width) + var(--preview-gap) + var(--card-padding)
    );
    display: flex;
    align-items: center;
    color: var(--muted-foreground);
  }
  .actions.empty {
    right: var(--card-padding);
  }
  .visibility {
    grid-area: visibility;
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 24px;
    color: var(--muted-foreground);
    font-size: 16px;
    line-height: 24px;
  }
  .album-menu {
    position: fixed;
    inset: auto;
    margin: 0;
    padding: 4px;
    width: 192px;
    max-width: calc(100vw - 24px);
    max-height: calc(100dvh - 24px);
    overflow-y: auto;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--popover);
    color: var(--foreground);
  }
  .album-menu:popover-open {
    display: grid;
    gap: 2px;
  }
  .album-menu :global(button) {
    width: 100%;
    justify-content: flex-start;
  }
  .album-menu :global(.delete-action svg) {
    color: var(--destructive);
  }
  @media (max-width: 480px) {
    .album-card {
      --preview-width: 104px;
      --preview-gap: 12px;
      --card-padding: 14px;
    }
    .album-content {
      grid-template-areas:
        'title title'
        'description previews'
        'visibility previews';
      block-size: 212px;
      padding-inline-start: 20px;
    }
    .preview-rail {
      padding-inline-start: 12px;
    }
    .actions {
      top: 10px;
      right: var(--card-padding);
    }
    h2 {
      font-size: 23px;
      line-height: 30px;
    }
    p {
      font-size: 1.0625rem;
      line-height: 26px;
    }
  }
  @media (pointer: coarse) {
    .actions :global(button) {
      min-width: 44px;
      min-height: 44px;
    }
  }
</style>
