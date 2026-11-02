<script lang="ts">
  /** Shared native alpha slider keeps background and thumbnail controls aligned and accessible. */
  import { Check, RotateCcw } from '@lucide/svelte';
  import Button from '#lib/components/ui/button/button.svelte';
  /** The caller owns live preview and persistence; this control always uses normalized alpha. */
  let {
    id,
    label,
    value,
    defaultValue,
    oninput,
    oncommit,
    saved = false,
  }: {
    id: string;
    label: string;
    value: number;
    defaultValue: number;
    oninput: (value: number) => void;
    oncommit: () => void;
    saved?: boolean;
  } = $props();
</script>

<div class="opacity-control">
  <input
    {id}
    type="range"
    aria-label={label}
    min="0"
    max="100"
    step="1"
    value={Math.round(value * 100)}
    aria-valuetext={String(Math.round(value * 100)) + ' percent'}
    oninput={(event) => {
      oninput(Number(event.currentTarget.value) / 100);
    }}
    onchange={oncommit}
  />
  <output for={id}>{Math.round(value * 100)}%</output>
  <Button
    variant="ghost"
    size="icon"
    type="button"
    aria-label={'Reset ' +
      label.toLowerCase() +
      ' to ' +
      String(Math.round(defaultValue * 100)) +
      ' percent'}
    onclick={() => {
      oninput(defaultValue);
      oncommit();
    }}
    >{#if saved}<Check size={17} class="text-(--success)" />{:else}<RotateCcw
        size={17}
      />{/if}</Button
  >
</div>

<style>
  .opacity-control {
    display: grid;
    grid-template-columns: minmax(100px, 240px) 48px 40px;
    align-items: center;
    gap: 12px;
    margin-top: 0;
  }
  input[type='range'] {
    width: 100%;
    min-height: 44px;
    accent-color: var(--primary);
    cursor: pointer;
  }
  output {
    font-size: 15px;
    font-variant-numeric: tabular-nums;
    text-align: right;
    color: var(--foreground);
  }
</style>
