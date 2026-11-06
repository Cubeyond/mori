// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Match the dashboard convention: the current page followed by the product name. */
export function pageTitle(name?: string | null): string {
  const label = name?.trim() ?? '';
  return label.length > 0 ? `${label} – Mori` : 'Mori';
}
