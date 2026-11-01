<script lang="ts">
  import { buttonVariants, type ButtonProps } from './variants';
  import { cn } from '#lib/components/ui/utils';
  /** Adapted shadcn-svelte primitive; see ../LICENSE.md for source and license. */
  let {
    class: className,
    variant = 'default',
    size = 'default',
    ref = $bindable(null),
    href,
    /** Native attributes supported by this primitive. */
    type = 'button',
    disabled,
    children,
    ...restProps
  }: ButtonProps = $props();
</script>

{#if href !== undefined}
  <a
    bind:this={ref}
    data-slot="button"
    class={cn(buttonVariants({ variant, size }), className)}
    href={disabled === true ? undefined : href}
    aria-disabled={disabled}
    role={disabled === true ? 'link' : undefined}
    tabindex={disabled === true ? -1 : undefined}
    {...restProps}
  >
    {@render children?.()}
  </a>
{:else}
  <button
    bind:this={ref}
    data-slot="button"
    class={cn(buttonVariants({ variant, size }), className)}
    {type}
    {disabled}
    {...restProps}
  >
    {@render children?.()}
  </button>
{/if}
