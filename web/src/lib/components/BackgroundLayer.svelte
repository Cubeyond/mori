<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: Elastic-2.0 -->

<script lang="ts">
  /** Static personal backdrop has no scroll handlers, animation, or network image requests. */
  import { onMount } from 'svelte';
  import { background, loadBackground } from '#lib/features/background';
  import { preferences } from '#lib/features/preferences';
  onMount(() => {
    void loadBackground();
  });
</script>

{#if $background !== null}<div
    class="personal-background"
    aria-hidden="true"
    data-personal-background
  >
    <img
      src={$background}
      alt=""
      style:opacity={$preferences.backgroundOpacity}
    />
  </div>{/if}

<style>
  .personal-background {
    position: fixed;
    inset: 0;
    z-index: 0;
    pointer-events: none;
    overflow: hidden;
  }
  img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
