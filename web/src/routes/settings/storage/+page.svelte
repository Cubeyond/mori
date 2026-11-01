<script lang="ts">
  import { pageTitle } from '#lib/features/page-title';
  /** Administrator-only S3 setup keeps write-only credentials out of browser persistence. */
  import { onMount } from 'svelte';
  import { CircleCheck, LoaderCircle, LockKeyhole } from '@lucide/svelte';
  import { resolve } from '$app/paths';
  import AccessGate from '#lib/components/AccessGate.svelte';
  import Breadcrumbs from '#lib/components/Breadcrumbs.svelte';
  import Button from '#lib/components/ui/button/button.svelte';
  import Input from '#lib/components/ui/input/input.svelte';
  import {
    message,
    api,
    apiData,
    session,
    type StorageInfo,
    type StorageInput,
  } from '#lib/api/client';
  /** The saved summary remains visible while a connection test or save is running. */
  let saved = $state<StorageInfo | null>(null);
  /** Connection identity is editable only when existing media does not lock it. */
  let bucket = $state(''),
    region = $state('us-east-1'),
    distribution = $state('');
  /** Blank credential inputs retain saved values; removal of a saved token is explicit. */
  let accessKey = $state(''),
    secretKey = $state(''),
    token = $state(''),
    removeToken = $state(false);
  /** Progress and feedback stay local to this form and do not create floating notifications. */
  let loaded = $state(false),
    error = $state(''),
    feedback = $state('');
  /** One request owns form changes at a time. */
  let pending = $state<'save' | 'test' | null>(null);
  /** Preserve location safety after credentials are changed or the page is reloaded. */
  const locked = $derived(saved?.location_locked === true);
  /** Render an initial setup action independently of the form's readiness. */
  const configured = $derived(saved?.configured === true);
  /** Protocol fixtures are local preview infrastructure, never a configured Amazon S3 account. */
  const preview = $derived(saved?.mode === 'test');
  onMount(() => {
    if ($session.authenticated) void load();
  });
  /** Apply safe returned metadata and promptly discard submitted credential fields. */
  function apply(info: StorageInfo): void {
    saved = info;
    bucket = info.bucket;
    region = info.region;
    distribution = info.distribution_id ?? '';
    accessKey = secretKey = token = '';
    removeToken = false;
  }
  /** Load settings only after the server confirms the administrator session. */
  async function load(): Promise<void> {
    error = '';
    try {
      apply(await apiData(api.GET('/v1/admin/storage')));
      loaded = true;
    } catch (failure) {
      error = message(failure);
    }
  }
  /** Omitted credentials retain saved values, while an explicit empty token removes it. */
  function payload(): StorageInput {
    return {
      bucket: bucket.trim(),
      region: region.trim(),
      distribution_id: distribution.trim() || null,
      access_key_id: accessKey || null,
      secret_access_key: secretKey || null,
      session_token: token || (removeToken ? '' : null),
    };
  }
  /** Test the proposed connection or verify and activate it through the same server boundary. */
  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (pending !== null) return;
    const action =
      event.submitter instanceof HTMLButtonElement &&
      event.submitter.value === 'test'
        ? 'test'
        : 'save';
    pending = action;
    error = feedback = '';
    try {
      if (action === 'save') {
        apply(await apiData(api.PUT('/v1/admin/storage', { body: payload() })));
        feedback = preview ? 'Preview settings saved' : 'Storage saved';
      } else {
        const result = await apiData(
          api.POST('/v1/admin/storage/test', { body: payload() }),
        );
        feedback =
          result.mode === 'test'
            ? 'Preview storage verified'
            : 'S3 connection verified';
      }
    } catch (failure) {
      error = message(failure);
    } finally {
      pending = null;
    }
  }
</script>

<svelte:head><title>{pageTitle('Storage')}</title></svelte:head>
<main>
  <Breadcrumbs
    items={[
      { label: 'Settings', href: resolve('/settings') },
      { label: 'Storage', href: resolve('/settings/storage') },
    ]}
  />
  <div class="page-heading">
    <h1>Storage</h1>
  </div>
  {#if loaded}
    {#if preview}
      <p class="storage-description">
        Amazon S3 is not configured. This preview uses local test storage.
      </p>
    {:else}
      <p class="storage-description">
        {configured ? 'Configured' : 'Not configured'}
      </p>
    {/if}
  {/if}
  <AccessGate>
    {#if !loaded}
      {#if error}<p class="error" role="alert">{error}</p>
        <Button variant="outline" class="button retry" onclick={load}
          >Try again</Button
        >{:else}<p class="muted" role="status">
          Loading storage settings…
        </p>{/if}
    {:else}
      <form onsubmit={submit}>
        <fieldset disabled={pending !== null}>
          <section class="settings-group" aria-labelledby="connection-heading">
            <div class="settings-group-heading">
              <h2 id="connection-heading">Amazon S3</h2>
              {#if locked}<p>
                  <LockKeyhole size={16} />Storage location is locked while your
                  library contains videos.
                </p>{/if}
            </div>
            <div class="settings-fields">
              <div class="setting-row">
                <div class="setting-content">
                  <label for="storage-bucket">Bucket name</label>
                  <div class="setting-control">
                    <Input
                      id="storage-bucket"
                      bind:value={bucket}
                      placeholder="my-video-library"
                      required
                      maxlength={63}
                      disabled={locked}
                      spellcheck={false}
                      autocapitalize="none"
                      autocomplete="off"
                    />
                  </div>
                </div>
              </div>
              <div class="setting-row">
                <div class="setting-content">
                  <label for="storage-region">AWS region</label>
                  <div class="setting-control">
                    <Input
                      id="storage-region"
                      bind:value={region}
                      placeholder="us-east-1"
                      required
                      maxlength={64}
                      disabled={locked}
                      spellcheck={false}
                      autocapitalize="none"
                      autocomplete="off"
                    />
                  </div>
                </div>
              </div>
            </div>
          </section>
          <section class="settings-group" aria-labelledby="credentials-heading">
            <div class="settings-group-heading">
              <h2 id="credentials-heading">Credentials</h2>
              {#if configured && !preview}<p>
                  Leave blank to keep the saved credentials.
                </p>{/if}
            </div>
            <div class="settings-fields">
              <div class="setting-row">
                <div class="setting-content">
                  <label for="storage-access-key">Access key ID</label>
                  <div class="setting-control">
                    <Input
                      id="storage-access-key"
                      bind:value={accessKey}
                      placeholder="Enter access key ID"
                      required={!configured}
                      maxlength={256}
                      spellcheck={false}
                      autocapitalize="none"
                      autocomplete="off"
                    />
                  </div>
                </div>
              </div>
              <div class="setting-row">
                <div class="setting-content">
                  <label for="storage-secret-key">Secret access key</label>
                  <div class="setting-control">
                    <Input
                      id="storage-secret-key"
                      type="password"
                      bind:value={secretKey}
                      placeholder="Enter secret access key"
                      required={!configured}
                      maxlength={4096}
                      autocomplete="new-password"
                    />
                  </div>
                </div>
              </div>
              <div class="setting-row">
                <div class="setting-content">
                  <label for="storage-session-token"
                    >Session token <span class="optional">Optional</span></label
                  >
                  <div class="setting-control">
                    <Input
                      id="storage-session-token"
                      type="password"
                      bind:value={token}
                      placeholder="Enter session token"
                      maxlength={16384}
                      autocomplete="new-password"
                      disabled={removeToken}
                    />{#if saved?.has_session_token === true}<label
                        class="token-removal"
                        ><input
                          type="checkbox"
                          bind:checked={removeToken}
                        />Remove saved session token</label
                      >{/if}
                  </div>
                </div>
              </div>
            </div>
          </section>
          <section class="settings-group" aria-labelledby="delivery-heading">
            <div class="settings-group-heading">
              <h2 id="delivery-heading">
                CloudFront <span class="optional">Optional</span>
              </h2>
            </div>
            <div class="settings-fields">
              <div class="setting-row">
                <div class="setting-content">
                  <label for="storage-distribution">Distribution ID</label>
                  <div class="setting-control">
                    <Input
                      id="storage-distribution"
                      bind:value={distribution}
                      placeholder="E123EXAMPLE"
                      maxlength={64}
                      disabled={saved?.delivery_change ===
                        'requires_revocation'}
                      spellcheck={false}
                      autocapitalize="none"
                      autocomplete="off"
                    />
                    <p class="field-hint">
                      Leave blank to deliver directly from Amazon S3.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </fieldset>
        <div class="form-actions">
          <div class="form-feedback" aria-live="polite" aria-atomic="true">
            {#if error}<p class="error" role="alert">
                {error}
              </p>{:else if feedback}<p class="success" role="status">
                <CircleCheck size={18} />{feedback}
              </p>{/if}
          </div>
          <div class="action-buttons">
            <Button
              type="submit"
              value="test"
              variant="outline"
              class="button"
              disabled={pending !== null}
              >{#if pending === 'test'}<LoaderCircle
                  size={18}
                  class="spinning"
                />Testing…{:else}Test connection{/if}</Button
            ><Button
              type="submit"
              value="save"
              class="button primary"
              disabled={pending !== null}
              >{#if pending === 'save'}<LoaderCircle
                  size={18}
                  class="spinning"
                />Saving…{:else}Save changes{/if}</Button
            >
          </div>
        </div>
      </form>
    {/if}
  </AccessGate>
</main>

<style>
  fieldset {
    border: 0;
    padding: 0;
    margin: 0;
    min-width: 0;
    display: grid;
    gap: 20px;
  }
  .page-heading {
    margin-bottom: 12px;
  }
  .storage-description {
    font-size: 15px;
    line-height: 24px;
    color: var(--muted-foreground);
    margin-bottom: 24px;
  }
  .success {
    color: #b7ebcf;
  }
  .optional {
    display: block;
    color: #92929e;
    font-size: 14px;
    font-weight: 400;
  }
  .field-hint {
    font-size: 14px;
    color: #92929e;
    margin-top: 8px;
  }
  .token-removal {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 12px;
    font-size: 14px;
  }
  .token-removal input {
    accent-color: #e4e4eb;
    width: 16px;
    height: 16px;
    margin: 0;
  }
  .form-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    margin-top: 24px;
  }
  .action-buttons {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-left: auto;
    flex-shrink: 0;
  }
  .form-feedback {
    min-width: 0;
  }
  .success {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 15px;
  }
  :global(svg.spinning) {
    animation: spin 900ms linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  :global(.retry) {
    margin-top: 16px;
  }
  @media (max-width: 600px) {
    .page-heading {
      flex-wrap: wrap;
    }
    .form-actions {
      flex-direction: column;
      align-items: stretch;
      gap: 16px;
    }
    .action-buttons {
      margin: 0;
    }
    .action-buttons :global(button) {
      flex: 1;
    }
  }
</style>
