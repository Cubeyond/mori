<script lang="ts">
  /** Adapted shadcn-svelte primitive; see ../LICENSE.md for source and license. */
  import {
    cn,
    type WithElementRef,
    type WithoutChildren,
  } from '#lib/components/ui/utils';
  import type { HTMLTextareaAttributes } from 'svelte/elements';

  let {
    ref = $bindable(null),
    value = $bindable(),
    class: className,
    'data-slot': dataSlot = 'textarea',
    ...restProps
  }: WithoutChildren<WithElementRef<HTMLTextareaAttributes>> = $props();
</script>

<textarea
  bind:this={ref}
  data-slot={dataSlot}
  class={cn(
    'cn-textarea flex field-sizing-content min-h-16 w-full outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50',
    className,
  )}
  bind:value
  {...restProps}
></textarea>
