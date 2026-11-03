<script lang="ts">
  /** Shared collection search keeps labels, counts, and keyboard behavior consistent. */
  import { Search } from '@lucide/svelte';
  import Input from '#lib/components/ui/input/input.svelte';
  /** Counts represent all authorized matches, including pages not yet loaded. */
  let {
    query = $bindable(''),
    count,
    noun = 'item',
    label = 'Search media',
  }: {
    query?: string;
    count: number;
    noun?: 'item' | 'album';
    label?: string;
  } = $props();
</script>

<div class="collection-toolbar">
  <label class="search-field"
    ><Search size={18} /><span class="sr-only">{label}</span><Input
      type="search"
      maxlength={200}
      placeholder={label + '…'}
      bind:value={query}
    /></label
  >
  <span class="collection-count" aria-live="polite"
    >{count} {noun}{count === 1 ? '' : 's'}</span
  >
</div>
