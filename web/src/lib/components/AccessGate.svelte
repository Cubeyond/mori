<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: Elastic-2.0 -->

<script lang="ts">
  import Button from '#lib/components/ui/button/button.svelte';
  /** Session-aware page boundary; protected data requests still require server authorization. */
  import { onMount, type Snippet } from 'svelte';
  import { resolve } from '$app/paths';
  import { loadSession, session, message } from '#lib/api/client';
  import { bootstrap } from '#lib/api/bootstrap';
  /** Contents are mounted only after an authenticated session is confirmed. */
  let { children }: { children: Snippet } = $props();
  /** A failed session lookup is distinct from a confirmed anonymous visitor. */
  let loaded = $state(bootstrap !== null),
    error = $state('');
  onMount(() => {
    if (bootstrap !== null) return;
    void loadSession()
      .catch((failure: unknown) => (error = message(failure)))
      .finally(() => (loaded = true));
  });
</script>

{#if !loaded}<p class="empty-state muted" role="status">Loading…</p>
{:else if error}<p class="empty-state error" role="alert">{error}</p>
{:else if $session.authenticated}{@render children()}
{:else}<div class="empty-state">
    <h2>Log in to continue</h2>
    <p>Your library is only visible to the administrator.</p>
    <Button variant="default" class="button primary" href={resolve('/login')}
      >Login</Button
    >
  </div>{/if}
