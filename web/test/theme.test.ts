// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

/** Theme upgrades validate browser data and restore the color scheme before first paint. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import {
  themes,
  parseTheme,
  accents,
  parseAccent,
} from '../src/lib/features/theme.ts';
import { parsePreferences } from '../src/lib/features/preferences.ts';

void test('supported themes survive parsing and unknown values use Mori', () => {
  for (const theme of themes)
    assert.equal(parsePreferences({ theme: theme.id }).theme, theme.id);
  for (const value of [null, '', 'dark', {}, 1, '<script>'])
    assert.equal(parseTheme(value), 'mori');
});

void test('early theme restoration works without app hydration and tolerates blocked storage', () => {
  const html = readFileSync(
    new URL('../src/app.html', import.meta.url),
    'utf8',
  );
  const script = /<script>([\s\S]*?)<\/script>/.exec(html)?.[1];
  assert.ok(script !== undefined);
  const saved = [
    ...themes.map((theme) => ({ theme: theme.id, accent: 'teal' })),
    null,
    {},
    { theme: 'obsolete', accent: 'custom' },
    { theme: 'mori' },
    { theme: 'catppuccin-mocha', accent: 'mauve' },
  ];
  for (const value of saved) {
    const expected = parsePreferences(value);
    let dark = true;
    const root = {
      dataset: { theme: 'mori', accent: 'green' },
      classList: {
        toggle: (_: string, value: boolean) => {
          dark = value;
        },
      },
    };
    runInNewContext(script, {
      localStorage: {
        getItem: () => JSON.stringify(value),
      },
      document: { documentElement: root },
    });
    assert.equal(root.dataset.theme, expected.theme);
    assert.equal(root.dataset.accent, expected.accent);
    assert.equal(dark, expected.theme !== 'catppuccin-latte');
  }
  assert.doesNotThrow(() =>
    runInNewContext(script, {
      localStorage: {
        getItem: () => {
          throw new Error('Blocked');
        },
      },
    }),
  );
});

void test('Mori is restored and explicit existing palette and accent choices are retained', () => {
  assert.equal(parsePreferences({ theme: 'mori' }).theme, 'mori');
  for (const accent of accents)
    for (const theme of themes) {
      const value = parsePreferences({ theme: theme.id, accent });
      assert.equal(value.accent, accent);
      assert.equal(value.theme, theme.id);
    }
  for (const invalid of [null, 'custom', {}, 17])
    assert.equal(parseAccent(invalid), 'green');
});

/** Relative sRGB luminance makes text contrast independent of a theme's hue. */
function luminance(hex: string): number {
  const values = [1, 3, 5].map((index) => {
    const value = Number.parseInt(hex.slice(index, index + 2), 16) / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  const [red = 0, green = 0, blue = 0] = values;
  return red * 0.2126 + green * 0.7152 + blue * 0.0722;
}

void test('theme surfaces, primary actions and selected controls retain readable text', () => {
  const css = readFileSync(
    new URL('../src/lib/styles/themes.css', import.meta.url),
    'utf8',
  );
  let checked = 0;
  for (const theme of themes) {
    const block = css
      .split(":root[data-theme='" + theme.id + "'] {")[1]
      ?.split('}')[0];
    assert.ok(block !== undefined);
    const colors = new Map(
      [...block.matchAll(/--([\w-]+): (#[\da-f]+);/g)].map((match) => [
        match[1],
        match[2],
      ]),
    );
    for (const background of ['background', 'card', 'popover', 'accent'])
      for (const foreground of ['foreground', 'muted-foreground'])
        assert.ok(
          contrast(colors.get(background), colors.get(foreground)) >= 4.5,
          theme.id + ' ' + foreground + ' on ' + background,
        );
    assert.ok(
      contrast(colors.get('primary'), colors.get('primary-foreground')) >= 4.5,
      theme.id + ' primary action',
    );
    for (const accent of accents) {
      const background = colors.get('accent-' + accent);
      if (background === undefined) continue;
      const foreground = colors.get('on-' + accent);
      assert.ok(foreground !== undefined);
      assert.ok(contrast(background, foreground) >= 4.5, accent + ' contrast');
      checked++;
    }
  }
  assert.equal(checked, themes.length * accents.length);
});

/** Missing palette tokens fail the same accessibility contract as unreadable text. */
function contrast(
  background: string | undefined,
  foreground: string | undefined,
): number {
  assert.ok(background !== undefined && foreground !== undefined);
  const a = luminance(background),
    b = luminance(foreground);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}
