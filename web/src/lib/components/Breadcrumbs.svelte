<script lang="ts">
  /** Linked location hierarchy with an accessible current-page marker. */
  import type { ResolvedPathname } from '$app/types';
  import { ChevronRight } from '@lucide/svelte';
  /** Routes are resolved by each caller so base-path handling remains centralized in SvelteKit. */
  let { items }: { items: { label: string; href: ResolvedPathname }[] } =
    $props();
</script>

{#if items.length > 1}<nav class="breadcrumb" aria-label="Breadcrumb">
    {#each items as item, index (index)}
      {#if index > 0}<ChevronRight size={14} aria-hidden="true" />{/if}
      {#if index === items.length - 1}<span aria-current="page"
          >{item.label}</span
        >{:else}<a href={item.href}>{item.label}</a>{/if}
    {/each}
  </nav>{/if}
