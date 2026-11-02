<script lang="ts">
  import { pageTitle } from '#lib/features/page-title';
  /** Browser-local preferences acknowledge changes beside the edited control. */
  import NativeSelect from '#lib/components/ui/native-select/native-select.svelte';
  import Breadcrumbs from '#lib/components/Breadcrumbs.svelte';
  import { resolve } from '$app/paths';
  import { onDestroy } from 'svelte';
  import AccessGate from '#lib/components/AccessGate.svelte';
  import BackgroundSettings from '#lib/components/settings/BackgroundSettings.svelte';
  import ThumbnailSettings from '#lib/components/settings/ThumbnailSettings.svelte';
  import PreferenceFeedback from '#lib/components/settings/PreferenceFeedback.svelte';
  import { themes, parseTheme } from '#lib/features/theme';
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
    <div class="preferences-groups">
      <section class="settings-group" aria-labelledby="appearance-heading">
        <header class="settings-group-heading">
          <h2 id="appearance-heading">Appearance</h2>
        </header>
        <div class="setting-row">
          <div class="setting-content">
            <label for="preference-theme">Theme</label>
            <div class="setting-control preference-control">
              <NativeSelect
                id="preference-theme"
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
</style>
