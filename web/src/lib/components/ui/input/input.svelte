<script lang="ts">
  /** Adapted shadcn-svelte primitive; see ../LICENSE.md for source and license. */
  import { cn, type WithElementRef } from '#lib/components/ui/utils';
  import type {
    HTMLInputAttributes,
    HTMLInputTypeAttribute,
  } from 'svelte/elements';

  /** Native attributes supported by this primitive. */
  type InputType = Exclude<HTMLInputTypeAttribute, 'file'>;

  /** Native attributes supported by this primitive. */
  type Props = WithElementRef<
    Omit<HTMLInputAttributes, 'type' | 'value'> & {
      value?: string | number;
    } & (
        | { type: 'file'; files?: FileList }
        | { type?: InputType; files?: undefined }
      ),
    HTMLInputElement
  >;

  let {
    ref = $bindable(null),
    value = $bindable<string | number>(''),
    type,
    files = $bindable(),
    class: className,
    'data-slot': dataSlot = 'input',
    ...restProps
  }: Props = $props();
</script>

{#if type === 'file'}
  <input
    bind:this={ref}
    data-slot={dataSlot}
    class={cn(
      'cn-input w-full min-w-0 outline-none file:inline-flex file:border-0 file:bg-transparent file:text-foreground placeholder:text-muted-foreground disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50',
      className,
    )}
    type="file"
    bind:files
    bind:value
    {...restProps}
  />
{:else}
  <input
    bind:this={ref}
    data-slot={dataSlot}
    class={cn(
      'cn-input w-full min-w-0 outline-none file:inline-flex file:border-0 file:bg-transparent file:text-foreground placeholder:text-muted-foreground disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50',
      className,
    )}
    {type}
    bind:value
    {...restProps}
  />
{/if}
