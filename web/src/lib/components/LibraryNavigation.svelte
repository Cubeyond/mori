<script lang="ts">
  /** The desktop sidebar and full-page mobile navigation share destinations and active-route semantics. */
  import { resolve } from '$app/paths';
  import { Video, Library, Settings2, HardDrive } from '@lucide/svelte';
  /** Mobile navigation follows the same routes with larger touch rows. */
  let {
    path,
    variant = 'sidebar',
    onfollow,
  }: {
    path: string;
    variant?: 'sidebar' | 'drawer';
    onfollow?: () => void;
  } = $props();
</script>

<div class="navigation" class:drawer={variant === 'drawer'}>
  <nav aria-label="Library">
    <p class="category">Library</p>
    <a
      href={resolve('/videos')}
      aria-current={['/videos', '/manage'].includes(path) ? 'page' : undefined}
      onclick={onfollow}
    >
      <Video size={22} /><span>Videos</span>
    </a>
    <a
      href={resolve('/albums')}
      aria-current={path.startsWith('/albums') ? 'page' : undefined}
      onclick={onfollow}
    >
      <Library size={22} /><span>Albums</span>
    </a>
  </nav>
  <nav aria-label="Settings">
    <p class="category">Settings</p>
    <a
      href={resolve('/settings')}
      aria-current={path === '/settings' ? 'page' : undefined}
      onclick={onfollow}
    >
      <Settings2 size={22} /><span>Preferences</span>
    </a>
    <a
      href={resolve('/settings/storage')}
      aria-current={path === '/settings/storage' ? 'page' : undefined}
      onclick={onfollow}
    >
      <HardDrive size={22} /><span>Storage</span>
    </a>
  </nav>
</div>

<style>
  nav {
    display: grid;
    gap: 4px;
    margin-bottom: 24px;
  }
  nav:last-child {
    margin-bottom: 0;
  }
  .category {
    color: #858593;
    font-size: 15px;
    line-height: 22px;
    margin: 0 0 2px;
    padding-left: 8px;
  }
  a {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 44px;
    padding: 8px 12px;
    font-size: 16px;
    line-height: 24px;
    font-weight: 500;
    color: #e4e4eb;
    border-radius: 5px;
  }
  a:hover {
    background: #ffffff08;
  }
  a[aria-current='page'] {
    background: #ffffff0b;
    color: #fff;
  }
  .drawer nav {
    gap: 6px;
    margin-bottom: 28px;
  }
  .drawer .category {
    font-size: 15px;
    line-height: 22px;
    padding-left: 12px;
    margin-bottom: 4px;
  }
  .drawer a {
    min-height: 60px;
    gap: 16px;
    padding: 16px;
    font-size: 20px;
    line-height: 28px;
    border-radius: 8px;
  }
</style>
