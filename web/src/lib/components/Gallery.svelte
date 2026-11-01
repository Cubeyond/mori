<script lang="ts">
  /** Image-only masonry previews; selecting a video opens a full-frame player. */
  import { onMount } from 'svelte';
  import type { Video } from '#lib/api/client';
  import { arrangeGallery } from '#lib/features/gallery';
  import { preferences } from '#lib/features/preferences';
  import VideoCard from './library/VideoCard.svelte';
  import VideoViewer from './library/VideoViewer.svelte';
  /** Optional per-video administration is absent from public collections. */
  let {
    videos,
    onmanage,
  }: { videos: Video[]; onmanage?: (video: Video) => void } = $props();
  /** Resize measurements are independent of image or video downloads. */
  let canvas: HTMLDivElement;
  /** Measured content width and currently selected full-frame video. */
  let width = $state(0),
    selected = $state<Video | null>(null);
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
  aria-label="Video gallery"
>
  {#each layout.items as placement (placement.video.id)}
    <div
      class="video-item"
      data-column={placement.column}
      style:left={String(placement.x) + 'px'}
      style:top={String(placement.y) + 'px'}
      style:width={String(placement.width) + 'px'}
      style:height={String(placement.height) + 'px'}
    >
      <VideoCard
        video={placement.video}
        onopen={() => (selected = placement.video)}
        {onmanage}
      />
    </div>
  {/each}
</div>
{#if selected !== null}<VideoViewer
    video={selected}
    onclose={() => (selected = null)}
  />{/if}

<style>
  .gallery {
    position: relative;
    width: 100%;
  }
  .video-item {
    position: absolute;
  }
</style>
