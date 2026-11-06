<!-- SPDX-FileCopyrightText: 2026 Cubeyond
     SPDX-License-Identifier: AGPL-3.0-or-later -->

<script lang="ts">
  /** Shared text wordmark keeps header and mobile navigation sizing and alignment consistent. */
  import { resolve } from '$app/paths';
</script>

<a class="wordmark" href={resolve('/')} aria-label="Mori home">mori</a>

<style>
  .wordmark {
    display: inline-flex;
    align-items: center;
    height: 44px;
    font-family: 'Mori Serif', Georgia, serif;
    font-size: 44px;
    line-height: 1;
    letter-spacing: -0.045em;
    font-weight: 400;
    color: var(--wordmark);
    /* Center the actual serif glyphs optically within the 56px header. */
    transform: translateY(-1px);
  }
</style>
