<script lang="ts">
  import Button from '#lib/components/ui/button/button.svelte';
  /** Minimal header and grouped navigation for authenticated dashboard pages. */
  import '../lib/styles/app.css';
  import { onMount, type Snippet } from 'svelte';
  import { page } from '$app/state';
  import { afterNavigate } from '$app/navigation';
  import { Menu } from '@lucide/svelte';
  import Wordmark from '#lib/components/Wordmark.svelte';
  import LibraryNavigation from '#lib/components/LibraryNavigation.svelte';
  import MobileNavigation from '#lib/components/MobileNavigation.svelte';
  import AuthControl from '#lib/components/AuthControl.svelte';
  import UploadController from '#lib/components/library/UploadController.svelte';
  import { loadSession, session } from '#lib/api/client';
  import { loadPreferences } from '#lib/features/preferences';
  import { bootstrap } from '#lib/api/bootstrap';
  import BackgroundLayer from '#lib/components/BackgroundLayer.svelte';
  import { hasBackground } from '#lib/features/background';
  /** Active route contents, rendered once to preserve upload and player state. */
  let { children }: { children: Snippet } = $props();
  /** The compact navigation disclosure is controlled independently of the page. */
  let navigationOpen = $state(false);
  /** Only authenticated dashboard routes display the administration sidebar. */
  const dashboard = $derived(
    $session.authenticated &&
      (['/gallery', '/videos', '/manage', '/albums', '/settings'].includes(
        page.url.pathname,
      ) ||
        page.url.pathname.startsWith('/albums/') ||
        page.url.pathname.startsWith('/settings/')),
  );
  /** Avoid showing anonymous controls while the session is still being discovered. */
  let sessionReady = $state(bootstrap !== null);
  onMount(() => {
    loadPreferences();
    if (bootstrap !== null) return;
    void loadSession()
      .catch(() => undefined)
      .finally(() => (sessionReady = true));
  });
  afterNavigate(() => {
    navigationOpen = false;
  });
</script>

<BackgroundLayer />
<div class="application" class:personalized={$hasBackground}>
  <a class="skip-link" href="#page-content">Skip to content</a>
  <header class="topbar">
    <div class="header-inner">
      <div class="brand">
        {#if dashboard}<Button
            variant="ghost"
            size="icon"
            type="submit"
            class="icon-button navigation-toggle"
            aria-label="Open navigation"
            aria-haspopup="dialog"
            aria-expanded={navigationOpen}
            aria-controls="mobile-navigation"
            onclick={() => (navigationOpen = true)}
          >
            <Menu size={20} />
          </Button>{/if}
        <Wordmark />
      </div>
      <nav aria-label="Account navigation">
        <div class="account-slot">
          {#if sessionReady}<AuthControl />{/if}
        </div>
      </nav>
    </div>
  </header>
  <div class="workspace" class:dashboard>
    {#if dashboard}<aside id="sidebar" aria-label="Sidebar">
        <LibraryNavigation path={page.url.pathname} />
      </aside>{/if}
    <div id="page-content" class="page-content" tabindex="-1">
      {@render children()}
    </div>
  </div>
  {#if dashboard}<MobileNavigation
      open={navigationOpen}
      path={page.url.pathname}
      onclose={() => (navigationOpen = false)}
    />{/if}
  {#if $session.authenticated}<UploadController />{/if}
</div>

<style>
  .application {
    position: relative;
    z-index: 1;
    min-height: 100svh;
  }
  .personalized .page-content,
  .personalized aside {
    background: rgb(11 12 20 / 95%);
    border-radius: 8px;
  }
  .personalized .page-content {
    padding: 24px;
  }
  .personalized aside {
    padding: 12px;
  }
  .topbar {
    min-height: 56px;
    background: #0d0e15;
    border-bottom: 1px solid #ffffff0d;
  }
  .header-inner {
    max-width: 1400px;
    height: 56px;
    margin: 0 auto;
    padding: 0 32px;
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .brand {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .account-slot {
    min-width: 56px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
  }
  .workspace {
    max-width: 1400px;
    padding: 24px 32px 48px;
    margin: 0 auto;
    display: block;
  }
  .workspace.dashboard {
    display: grid;
    grid-template-columns: 200px minmax(0, 1fr);
    gap: 40px;
    align-items: start;
  }
  aside {
    position: sticky;
    top: 24px;
  }
  .page-content {
    min-width: 0;
    outline: none;
  }
  :global(.navigation-toggle) {
    display: none;
  }
  .skip-link {
    position: fixed;
    z-index: 100;
    top: 8px;
    left: 8px;
    padding: 12px 16px;
    background: #f4f4f5;
    color: #111;
    transform: translateY(-150%);
  }
  .skip-link:focus {
    transform: translateY(0);
  }
  @media (max-width: 1023px) {
    .workspace.dashboard {
      grid-template-columns: 176px minmax(0, 1fr);
      gap: 24px;
    }
  }
  @media (max-width: 767px) {
    .topbar,
    .header-inner {
      min-height: 56px;
      height: 56px;
    }
    .header-inner {
      padding: 0 20px;
    }
    :global(.navigation-toggle) {
      display: inline-flex;
    }
    .workspace,
    .workspace.dashboard {
      display: flex;
      flex-direction: column;
      padding: 24px 16px 32px;
      gap: 24px;
    }
    aside {
      display: none;
    }
    .page-content {
      width: 100%;
    }
    .personalized .page-content {
      padding: 16px;
    }
  }
</style>
