<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: AGPL-3.0-or-later -->

<script lang="ts">
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { Check, RefreshCw } from '@lucide/svelte';
  import AccessGate from '#lib/components/AccessGate.svelte';
  import Modal from '#lib/components/Modal.svelte';
  import Button from '#lib/components/ui/button/button.svelte';
  import Input from '#lib/components/ui/input/input.svelte';
  import NativeSelect from '#lib/components/ui/native-select/native-select.svelte';
  import {
    api,
    apiData,
    message,
    session,
    type AlbumSummary,
  } from '#lib/api/client';
  import type { components } from '../../../lib/api/generated/schema';
  import { optimizationModes } from '#lib/features/optimization/model';
  import { pageTitle } from '#lib/features/page-title';
  import { preferences } from '#lib/features/preferences';

  /** Redacted account and queue status. */
  type Status = components['schemas']['SyncStatus'];
  /** Editable automatic sync settings. */
  type Schedule = components['schemas']['ScheduleInput'];
  /** Weekday names accepted by the scheduler. */
  type Weekday = components['schemas']['Weekday'];
  /** Local calendar weekdays in display order. */
  const weekdays: Weekday[] = [
    'monday',
    'tuesday',
    'wednesday',
    'thursday',
    'friday',
    'saturday',
    'sunday',
  ];
  /** One source path returned by the connector. */
  type Source = components['schemas']['SourceAlbum'];
  /** One request in the Apple verification flow. */
  type Connect = components['schemas']['ConnectInput'];
  /** The worker's current verification question. */
  type Challenge = Extract<
    components['schemas']['ConnectResult'],
    { state: 'challenge' }
  >;

  let current = $state<Status>();
  let panel = $state<'connect' | 'albums' | 'settings'>();
  let draft = $state<Omit<Schedule, 'timezone'> & { weekday: Weekday }>();
  let deviceTimeZone = $state(Intl.DateTimeFormat().resolvedOptions().timeZone);
  let account = $state('');
  let password = $state('');
  let accountRegion = $state<components['schemas']['AccountRegion'] | ''>('');
  let challenge = $state<Challenge>();
  let answer = $state('');
  let sources = $state<Source[]>();
  let destinations = $state<AlbumSummary[]>([]);
  let selected = $state<Record<string, string>>({});
  let busy = $state(false);
  let error = $state('');
  let notice = $state('');
  let confirmDisconnect = $state(false);
  const selection = $derived(
    Object.entries(selected).map(([source, album_id]) => ({
      source,
      album_id,
    })),
  );
  const validSelection = $derived(
    selection.every((rule) => rule.album_id !== ''),
  );
  const visibleSources = $derived(
    sources === undefined
      ? []
      : [
          ...sources,
          ...selection
            .filter(
              (rule) =>
                sources?.some((source) => source.path === rule.source) ===
                false,
            )
            .map((rule) => ({ name: rule.source, path: rule.source })),
        ],
  );
  const inherited = $derived(
    optimizationModes.find(
      (option) => option.value === current?.upload_optimization,
    )?.label,
  );

  /** Calendar rules are anchored to a local time and zone. */
  function isCalendar(frequency: Schedule['frequency']): boolean {
    return frequency === 'daily' || frequency === 'weekly';
  }

  /** Refresh server state while preserving controls the administrator is editing. */
  async function refresh(): Promise<void> {
    const status = await apiData(api.GET('/v1/admin/icloud'));
    current = status;
    panel ??= status.account === null ? 'connect' : 'settings';
    if (draft === undefined) {
      const { automatic, frequency, weekday, local_time, optimization } =
        status.schedule;
      draft = {
        automatic,
        frequency,
        weekday: weekday ?? 'sunday',
        local_time,
        optimization,
      };
    }
  }

  onMount(() => {
    const unsubscribe = session.subscribe(({ authenticated }) => {
      if (authenticated)
        void refresh().catch((failure: unknown) => (error = message(failure)));
      else {
        current = undefined;
        panel = undefined;
        draft = undefined;
        password = '';
        challenge = undefined;
      }
    });
    const timer = setInterval(() => {
      if ($session.authenticated && !busy && panel === 'settings')
        void refresh().catch(() => undefined);
    }, 5_000);
    return () => {
      unsubscribe();
      clearInterval(timer);
    };
  });

  /** Keep user-visible feedback local to the action that requested it. */
  async function act(action: () => Promise<void>): Promise<void> {
    if (busy) return;
    busy = true;
    error = '';
    notice = '';
    try {
      await action();
    } catch (failure) {
      error = message(failure);
    } finally {
      busy = false;
    }
  }

  /** Begin provider verification, retaining the password only until the request settles. */
  async function connect(input: Connect): Promise<void> {
    await act(async () => {
      try {
        const result = await apiData(
          api.POST('/v1/admin/icloud/connect', { body: input }),
        );
        answer = '';
        if (result.state === 'challenge') {
          challenge = result;
          if (result.retry)
            error = 'Apple did not accept that code. Try again.';
          return;
        }
        challenge = undefined;
        await refresh();
        if (result.state === 'connected' && current?.albums.length === 0) {
          panel = 'albums';
          selected = {};
          sources = undefined;
          await listAlbums();
        } else if (result.state === 'connected') panel = 'settings';
      } finally {
        password = '';
      }
    });
  }

  /** Read source paths and existing destination albums without reserving any uploads. */
  async function listAlbums(): Promise<void> {
    try {
      const [catalog, albums] = await Promise.all([
        apiData(api.GET('/v1/admin/icloud/albums')),
        apiData(api.GET('/v1/admin/albums')),
      ]);
      sources = catalog;
      destinations = albums;
    } catch (failure) {
      await refresh().catch(() => undefined);
      if (current?.needs_auth === true) {
        panel = 'settings';
        return;
      }
      throw failure;
    }
  }

  /** Edit the currently persisted mappings on a separate page state. */
  function manage(): void {
    selected = Object.fromEntries(
      (current?.albums ?? []).map((rule) => [rule.source, rule.album_id]),
    );
    sources = undefined;
    panel = 'albums';
    void act(listAlbums);
  }

  /** Retain mappings for temporarily missing iCloud albums until explicitly deselected. */
  function toggle(path: string, checked: boolean): void {
    if (checked) selected = { ...selected, [path]: '' };
    else
      selected = Object.fromEntries(
        Object.entries(selected).filter(([source]) => source !== path),
      );
  }

  /** Select a date in the browser's locale for human-readable run feedback. */
  function date(value: number | null | undefined): string {
    if (value == null) return '—';
    return new Intl.DateTimeFormat(undefined, {
      dateStyle: 'medium',
      timeStyle: 'short',
    }).format(new Date(value * 1000));
  }

  /** Display an automatic calendar run in the rule's saved zone. */
  function nextDate(value: number, rule: Schedule): string {
    if (!isCalendar(rule.frequency)) return date(value);
    try {
      const formatter = new Intl.DateTimeFormat(undefined, {
        weekday: 'short',
        month: 'short',
        day: 'numeric',
        hour: 'numeric',
        minute: '2-digit',
        timeZone: rule.timezone,
      });
      return `${formatter.format(new Date(value * 1000))} (${rule.timezone})`;
    } catch {
      return date(value);
    }
  }
</script>

<svelte:head><title>{pageTitle('iCloud Photos')}</title></svelte:head>
<main>
  <div class="page-heading">
    <h1>iCloud Photos</h1>
    {#if current?.account != null && !current.needs_auth && panel === 'settings'}
      <Button
        class="button primary"
        disabled={busy ||
          current.activity === 'running' ||
          current.albums.length === 0}
        onclick={() =>
          act(async () => {
            await apiData(api.POST('/v1/admin/icloud/sync'));
            await refresh();
            notice = 'Sync queued';
          })}
      >
        <RefreshCw size={16} />
        {current.activity === 'running' ? 'Syncing…' : 'Sync now'}
      </Button>
    {/if}
  </div>
  <AccessGate>
    {#if error}<p class="feedback error" role="alert">{error}</p>{/if}
    {#if notice}<p class="feedback muted" role="status">{notice}</p>{/if}
    {#if current === undefined}
      {#if error}<Button
          variant="outline"
          class="button"
          onclick={() => act(refresh)}>Try again</Button
        >
      {:else}<p class="muted" role="status">Loading iCloud settings…</p>{/if}
    {:else if !current.enabled}
      <section class="settings-group">
        <header class="settings-group-heading">
          <h2>iCloud sync is unavailable</h2>
        </header>
        <div class="setting-row">
          <p class="muted">
            Start Mori with the iCloud worker to connect an Apple Account.
          </p>
        </div>
      </section>
    {:else if panel === 'connect'}
      {#if challenge}
        <form
          class="settings-group"
          onsubmit={(event) => {
            event.preventDefault();
            if (challenge)
              void connect({
                action: 'answer',
                challenge: challenge.id,
                answer,
              });
          }}
        >
          <header class="settings-group-heading">
            <h2>Verify your account</h2>
            <p>
              {challenge.kind === 'phone'
                ? 'Choose a trusted phone number.'
                : challenge.kind === 'sms'
                  ? 'Enter the code sent to your phone.'
                  : 'Enter the code from your trusted Apple device.'}
            </p>
          </header>
          <div class="setting-row">
            <div class="setting-content">
              <span class="setting-label">Apple Account</span><span class="wrap"
                >{account}</span
              >
            </div>
          </div>
          <div class="setting-row">
            <div class="setting-content">
              <label for="icloud-verification"
                >{challenge.kind === 'phone'
                  ? 'Phone number'
                  : 'Verification code'}</label
              >
              <div class="inline-control">
                {#if challenge.kind === 'phone'}
                  <NativeSelect
                    id="icloud-verification"
                    bind:value={answer}
                    required
                    disabled={busy}
                  >
                    <option value="">Select a phone</option>
                    {#each challenge.phones as phone (phone.value)}<option
                        value={phone.value}>{phone.label}</option
                      >{/each}
                  </NativeSelect>
                {:else}
                  <Input
                    id="icloud-verification"
                    placeholder="6-digit code"
                    inputmode="numeric"
                    autocomplete="one-time-code"
                    maxlength={6}
                    pattern={'[0-9]{6}'}
                    bind:value={answer}
                    required
                    disabled={busy}
                  />
                  {#if challenge.kind === 'code'}
                    <Button
                      variant="ghost"
                      class="button"
                      type="button"
                      disabled={busy}
                      onclick={() => {
                        if (challenge)
                          void connect({
                            action: 'answer',
                            challenge: challenge.id,
                            answer: 'sms',
                          });
                      }}>Use SMS instead</Button
                    >
                  {/if}
                {/if}
              </div>
            </div>
          </div>
          <div class="setting-row">
            <div class="setting-content">
              <span></span>
              <div class="inline-control">
                <Button
                  variant="outline"
                  class="button"
                  type="button"
                  disabled={busy}
                  onclick={() => {
                    if (challenge)
                      void connect({
                        action: 'cancel',
                        challenge: challenge.id,
                      });
                  }}>Back</Button
                >
                <Button class="button primary" type="submit" disabled={busy}
                  >{challenge.kind === 'phone' ? 'Send code' : 'Verify'}</Button
                >
              </div>
            </div>
          </div>
        </form>
      {:else}
        <form
          class="settings-group"
          onsubmit={(event) => {
            event.preventDefault();
            void connect({
              action: 'start',
              account,
              password,
              region: accountRegion === '' ? null : accountRegion,
              upload_optimization: $preferences.uploadOptimization,
            });
          }}
        >
          <header class="settings-group-heading">
            <h2>Connect iCloud Photos</h2>
            <p>Connect your Apple Account to choose albums.</p>
          </header>
          <div class="setting-row">
            <div class="setting-content">
              <label for="icloud-account">Apple Account</label>
              <Input
                id="icloud-account"
                type="email"
                placeholder="name@example.com"
                autocomplete="off"
                maxlength={254}
                required
                bind:value={account}
                disabled={busy}
              />
            </div>
          </div>
          <div class="setting-row">
            <div class="setting-content">
              <label for="icloud-password">Password</label>
              <Input
                id="icloud-password"
                type="password"
                autocomplete="off"
                maxlength={1024}
                required
                bind:value={password}
                disabled={busy}
              />
            </div>
          </div>
          <div class="setting-row">
            <div class="setting-content">
              <label for="icloud-account-region">iCloud region</label>
              <div class="setting-control">
                <NativeSelect
                  id="icloud-account-region"
                  bind:value={accountRegion}
                  disabled={busy}
                >
                  <option value="">Automatic</option>
                  <option value="china">China mainland (GCBD)</option>
                </NativeSelect>
              </div>
            </div>
          </div>
          <div class="setting-row">
            <div class="setting-content">
              <span></span>
              <div class="inline-control">
                {#if current.account}<Button
                    variant="outline"
                    class="button"
                    type="button"
                    disabled={busy}
                    onclick={() => (panel = 'settings')}>Back</Button
                  >{/if}
                <Button class="button primary" type="submit" disabled={busy}
                  >{busy ? 'Connecting…' : 'Continue'}</Button
                >
              </div>
            </div>
          </div>
        </form>
      {/if}
    {:else if panel === 'albums'}
      <form
        class="settings-group"
        onsubmit={(event) => {
          event.preventDefault();
          if (!validSelection) return;
          void act(async () => {
            await apiData(
              api.PUT('/v1/admin/icloud/albums', {
                body: { albums: selection },
              }),
            );
            panel = 'settings';
            await refresh();
            notice = selection.length
              ? 'Import queued'
              : 'Album selection saved';
          });
        }}
      >
        <header class="settings-group-heading">
          <h2>Import albums</h2>
          <p>Choose iCloud albums and where they go in Mori.</p>
        </header>
        {#if sources === undefined}
          <div class="setting-row">
            {#if busy}<p class="muted" role="status">Finding iCloud albums…</p>
            {:else}<Button
                variant="outline"
                class="button"
                onclick={() => act(listAlbums)}>Try again</Button
              >{/if}
          </div>
        {:else if sources.length === 0}
          <div class="setting-row">
            <p class="muted">No iCloud albums found.</p>
          </div>
        {:else if destinations.length === 0}
          <div class="setting-row inline-control">
            <Button variant="outline" class="button" href={resolve('/albums')}
              >Create a Mori album</Button
            >
          </div>
        {:else}
          <div class="album-row album-head">
            <span>iCloud album</span><span>Mori album</span>
          </div>
          {#each visibleSources as source (source.path)}
            {@const checked = Object.hasOwn(selected, source.path)}
            <div class="album-row">
              <label class="album-source"
                ><input
                  type="checkbox"
                  {checked}
                  disabled={busy}
                  onchange={(event) => {
                    toggle(source.path, event.currentTarget.checked);
                  }}
                /><span class="wrap">{source.name}</span></label
              >
              <NativeSelect
                aria-label={'Mori album for ' + source.name}
                value={selected[source.path] ?? ''}
                disabled={busy || !checked}
                required={checked}
                onchange={(
                  event: Event & { currentTarget: HTMLSelectElement },
                ) => {
                  selected = {
                    ...selected,
                    [source.path]: event.currentTarget.value,
                  };
                }}
              >
                <option value="">Choose a Mori album</option>
                {#each destinations as album (album.id)}<option value={album.id}
                    >{album.name}</option
                  >{/each}
              </NativeSelect>
            </div>
          {/each}
        {/if}
        <div class="setting-row album-footer">
          <span class="muted"
            >{selection.length}
            {selection.length === 1 ? 'album' : 'albums'} selected</span
          >
          <div class="inline-control">
            <Button
              variant="outline"
              class="button"
              type="button"
              disabled={busy}
              onclick={() => (panel = 'settings')}>Back</Button
            >
            <Button
              class="button primary"
              type="submit"
              disabled={busy ||
                sources === undefined ||
                !validSelection ||
                (!selection.length && !current.albums.length)}
            >
              {selection.length ? 'Import selected albums' : 'Save selection'}
            </Button>
          </div>
        </div>
      </form>
    {:else if draft}
      <div class="settings-groups">
        <section class="settings-group">
          <header class="settings-group-heading"><h2>Account</h2></header>
          <div class="setting-row account-row">
            <div class="setting-content">
              <span class="setting-label">Apple Account</span><span class="wrap"
                >{current.account}</span
              >
            </div>
            <div class="inline-control">
              {#if current.needs_auth}
                <Button
                  class="button primary"
                  disabled={busy}
                  onclick={() => {
                    account = current?.account ?? '';
                    panel = 'connect';
                  }}>Reconnect</Button
                >
              {:else}<span class="connected"><Check size={16} />Connected</span
                >{/if}
              <Button
                variant="outline"
                class="button"
                disabled={busy || current.activity === 'running'}
                onclick={() => (confirmDisconnect = true)}>Disconnect</Button
              >
            </div>
          </div>
        </section>
        <form
          class="settings-group"
          onsubmit={(event) => {
            event.preventDefault();
            if (!draft) return;
            deviceTimeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
            const body = { ...draft, timezone: deviceTimeZone };
            void act(async () => {
              await apiData(api.PUT('/v1/admin/icloud/schedule', { body }));
              await refresh();
              notice = 'Schedule saved';
            });
          }}
        >
          <header class="settings-group-heading"><h2>Sync schedule</h2></header>
          <div class="setting-row">
            <div class="setting-content">
              <label for="icloud-automatic">Automatic sync</label>
              <div class="switch">
                <input
                  id="icloud-automatic"
                  type="checkbox"
                  role="switch"
                  bind:checked={draft.automatic}
                  disabled={busy}
                /><span aria-hidden="true"></span>
              </div>
            </div>
          </div>
          {#if draft.automatic}
            <div class="setting-row">
              <div class="setting-content">
                <label for="icloud-frequency">Frequency</label>
                <div class="setting-control">
                  <NativeSelect
                    id="icloud-frequency"
                    bind:value={draft.frequency}
                    disabled={busy}
                  >
                    <option value="weekly">Weekly</option>
                    <option value="daily">Daily</option>
                    <option value="six_hours">Every 6 hours</option>
                    <option value="hourly">Hourly</option>
                  </NativeSelect>
                </div>
              </div>
            </div>
            {#if draft.frequency === 'weekly'}
              <div class="setting-row">
                <div class="setting-content">
                  <label for="icloud-weekday">Day of the week</label>
                  <div class="setting-control">
                    <NativeSelect
                      id="icloud-weekday"
                      bind:value={draft.weekday}
                      disabled={busy}
                    >
                      {#each weekdays as day (day)}
                        <option value={day}
                          >{day.charAt(0).toUpperCase() + day.slice(1)}</option
                        >
                      {/each}
                    </NativeSelect>
                  </div>
                </div>
              </div>
            {/if}
            {#if isCalendar(draft.frequency)}
              <div class="setting-row">
                <div class="setting-content">
                  <label for="icloud-time">Time</label>
                  <Input
                    id="icloud-time"
                    type="time"
                    bind:value={draft.local_time}
                    required
                    disabled={busy}
                  />
                </div>
              </div>
              <div class="setting-row">
                <div class="setting-content">
                  <span class="setting-label">Device time zone</span>
                  <span>{deviceTimeZone}</span>
                </div>
              </div>
            {/if}
          {/if}
          <div class="setting-row">
            <div class="setting-content">
              <label for="icloud-optimization">Upload optimization</label>
              <div class="setting-control inline-control">
                <NativeSelect
                  id="icloud-optimization"
                  value={draft.optimization ?? ''}
                  disabled={busy}
                  onchange={(
                    event: Event & { currentTarget: HTMLSelectElement },
                  ) => {
                    if (!draft) return;
                    const preset = optimizationModes.find(
                      (item) => item.value === event.currentTarget.value,
                    );
                    draft.optimization = preset?.value ?? null;
                  }}
                >
                  <option value="">Follow upload preference</option>
                  {#each optimizationModes as mode (mode.value)}<option
                      value={mode.value}>{mode.label}</option
                    >{/each}
                </NativeSelect>
                {#if draft.optimization === null}<span class="muted"
                    >{inherited}</span
                  >{/if}
              </div>
            </div>
          </div>
          <div class="setting-row">
            <div class="setting-content">
              <span></span>
              <div>
                <Button class="button primary" type="submit" disabled={busy}
                  >Save schedule</Button
                >
              </div>
            </div>
          </div>
          {#if current.next_sync !== null && !current.needs_auth}
            <div class="setting-row">
              <div class="setting-content">
                <span class="setting-label">Next sync</span>
                <span>{nextDate(current.next_sync, current.schedule)}</span>
              </div>
            </div>
          {/if}
        </form>
        <section class="settings-group">
          <header class="settings-group-heading"><h2>Albums</h2></header>
          <div class="setting-row">
            <div class="setting-content">
              <span class="setting-label"
                >{current.albums.length}
                {current.albums.length === 1 ? 'album' : 'albums'} selected</span
              >
              <div>
                <Button
                  variant="outline"
                  class="button"
                  onclick={manage}
                  disabled={busy ||
                    current.activity === 'running' ||
                    current.needs_auth}>Manage albums</Button
                >
              </div>
            </div>
          </div>
          <div class="setting-row">
            <div class="setting-content">
              <span class="setting-label">Last sync</span><span
                >{date(current.last_sync)}</span
              >
            </div>
          </div>
          {#if current.last_error}<p class="sync-message error" role="alert">
              {current.last_error}
            </p>
          {:else if current.activity === 'queued'}<p
              class="sync-message muted"
              role="status"
            >
              Sync queued
            </p>{/if}
        </section>
      </div>
    {/if}
  </AccessGate>
</main>

<Modal
  open={confirmDisconnect}
  title="Disconnect iCloud Photos"
  onclose={() => (confirmDisconnect = false)}
>
  <p>
    Disconnecting stops sync and removes the album selection. Imported media
    stays in Mori.
  </p>
  {#snippet footer()}
    <Button
      variant="outline"
      class="button"
      onclick={() => (confirmDisconnect = false)}>Cancel</Button
    >
    <Button
      class="button primary"
      disabled={busy}
      onclick={() =>
        act(async () => {
          await apiData(api.DELETE('/v1/admin/icloud/connect'));
          confirmDisconnect = false;
          panel = 'connect';
          draft = undefined;
          await refresh();
        })}>Disconnect</Button
    >
  {/snippet}
</Modal>

<style>
  .settings-groups {
    display: grid;
    gap: 24px;
  }
  .inline-control {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .inline-control :global(input) {
    flex: 1;
    min-width: 8rem;
  }
  .feedback {
    margin-bottom: 16px;
  }
  .account-row,
  .album-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    flex-wrap: wrap;
  }
  .connected {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .connected :global(svg) {
    color: var(--success);
  }
  .wrap {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .album-row {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    align-items: center;
    gap: 24px;
    padding: 12px 24px;
    border-bottom: 1px solid var(--subtle-border);
  }
  .album-head {
    margin-top: 16px;
    font-weight: 500;
  }
  .album-source {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .album-source input {
    flex-shrink: 0;
    accent-color: var(--selection);
    width: 18px;
    height: 18px;
  }
  .sync-message {
    padding: 0 24px 20px;
  }
  .switch {
    position: relative;
    display: grid;
    align-items: center;
    width: 44px;
    height: 44px;
  }
  .switch input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }
  .switch span {
    width: 42px;
    height: 24px;
    padding: 3px;
    border-radius: 14px;
    background: var(--input);
    pointer-events: none;
  }
  .switch span::after {
    content: '';
    display: block;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--foreground);
  }
  .switch input:checked + span {
    background: var(--selection);
  }
  .switch input:checked + span::after {
    transform: translateX(18px);
    background: var(--selection-foreground);
  }
  .switch input:focus-visible + span {
    outline: 2px solid var(--ring);
    outline-offset: 4px;
  }
  @media (max-width: 600px) {
    .album-row {
      gap: 12px;
      padding: 12px 20px;
    }
    .album-source {
      gap: 8px;
    }
  }
</style>
