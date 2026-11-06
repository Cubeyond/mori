// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Preserve bookmarks while Gallery becomes the canonical mixed-media destination. */
import { redirect } from '@sveltejs/kit';
import { resolve } from '$app/paths';
import type { PageLoad } from './$types';
/** Existing upload query parameters retain their behavior after the rename. */
export const load: PageLoad = ({ url }) =>
  redirect(308, resolve('/gallery') + url.search);
