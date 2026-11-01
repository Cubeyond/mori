<script lang="ts">
  import { pageTitle } from '#lib/features/page-title';
  /** Browser-local preferences acknowledge changes beside the edited control. */
  import NativeSelect from '#lib/components/ui/native-select/native-select.svelte';
  import Breadcrumbs from '#lib/components/Breadcrumbs.svelte';
  import { resolve } from '$app/paths';
  import { Check, CircleAlert } from '@lucide/svelte';
  import { onDestroy } from 'svelte';
  import AccessGate from '#lib/components/AccessGate.svelte';
  import BackgroundSettings from '#lib/components/settings/BackgroundSettings.svelte';
  import {
    preferences,
    savePreferences,
    type Preferences,
  } from '#lib/features/preferences';
  /** Acknowledgement follows the last edited preference rather than the page title. */
  let changed = $state<keyof Preferences | null>(null);
  /** Persistent-storage failure remains visible instead of pretending the change was saved. */
  let notice = $state(''),
    persisted = $state(true);
  /** Rapid changes replace one acknowledgement without stacking timers. */
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => {
    if (noticeTimer !== undefined) clearTimeout(noticeTimer);
  });
  /** Persist one supported preference and acknowledge it beside its corresponding control. */
  function update<K extends keyof Preferences>(
    key: K,
    value: Preferences[K],
  ): void {
    persisted = savePreferences({ ...$preferences, [key]: value });
    changed = key;
    notice = persisted ? 'Saved' : 'Applied for this visit';
    if (noticeTimer !== undefined) clearTimeout(noticeTimer);
    if (persisted) noticeTimer = setTimeout(() => (notice = ''), 2400);
  }
</script>

<svelte:head><title>{pageTitle('Preferences')}</title></svelte:head>
<main>
  <Breadcrumbs items={[{ label: 'Preferences', href: resolve('/settings') }]} />
  <div class="page-heading"><h1>Preferences</h1></div>
  <AccessGate>
    <section class="settings-group" aria-label="Preferences">
      <BackgroundSettings />
      <div class="setting-row">
        <div class="setting-content">
          <label for="preference-density">Gallery density</label>
          <div class="setting-control preference-control">
            <NativeSelect
              id="preference-density"
              value={$preferences.density}
              onchange={(
                event: Event & { currentTarget: HTMLSelectElement },
              ) => {
                update(
                  'density',
                  event.currentTarget.value === 'compact'
                    ? 'compact'
                    : 'comfortable',
                );
              }}
              ><option value="comfortable">Comfortable</option><option
                value="compact">Compact</option
              ></NativeSelect
            >
            <span
              class="setting-feedback"
              class:temporary={!persisted &&
                changed === 'density' &&
                notice !== ''}
              role="status"
              aria-live="polite"
              aria-atomic="true"
              title={changed === 'density' && notice !== ''
                ? notice
                : undefined}
            >
              {#if changed === 'density' && notice !== ''}{#if persisted}<Check
                    size={20}
                    aria-hidden="true"
                  /><span class="sr-only">{notice}</span>{:else}<CircleAlert
                    size={18}
                    aria-hidden="true"
                  /><span>{notice}</span>{/if}{/if}
            </span>
          </div>
        </div>
      </div>
      <div class="setting-row">
        <div class="setting-content">
          <label for="preference-visibility">New upload visibility</label>
          <div class="setting-control preference-control">
            <NativeSelect
              id="preference-visibility"
              value={$preferences.uploadVisibility}
              onchange={(
                event: Event & { currentTarget: HTMLSelectElement },
              ) => {
                update(
                  'uploadVisibility',
                  event.currentTarget.value === 'public' ? 'public' : 'private',
                );
              }}
              ><option value="private">Private</option><option value="public"
                >Public</option
              ></NativeSelect
            >
            <span
              class="setting-feedback"
              class:temporary={!persisted &&
                changed === 'uploadVisibility' &&
                notice !== ''}
              role="status"
              aria-live="polite"
              aria-atomic="true"
              title={changed === 'uploadVisibility' && notice !== ''
                ? notice
                : undefined}
            >
              {#if changed === 'uploadVisibility' && notice !== ''}{#if persisted}<Check
                    size={20}
                    aria-hidden="true"
                  /><span class="sr-only">{notice}</span>{:else}<CircleAlert
                    size={18}
                    aria-hidden="true"
                  /><span>{notice}</span>{/if}{/if}
            </span>
          </div>
        </div>
      </div>
    </section>
  </AccessGate>
</main>

<style>
  .preference-control {
    display: grid;
    grid-template-columns: minmax(0, 240px) 24px;
    align-items: center;
    gap: 12px;
  }
  .preference-control :global([data-slot='native-select-wrapper']) {
    min-width: 0;
  }
  .setting-feedback {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: 24px;
    min-height: 24px;
    color: #b7ebcf;
  }
  .setting-feedback.temporary {
    grid-column: 1 / -1;
    width: 100%;
    justify-content: flex-start;
    color: #f4d69a;
    font-size: 14px;
    line-height: 22px;
  }
</style>
