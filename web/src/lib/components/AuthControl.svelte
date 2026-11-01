<script lang="ts">
  /** Icon-only authentication actions for the single configured administrator. */
  import { resolve } from '$app/paths';
  import { goto } from '$app/navigation';
  import { LogIn, LogOut } from '@lucide/svelte';
  import { api, apiData, session, message } from '#lib/api/client';
  import Button from '#lib/components/ui/button/button.svelte';
  /** Failed logout leaves the session intact and presents retryable feedback. */
  let pending = $state(false),
    error = $state('');
  /** Revoke this browser session before returning to the public homepage. */
  async function logout(): Promise<void> {
    pending = true;
    error = '';
    try {
      await apiData(api.POST('/v1/auth/logout'));
      session.set({ authenticated: false, csrf: null });
      await goto(resolve('/'));
    } catch (failure) {
      error = message(failure);
    } finally {
      pending = false;
    }
  }
</script>

{#if $session.authenticated}<Button
    variant="ghost"
    size="icon"
    aria-label="Log out"
    title="Log out"
    disabled={pending}
    onclick={logout}><LogOut size={18} /></Button
  >
{:else}<Button
    variant="ghost"
    size="icon"
    href={resolve('/login')}
    aria-label="Log in"
    title="Log in"><LogIn size={18} /></Button
  >{/if}
{#if error}<p
    class="error fixed right-5 bottom-5 z-50 max-w-sm rounded-md border border-border bg-popover p-3"
    role="alert"
  >
    {error}
  </p>{/if}
