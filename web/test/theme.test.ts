/** Theme upgrades validate browser data and restore the color scheme before first paint. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { themes, parseTheme } from '../src/lib/features/theme.ts';
import { parsePreferences } from '../src/lib/features/preferences.ts';

void test('supported themes survive parsing and unknown values preserve Mori', () => {
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
  for (const theme of themes) {
    let dark = true;
    const root = {
      dataset: { theme: '' },
      classList: {
        toggle: (_: string, value: boolean) => {
          dark = value;
        },
      },
    };
    runInNewContext(script, {
      localStorage: { getItem: () => JSON.stringify({ theme: theme.id }) },
      document: { documentElement: root },
    });
    assert.equal(root.dataset.theme, theme.id);
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
