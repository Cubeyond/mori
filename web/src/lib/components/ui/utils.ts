/** Typed support for locally adopted shadcn-svelte primitives. */
export { cn } from 'cn';
/** Element reference exposed for native control bindings. */
export type WithElementRef<T, U extends HTMLElement = HTMLElement> = T & {
  ref?: U | null;
};
/** Content wrappers own their rendered child snippets. */
export type WithoutChildrenOrChild<T> = Omit<T, 'children' | 'child'>;

/** Textarea primitives own their value instead of child snippets. */
export type WithoutChildren<T> = Omit<T, 'children'>;
