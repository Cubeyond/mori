<script lang="ts">
  /** Retained completion metadata remains readable after the editable queue has emptied. */
  import { ArrowRight, Check } from '@lucide/svelte';
  import { formatBytes } from '#lib/features/upload-selection';
  import {
    summarizeUploads,
    type UploadResult,
  } from '#lib/features/upload-results';
  /** Results contain metadata only; original blobs and compression copies can be released immediately. */
  let { results }: { results: readonly UploadResult[] } = $props();
  /** Derive the weighted total from all successes in this batch, including across pause/resume. */
  const summary = $derived(summarizeUploads(results));
</script>

<section class="upload-results" aria-label="Upload results">
  <div class="result-summary" role="status">
    <div class="summary-heading">
      <Check size={18} /><span
        >{summary.count === 1
          ? results[0]?.name
          : String(summary.count) + ' files uploaded'}</span
      >
    </div>
    <div class="size-comparison" aria-label="Total original and uploaded size">
      <span>{formatBytes(summary.originalBytes)}</span><ArrowRight
        size={16}
        aria-hidden="true"
      /><strong>{formatBytes(summary.uploadedBytes)}</strong>
    </div>
    <p>
      {summary.savedBytes > 0
        ? 'Saved ' +
          formatBytes(summary.savedBytes) +
          ' · ' +
          String(summary.savedPercent) +
          '% smaller'
        : 'Original files kept'}
    </p>
  </div>
  {#if results.length > 1}<div
      class="file-results"
      role="list"
      aria-label="Completed files"
    >
      {#each results as result (result.id)}
        <div class="file-result" role="listitem">
          <span class="result-name">{result.name}</span>
          <div class="size-comparison">
            <span class="sr-only">Original size </span><span
              >{formatBytes(result.originalBytes)}</span
            ><ArrowRight size={13} aria-hidden="true" /><span class="sr-only"
              >Uploaded size
            </span><strong>{formatBytes(result.uploadedBytes)}</strong>
          </div>
          <span class="result-saving"
            >{result.uploadedBytes < result.originalBytes
              ? String(
                  Math.round(
                    (1 - result.uploadedBytes / result.originalBytes) * 100,
                  ),
                ) + '% smaller'
              : 'Original kept'}</span
          >
        </div>
      {/each}
    </div>{/if}
</section>

<style>
  .upload-results {
    margin-block: 4px 0;
  }
  .result-summary {
    display: grid;
    gap: 10px;
    padding-bottom: 20px;
  }
  .summary-heading {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
    color: var(--muted-foreground);
  }
  .summary-heading span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .summary-heading :global(svg) {
    color: var(--primary);
  }
  .size-comparison {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
    font-size: 13px;
    font-variant-numeric: tabular-nums;
    color: var(--muted-foreground);
  }
  .size-comparison strong {
    font-weight: 500;
    color: var(--foreground);
  }
  .result-summary > .size-comparison {
    font-size: 25px;
    gap: 12px;
    line-height: 1.4;
  }
  .result-summary p {
    color: var(--muted-foreground);
    font-size: 14px;
    margin: 0;
  }
  .file-results {
    display: grid;
    border-top: 1px solid var(--border);
  }
  .file-result {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    align-items: center;
    gap: 6px 16px;
    padding-block: 14px;
    border-bottom: 1px solid var(--border);
  }
  .file-result:last-child {
    border-bottom: 0;
    padding-bottom: 0;
  }
  .result-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 14px;
    min-width: 0;
  }
  .result-saving {
    font-size: 12px;
    color: var(--muted-foreground);
    grid-column: 1 / -1;
  }
  @media (max-width: 480px) {
    .file-result {
      grid-template-columns: minmax(0, 1fr);
      gap: 6px;
    }
    .file-result > .size-comparison {
      font-size: 14px;
    }
    .result-summary > .size-comparison {
      font-size: 23px;
    }
  }
</style>
