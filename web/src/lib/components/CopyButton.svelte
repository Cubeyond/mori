<script lang="ts">
  /** Compact clipboard feedback stays inside the original control and never changes layout. */
  import { onDestroy } from 'svelte';
  import { Link, CodeXml, Copy, Check, CircleAlert } from '@lucide/svelte';
  import Button from './ui/button/button.svelte';
  /** Each caller supplies the exact selectable value and an accessible action name. */
  let {
    value,
    label,
    icon = 'copy',
    disabled = false,
    class: className = 'icon-button',
  }: {
    value: string;
    label: string;
    icon?: 'link' | 'code' | 'copy';
    disabled?: boolean;
    class?: string;
  } = $props();
  /** Success and failure remain local to this copy action. */
  let state = $state<'idle' | 'copied' | 'error'>('idle');
  /** Repeated copying restarts one bounded feedback timer. */
  let timer: ReturnType<typeof setTimeout> | undefined;
  /** Clipboard failure leaves manual sharing fields available without an unrelated toast. */
  async function copy(): Promise<void> {
    if (timer !== undefined) clearTimeout(timer);
    try {
      await navigator.clipboard.writeText(value);
      state = 'copied';
    } catch {
      state = 'error';
    }
    timer = setTimeout(() => (state = 'idle'), 1600);
  }
  onDestroy(() => {
    if (timer !== undefined) clearTimeout(timer);
  });
</script>

<Button
  variant="ghost"
  size="icon"
  type="button"
  class={className}
  {disabled}
  aria-label={label}
  title={state === 'copied'
    ? 'Copied'
    : state === 'error'
      ? 'Copy failed. Select the value in Share.'
      : label}
  data-copy-state={state}
  onclick={copy}
>
  {#if state === 'copied'}<Check
      size={18}
    />{:else if state === 'error'}<CircleAlert size={18} />
  {:else if icon === 'link'}<Link size={18} />{:else if icon === 'code'}<CodeXml
      size={18}
    />{:else}<Copy size={18} />{/if}
</Button>
<span class="sr-only" role="status"
  >{state === 'copied'
    ? 'Copied to clipboard.'
    : state === 'error'
      ? 'Copy failed. Select the sharing value manually.'
      : ''}</span
>
