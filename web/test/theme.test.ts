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

void test('supported themes survive parsing and unknown values use Mocha', () => {
  for (const theme of themes)
    assert.equal(parsePreferences({ theme: theme.id }).theme, theme.id);
  for (const value of [null, '', 'dark', {}, 1, '<script>'])
    assert.equal(parseTheme(value), 'catppuccin-mocha');
});

void test('early theme restoration works without app hydration and tolerates blocked storage', () => {
  const html = readFileSync(
    new URL('../src/app.html', import.meta.url),
    'utf8',
  );
  const script = /<script>([\s\S]*?)<\/script>/.exec(html)?.[1];
  assert.ok(script !== undefined);
  for (const theme of themes) {
    let dark = true;
    const root = {
      dataset: { theme: '', accent: '' },
      classList: {
        toggle: (_: string, value: boolean) => {
          dark = value;
        },
      },
    };
    runInNewContext(script, {
      localStorage: {
        getItem: () => JSON.stringify({ theme: theme.id, accent: 'teal' }),
      },
      document: { documentElement: root },
    });
    assert.equal(root.dataset.theme, theme.id);
    assert.equal(root.dataset.accent, 'teal');
    assert.equal(dark, theme.dark);
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

void test('legacy Mori preferences migrate and accents remain independent of surface themes', () => {
  assert.equal(parsePreferences({ theme: 'mori' }).theme, 'catppuccin-mocha');
  for (const accent of accents)
    for (const theme of themes) {
      const value = parsePreferences({ theme: theme.id, accent });
      assert.equal(value.accent, accent);
      assert.equal(value.theme, theme.id);
    }
  for (const invalid of [null, 'custom', {}, 17])
    assert.equal(parseAccent(invalid), 'mauve');
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

void test('every theme-native accent retains readable primary-button text', () => {
  const css = readFileSync(
    new URL('../src/lib/styles/themes.css', import.meta.url),
    'utf8',
  );
  let checked = 0;
  for (const block of css.split('}')) {
    const colors = new Map(
      [...block.matchAll(/--(accent-[\w-]+|on-[\w-]+): (#[\da-f]+);/g)].map(
        (match) => [match[1], match[2]],
      ),
    );
    for (const accent of accents) {
      const background = colors.get('accent-' + accent);
      if (background === undefined) continue;
      const foreground = colors.get('on-' + accent);
      assert.ok(foreground !== undefined);
      const a = luminance(background),
        b = luminance(foreground);
      assert.ok(
        (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05) >= 4.5,
        accent + ' contrast',
      );
      checked++;
    }
  }
  assert.equal(checked, themes.length * accents.length);
});
