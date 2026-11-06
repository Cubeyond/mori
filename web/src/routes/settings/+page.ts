// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Preserve bookmarks while Preferences receives its own canonical settings path. */
import { redirect } from '@sveltejs/kit';
import { resolve } from '$app/paths';
import type { PageLoad } from './$types';

/** Keep query parameters and section anchors when redirecting client-side visits. */
export const load: PageLoad = ({ url }) =>
  redirect(308, resolve('/settings/preferences') + url.search + url.hash);
