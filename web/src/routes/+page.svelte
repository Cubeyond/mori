<script lang="ts">
  import { pageTitle } from '#lib/features/page-title';
  /** The welcome page offers real entry points without exposing public library metadata. */
  import { resolve } from '$app/paths';
  import {
    ArrowRight,
    Upload,
    Images,
    Library,
    LogIn,
    Link,
  } from '@lucide/svelte';
  import Button from '#lib/components/ui/button/button.svelte';
  import { session } from '#lib/api/client';
</script>

<svelte:head>
  <title>{pageTitle()}</title>
  <meta
    name="description"
    content="Images and videos, collected privately and shared simply."
  />
</svelte:head>
<main class="home">
  <div class="introduction">
    <h1>Images &amp; videos.<br />Simply shared.</h1>
    <p class="description">
      One home for your images and videos. Keep them private, collect them in
      albums, and share what you choose.
    </p>
    {#if $session.authenticated}
      <Button
        class="button primary"
        onclick={() => window.dispatchEvent(new Event('mori:upload'))}
      >
        <Upload size={20} />Upload media
      </Button>
    {:else}
      <Button class="button primary" href={resolve('/login')}
        ><LogIn size={20} />Log in</Button
      >
    {/if}
  </div>
  <section
    class="home-panel"
    aria-label={$session.authenticated
      ? 'Your library'
      : 'Watching and sharing'}
  >
    {#if $session.authenticated}
      <a class="destination" href={resolve('/gallery')}>
        <span class="destination-icon"><Images size={24} /></span>
        <span
          ><h2>Gallery</h2>
          <p>Every image and video, together.</p></span
        >
        <ArrowRight size={20} />
      </a>
      <a class="destination" href={resolve('/albums')}>
        <span class="destination-icon"><Library size={24} /></span>
        <span
          ><h2>Albums</h2>
          <p>Mix images and videos. Share one collection.</p></span
        >
        <ArrowRight size={20} />
      </a>
    {:else}
      <div class="destination">
        <span class="destination-icon"><Link size={24} /></span>
        <div>
          <h2>Here to explore?</h2>
          <p>Open a shared image, video, or album. No account needed.</p>
        </div>
      </div>
      <div class="destination">
        <span class="destination-icon"><Library size={24} /></span>
        <div>
          <h2>A library of your own</h2>
          <p>Log in to upload, organize, and share your images and videos.</p>
        </div>
      </div>
    {/if}
  </section>
</main>

<style>
  .home {
    max-width: 1120px;
    margin: clamp(28px, 7vh, 72px) auto 32px;
    display: grid;
    grid-template-columns: 1.15fr 1fr;
    align-items: center;
    gap: clamp(32px, 5vw, 72px);
  }
  h1 {
    font-family: 'Mori Serif', Georgia, serif;
    font-size: clamp(52px, 5.5vw, 76px);
    line-height: 1.04;
    font-weight: 400;
    letter-spacing: -0.035em;
    color: #f0ece5;
  }
  .description {
    max-width: 460px;
    font-size: 20px;
    line-height: 1.6;
    color: #a5a5b0;
    margin: 24px 0 28px;
    text-wrap: pretty;
  }
  .home-panel {
    background: #ffffff03;
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
  }
  .destination {
    display: flex;
    align-items: center;
    gap: 18px;
    padding: 28px 24px;
  }
  .destination + .destination {
    border-top: 1px solid var(--border);
  }
  a.destination:hover {
    background: #ffffff05;
  }
  .destination-icon {
    align-self: flex-start;
    display: flex;
    align-items: center;
    min-height: 28px;
    color: #b7b7c3;
  }
  .destination > :global(svg:last-child) {
    margin-left: auto;
    color: #92929e;
  }
  h2 {
    font-size: 19px;
    line-height: 28px;
    font-weight: 500;
    letter-spacing: -0.025em;
  }
  .destination p {
    margin-top: 6px;
    font-size: 16px;
    line-height: 1.6;
    color: #9c9caa;
    text-wrap: pretty;
  }
  @media (max-width: 767px) {
    .home {
      grid-template-columns: minmax(0, 1fr);
      gap: 36px;
      margin: 16px auto;
      max-width: 560px;
    }
    h1 {
      font-size: clamp(48px, 10vw, 64px);
    }
    .description {
      font-size: 18px;
      margin: 20px 0 24px;
    }
    .destination {
      padding: 24px 20px;
      gap: 16px;
    }
  }
</style>
