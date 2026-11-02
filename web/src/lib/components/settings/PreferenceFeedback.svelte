<script lang="ts">
  /** Reusable per-control acknowledgement keeps settings feedback quiet and consistently aligned. */
  import { Check, CircleAlert } from '@lucide/svelte';
  /** A persistence failure stays visible; successful changes use a brief check mark. */
  let {
    visible,
    persisted,
    notice,
  }: { visible: boolean; persisted: boolean; notice: string } = $props();
</script>

<span
  class="feedback"
  class:temporary={visible && !persisted}
  role="status"
  aria-live="polite"
  aria-atomic="true"
  title={visible ? notice : undefined}
>
  {#if visible}{#if persisted}<Check size={20} aria-hidden="true" /><span
        class="sr-only">{notice}</span
      >{:else}<CircleAlert size={18} aria-hidden="true" /><span>{notice}</span
      >{/if}{/if}
</span>

<style>
  .feedback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 24px;
    min-height: 24px;
    color: var(--success);
  }
  .temporary {
    grid-column: 1 / -1;
    width: 100%;
    justify-content: flex-start;
    color: var(--warning);
    font-size: 14px;
    line-height: 22px;
  }
</style>
