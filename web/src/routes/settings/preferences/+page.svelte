<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: Elastic-2.0 -->

<script lang="ts">
  import { pageTitle } from '#lib/features/page-title';
  /** Browser-local preferences acknowledge changes beside the edited control. */
  import NativeSelect from '#lib/components/ui/native-select/native-select.svelte';
  import { onDestroy } from 'svelte';
  import { Check } from '@lucide/svelte';
  import AccessGate from '#lib/components/AccessGate.svelte';
  import BackgroundSettings from '#lib/components/settings/BackgroundSettings.svelte';
  import ThumbnailSettings from '#lib/components/settings/ThumbnailSettings.svelte';
  import PreferenceFeedback from '#lib/components/settings/PreferenceFeedback.svelte';
  import {
    optimizationModes,
    parseOptimizationMode,
  } from '#lib/features/optimization/model';
  import { themes, parseTheme, accents } from '#lib/features/theme';
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
  <div class="page-heading"><h1>Preferences</h1></div>
  <AccessGate>
    <div class="preferences-groups">
      <section class="settings-group" aria-labelledby="appearance-heading">
        <header class="settings-group-heading">
          <h2 id="appearance-heading">Appearance</h2>
        </header>
        <div class="setting-row">
          <div class="setting-content">
            <span id="preference-theme-label" class="setting-label">Theme</span>
            <div class="setting-control preference-control">
              <NativeSelect
                id="preference-theme"
                aria-labelledby="preference-theme-label"
                value={$preferences.theme}
                onchange={(
                  event: Event & { currentTarget: HTMLSelectElement },
                ) => {
                  update('theme', parseTheme(event.currentTarget.value));
                }}
              >
                {#each themes as theme (theme.id)}<option value={theme.id}
                    >{theme.label}</option
                  >{/each}
              </NativeSelect>
              <PreferenceFeedback
                visible={changed === 'theme' && notice !== ''}
                {persisted}
                {notice}
              />
            </div>
          </div>
        </div>
        <div class="setting-row">
          <div class="setting-content">
            <span id="accent-label" class="setting-label">Accent</span>
            <div class="setting-control preference-control">
              <div
                class="accent-options"
                role="group"
                aria-labelledby="accent-label"
              >
                {#each accents as accent (accent)}<button
                    type="button"
                    class="accent-swatch"
                    aria-label={'Accent ' + accent}
                    aria-pressed={$preferences.accent === accent}
                    style:--swatch={'var(--accent-' + accent + ')'}
                    style:--swatch-text={'var(--on-' + accent + ')'}
                    onclick={() => {
                      update('accent', accent);
                    }}
                  >
                    {#if $preferences.accent === accent}<Check size={15} />{/if}
                  </button>{/each}
              </div>
              <PreferenceFeedback
                visible={changed === 'accent' && notice !== ''}
                {persisted}
                {notice}
              />
            </div>
          </div>
        </div>
      </section>
      <section class="settings-group" aria-labelledby="background-heading">
        <header class="settings-group-heading">
          <h2 id="background-heading">Background</h2>
        </header>
        <BackgroundSettings />
      </section>
      <section class="settings-group" aria-labelledby="gallery-heading">
        <header class="settings-group-heading">
          <h2 id="gallery-heading">Gallery</h2>
        </header>
        <div class="setting-row">
          <div class="setting-content">
            <span id="preference-density-label" class="setting-label"
              >Gallery density</span
            >
            <div class="setting-control preference-control">
              <NativeSelect
                id="preference-density"
                aria-labelledby="preference-density-label"
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
              <PreferenceFeedback
                visible={changed === 'density' && notice !== ''}
                {persisted}
                {notice}
              />
            </div>
          </div>
        </div>
        <ThumbnailSettings />
      </section>
      <section class="settings-group" aria-labelledby="uploads-heading">
        <header class="settings-group-heading">
          <h2 id="uploads-heading">Uploads</h2>
        </header>
        <div class="setting-row">
          <div class="setting-content optimization-setting">
            <span id="preference-optimization-label" class="setting-label"
              >Upload optimization</span
            >
            <div class="setting-control preference-control">
              <NativeSelect
                id="preference-optimization"
                aria-labelledby="preference-optimization-label"
                value={$preferences.uploadOptimization}
                onchange={(
                  event: Event & { currentTarget: HTMLSelectElement },
                ) => {
                  update(
                    'uploadOptimization',
                    parseOptimizationMode(event.currentTarget.value),
                  );
                }}
              >
                {#each optimizationModes as mode (mode.value)}<option
                    value={mode.value}>{mode.label}</option
                  >{/each}
              </NativeSelect>
              <PreferenceFeedback
                visible={changed === 'uploadOptimization' && notice !== ''}
                {persisted}
                {notice}
              />
            </div>
            <p class="optimization-hint">
              {optimizationModes.find(
                (mode) => mode.value === $preferences.uploadOptimization,
              )?.description}
            </p>
          </div>
        </div>
        <div class="setting-row">
          <div class="setting-content">
            <span id="preference-visibility-label" class="setting-label"
              >New upload visibility</span
            >
            <div class="setting-control preference-control">
              <NativeSelect
                id="preference-visibility"
                aria-labelledby="preference-visibility-label"
                value={$preferences.uploadVisibility}
                onchange={(
                  event: Event & { currentTarget: HTMLSelectElement },
                ) => {
                  update(
                    'uploadVisibility',
                    event.currentTarget.value === 'public'
                      ? 'public'
                      : 'private',
                  );
                }}
                ><option value="private">Private</option><option value="public"
                  >Public</option
                ></NativeSelect
              >
              <PreferenceFeedback
                visible={changed === 'uploadVisibility' && notice !== ''}
                {persisted}
                {notice}
              />
            </div>
          </div>
        </div>
      </section>
    </div>
  </AccessGate>
</main>

<style>
  .preferences-groups {
    display: grid;
    gap: 24px;
  }
  .preference-control {
    display: grid;
    grid-template-columns: minmax(0, 240px) 24px;
    align-items: center;
    gap: 12px;
  }
  .preference-control :global([data-slot='native-select-wrapper']) {
    min-width: 0;
  }
  .optimization-setting {
    row-gap: 8px;
  }
  .optimization-hint {
    grid-column: 2;
    max-width: 240px;
    margin: 0;
    font-size: 14px;
    line-height: 1.5;
    color: var(--muted-foreground);
  }
  @media (max-width: 600px) {
    .optimization-hint {
      grid-column: 1;
    }
  }
  .accent-options {
    display: flex;
    gap: 2px;
    flex-wrap: wrap;
    max-width: 240px;
  }
  .accent-swatch {
    display: grid;
    place-items: center;
    width: 28px;
    height: 36px;
    border-radius: 7px;
    position: relative;
    color: var(--swatch-text);
    isolation: isolate;
  }
  .accent-swatch::before {
    content: '';
    position: absolute;
    inset: 6px 2px;
    border-radius: 50%;
    background: var(--swatch);
    z-index: -1;
  }
  .accent-swatch[aria-pressed='true']::after {
    content: '';
    position: absolute;
    inset: 3px -1px;
    border: 1px solid var(--swatch);
    border-radius: 50%;
  }
  @media (pointer: coarse) {
    .accent-swatch {
      min-width: 36px;
      min-height: 44px;
    }
    .accent-options {
      max-width: none;
    }
  }
</style>
