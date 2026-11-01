<script lang="ts">
  /** Adapted shadcn-svelte primitive; see ../LICENSE.md for source and license. */
  import { ChevronDown } from '@lucide/svelte';
  import { cn, type WithElementRef } from '#lib/components/ui/utils';
  import type { HTMLSelectAttributes } from 'svelte/elements';

  /** Native attributes supported by this primitive. */
  type NativeSelectProps = Omit<
    WithElementRef<HTMLSelectAttributes, HTMLSelectElement>,
    'size' | 'value'
  > & { value?: string } & {
    size?: 'sm' | 'default';
  };

  let {
    ref = $bindable(null),
    value = $bindable<string>(''),
    class: className,
    size = 'default',
    children,
    ...restProps
  }: NativeSelectProps = $props();
</script>

<div
  class={cn(
    'cn-native-select-wrapper group/native-select relative w-full has-[select:disabled]:opacity-50',
    className,
  )}
  data-slot="native-select-wrapper"
  data-size={size}
>
  <select
    bind:value
    bind:this={ref}
    data-slot="native-select"
    data-size={size}
    class="cn-native-select block outline-none disabled:pointer-events-none disabled:cursor-not-allowed"
    {...restProps}
  >
    {@render children?.()}
  </select>
  <ChevronDown
    class="cn-native-select-icon pointer-events-none absolute select-none"
    aria-hidden="true"
  />
</div>
