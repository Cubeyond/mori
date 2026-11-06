<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: Elastic-2.0 -->

<script lang="ts">
  /** Shared album fields keep ordinary and batch creation aligned and consistently validated. */
  import Input from '#lib/components/ui/input/input.svelte';
  import NativeSelect from '#lib/components/ui/native-select/native-select.svelte';
  import Textarea from '#lib/components/ui/textarea/textarea.svelte';
  /** Parent-owned values preserve drafts when switching between existing and new albums. */
  let {
    name = $bindable(''),
    slug = $bindable(''),
    visibility = $bindable('private'),
    description = $bindable(''),
    prefix = 'album',
    disabled = false,
    suggestSlug = false,
  }: {
    name?: string;
    slug?: string;
    visibility?: string;
    description?: string;
    prefix?: string;
    disabled?: boolean;
    suggestSlug?: boolean;
  } = $props();
  /** A suggestion is used only during explicit creation; edited URL slugs remain independent. */
  let suggestion = '';
  /** Generate a conservative ASCII suggestion without overriding a manually edited address. */
  function suggest(value: string): void {
    name = value;
    if (!suggestSlug) return;
    const next = value
      .normalize('NFKD')
      .replace(/\p{Mark}/gu, '')
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '')
      .slice(0, 64)
      .replace(/-$/, '');
    if (slug === '' || slug === suggestion) slug = next;
    suggestion = next;
  }
</script>

<label class="field" for={prefix + '-name'}
  >Display name<Input
    id={prefix + '-name'}
    required
    maxlength={200}
    value={name}
    oninput={(event: Event & { currentTarget: HTMLInputElement }) => {
      suggest(event.currentTarget.value);
    }}
    autocomplete="off"
    {disabled}
  /></label
>
<label class="field" for={prefix + '-description'}
  >Description <span class="muted">(optional)</span><Textarea
    id={prefix + '-description'}
    maxlength={500}
    rows={2}
    bind:value={description}
    {disabled}
  /></label
>
<label class="field" for={prefix + '-slug'}
  >URL slug<Input
    id={prefix + '-slug'}
    required
    minlength={3}
    maxlength={64}
    pattern="[a-z0-9]+(-[a-z0-9]+)*"
    placeholder="my-album"
    bind:value={slug}
    autocomplete="off"
    {disabled}
  /></label
>
<label class="field" for={prefix + '-visibility'}
  >Visibility<NativeSelect
    id={prefix + '-visibility'}
    bind:value={visibility}
    {disabled}
    ><option value="private">Private</option><option value="public"
      >Public</option
    ></NativeSelect
  ></label
>
