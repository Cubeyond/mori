/** Button variants adapted from the shadcn-svelte registry. */
/** Shared shadcn-svelte button variants and typed native attributes. */
import { type VariantProps, tv } from 'tailwind-variants';
import { type WithElementRef } from '#lib/components/ui/utils';
import type { ResolvedPathname } from '$app/types';
import type {
  HTMLAnchorAttributes,
  HTMLButtonAttributes,
} from 'svelte/elements';

/** Official shadcn-svelte visual variants, shared by button and menu triggers. */
export const buttonVariants = tv({
  base: 'cn-button group/button inline-flex shrink-0 items-center justify-center whitespace-nowrap transition-all outline-none select-none disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0',
  variants: {
    variant: {
      default: 'cn-button-variant-default',
      outline: 'cn-button-variant-outline',
      secondary: 'cn-button-variant-secondary',
      ghost: 'cn-button-variant-ghost',
      destructive: 'cn-button-variant-destructive',
      link: 'cn-button-variant-link',
    },
    size: {
      default: 'cn-button-size-default',
      xs: 'cn-button-size-xs',
      sm: 'cn-button-size-sm',
      lg: 'cn-button-size-lg',
      icon: 'cn-button-size-icon',
      'icon-xs': 'cn-button-size-icon-xs',
      'icon-sm': 'cn-button-size-icon-sm',
      'icon-lg': 'cn-button-size-icon-lg',
    },
  },
  defaultVariants: {
    variant: 'default',
    size: 'default',
  },
});

/** Native attributes supported by this primitive. */
type ButtonVariant = VariantProps<typeof buttonVariants>['variant'];
/** Native attributes supported by this primitive. */
type ButtonSize = VariantProps<typeof buttonVariants>['size'];

/** Native attributes supported by this primitive. */
export type ButtonProps = WithElementRef<HTMLButtonAttributes> &
  WithElementRef<HTMLAnchorAttributes> & {
    href?: ResolvedPathname;
    variant?: ButtonVariant;
    size?: ButtonSize;
  };
