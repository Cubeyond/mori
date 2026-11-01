<script lang="ts">
  import Textarea from '#lib/components/ui/textarea/textarea.svelte';
  import Input from '#lib/components/ui/input/input.svelte';
  /** Readable sharing values remain usable even when clipboard access is unavailable. */
  import CopyButton from '../CopyButton.svelte';
  import type { Video } from '#lib/api/client';
  import { shareValue, type ShareKind } from '#lib/features/sharing';
  /** Only fully published video metadata is provided by the parent. */
  let { video }: { video: Video } = $props();
  /** Sharing exposes a direct video resource and fullscreen-capable iframe markup. */
  const fields: { kind: ShareKind; label: string }[] = $derived([
    {
      kind: 'media',
      label: video.kind === 'image' ? 'Direct image link' : 'Direct video link',
    },
    ...(video.kind === 'video'
      ? [{ kind: 'iframe' as const, label: 'Iframe embed code' }]
      : []),
  ]);
</script>

<div class="sharing-fields">
  {#each fields as field (field.kind)}
    <div class="share-field">
      <div class="field-heading">
        <span>{field.label}</span><CopyButton
          label={`Copy ${field.label.toLowerCase()}`}
          value={shareValue(video, field.kind)}
        />
      </div>
      <label class="sr-only" for={`share-${field.kind}`}>{field.label}</label>
      {#if field.kind === 'iframe'}<Textarea
          id={`share-${field.kind}`}
          readonly
          rows={5}
          value={shareValue(video, field.kind)}
          onfocus={(
            event: FocusEvent & { currentTarget: HTMLTextAreaElement },
          ) => {
            event.currentTarget.select();
          }}
          spellcheck="false"
        ></Textarea>
      {:else}<Input
          id={`share-${field.kind}`}
          readonly
          value={shareValue(video, field.kind)}
          onfocus={(
            event: FocusEvent & { currentTarget: HTMLInputElement },
          ) => {
            event.currentTarget.select();
          }}
          spellcheck="false"
        />{/if}
    </div>
  {/each}
</div>

<style>
  .share-field {
    margin-bottom: 17px;
  }
  .field-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 14px;
    color: #b6b9af;
  }
  .share-field :global(input),
  .share-field :global(textarea) {
    font-family: ui-monospace, monospace;
    font-size: 14px;
    line-height: 1.7;
  }
  .share-field :global(textarea) {
    resize: vertical;
    overflow-wrap: anywhere;
  }
</style>
