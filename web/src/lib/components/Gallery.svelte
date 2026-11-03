<script lang="ts">
  /** Image-only masonry previews; selecting a video opens a full-frame player. */
  import { onMount } from 'svelte';
  import type { MediaItem } from '#lib/api/client';
  import { arrangeGallery } from '#lib/features/gallery';
  import { preferences } from '#lib/features/preferences';
  import MediaCard from './library/MediaCard.svelte';
  import MediaViewer from './library/MediaViewer.svelte';
  /** Optional per-video administration is absent from public collections. */
  let {
    videos,
    onmanage,
    selection,
    onselect,
  }: {
    videos: MediaItem[];
    onmanage?: (video: MediaItem) => void;
    selection?: ReadonlySet<string> | undefined;
    onselect?: (id: string) => void;
  } = $props();
  /** Resize measurements are independent of image or video downloads. */
  let canvas: HTMLDivElement;
  /** Measured content width and currently selected full-frame video. */
  let width = $state(0),
    selected = $state<MediaItem | null>(null);
  /** Preview navigation follows the same filtered reading order as thumbnails. */
  const playable = $derived(
    videos.filter((item) =>
      ['ready', 'published', 'publishing', 'revoking'].includes(item.state),
    ),
  );
  const selectedIndex = $derived(
    playable.findIndex((item) => item.id === selected?.id),
  );
  /** Boundaries remain explicit rather than looping unexpectedly to the beginning. */
  function navigate(direction: number): void {
    const next = playable[selectedIndex + direction];
    if (next !== undefined) selected = next;
  }
  /** Desktop uses the reference's four columns and consistent 14px gutters. */
  const layout = $derived(
    arrangeGallery(
      videos,
      width,
      $preferences.density === 'compact',
      width < 480 ? 10 : 14,
    ),
  );
  onMount(() => {
    const observer = new ResizeObserver(([entry]) => {
      if (entry !== undefined) width = entry.contentRect.width;
    });
    observer.observe(canvas);
    return () => {
      observer.disconnect();
    };
  });
</script>

<div
  class="gallery"
  bind:this={canvas}
  style:height={String(layout.height) + 'px'}
  aria-label="Media gallery"
>
  {#each layout.items as placement (placement.video.id)}
    <div
      class="media-item"
      data-column={placement.column}
      style:left={String(placement.x) + 'px'}
      style:top={String(placement.y) + 'px'}
      style:width={String(placement.width) + 'px'}
      style:height={String(placement.height) + 'px'}
    >
      <MediaCard
        video={placement.video}
        onopen={() => {
          if (selection !== undefined) onselect?.(placement.video.id);
          else selected = placement.video;
        }}
        selection={selection === undefined
          ? undefined
          : selection.has(placement.video.id)}
        {onmanage}
      />
    </div>
  {/each}
</div>
{#if selected !== null}<MediaViewer
    video={selected}
    position={playable.length > 1
      ? String(selectedIndex + 1) + ' / ' + String(playable.length)
      : ''}
    onprevious={selectedIndex > 0
      ? () => {
          navigate(-1);
        }
      : undefined}
    onnext={selectedIndex >= 0 && selectedIndex < playable.length - 1
      ? () => {
          navigate(1);
        }
      : undefined}
    onclose={() => (selected = null)}
  />{/if}

<style>
  .gallery {
    position: relative;
    width: 100%;
  }
  .media-item {
    position: absolute;
  }
</style>
