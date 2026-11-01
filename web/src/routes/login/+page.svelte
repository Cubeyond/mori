<script lang="ts">
  import { pageTitle } from '#lib/features/page-title';
  import Button from '#lib/components/ui/button/button.svelte';
  import Input from '#lib/components/ui/input/input.svelte';
  import { resolve } from '$app/paths';
  /** Page-specific state and handlers; all authorization remains server-enforced. */

  import { goto } from '$app/navigation';
  import { api, apiData, session, message } from '#lib/api/client';
  /** Transient password input cleared after successful authentication. */
  let password = $state('');
  /** The deployment's default account name remains editable for configured instances. */
  let username = $state('root');
  /** Safe error message for the active view. */
  let error = $state('');
  /** Whether password verification is awaiting a response. */
  let busy = $state(false);
  /** Establish a revocable administrator session and enter the video library. */
  async function submit(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    error = '';
    try {
      session.set(
        await apiData(
          api.POST('/v1/auth/login', { body: { username, password } }),
        ),
      );
      password = '';
      await goto(resolve('/gallery'));
    } catch (e) {
      error = message(e);
    } finally {
      busy = false;
    }
  }
</script>

<svelte:head><title>{pageTitle('Login')}</title></svelte:head>
<main class="mx-auto max-w-sm px-5 pb-12 pt-[12vh]">
  <p class="mb-3 text-xs tracking-[.2em] text-white/40">YOUR LIBRARY</p>
  <h1 class="serif mb-2 text-4xl">Welcome back.</h1>
  <p class="muted mb-9 text-sm">
    Log in to keep, arrange, and share your videos.
  </p>
  {#if $session.authenticated}<p class="muted mb-5 text-sm">
      You are signed in as the administrator.
    </p>
    <Button variant="default" class="button primary" href={resolve('/gallery')}
      >Open videos</Button
    >{:else}<form onsubmit={submit}>
      <label class="field"
        >Username<Input
          required
          maxlength={64}
          autocomplete="username"
          bind:value={username}
        /></label
      >
      <label class="field"
        >Administrator password<Input
          type="password"
          autocomplete="current-password"
          required
          maxlength={1024}
          bind:value={password}
        /></label
      >{#if error}<p class="error mb-4" role="alert">{error}</p>{/if}<Button
        variant="default"
        type="submit"
        class="button primary w-full"
        disabled={busy}>{busy ? 'Logging in…' : 'Log in'}</Button
      >
    </form>{/if}
</main>
