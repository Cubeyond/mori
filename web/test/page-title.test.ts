// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Tab labels share one current-page and product-name convention. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { pageTitle } from '../src/lib/features/page-title.ts';

void test('homepage and unavailable titles show only the brand', () => {
  for (const name of [undefined, null, '', '   '])
    assert.equal(pageTitle(name), 'Mori');
});

void test('page and album titles use the dashboard separator and a single brand suffix', () => {
  for (const name of ['Videos', 'Albums', 'Preferences', 'Storage', 'Nature']) {
    assert.equal(pageTitle(` ${name} `), `${name} – Mori`);
  }
});
