/** Verify the build tool's patched merge dependency preserves safe object prototypes. */
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { test } from 'node:test';

/** Minimal boundary for the third-party configuration merge operation. */
type Merge = (target: object, source: object) => Record<string, unknown>;

/** Narrow the CommonJS entry without propagating its untyped require result. */
function isMerge(value: unknown): value is Merge {
  return typeof value === 'function';
}

const requirePlugin = createRequire(
  import.meta.resolve('@sveltejs/vite-plugin-svelte'),
);
for (const entry of ['deepmerge', 'deepmerge/dist/umd.js']) {
  const merge: unknown = requirePlugin(entry);
  assert.ok(isMerge(merge));

  void test(`${entry} cannot change the result prototype`, () => {
    const payload: unknown = JSON.parse('{"__proto__":{"unexpected":true}}');
    assert.ok(typeof payload === 'object' && payload !== null);
    const nullTarget: unknown = Object.create(null);
    assert.ok(typeof nullTarget === 'object' && nullTarget !== null);
    for (const result of [
      merge(payload, {}),
      merge(nullTarget, payload),
      merge({}, { nested: payload }),
    ]) {
      assert.equal(Object.getPrototypeOf(result), Object.prototype);
      assert.equal(result['unexpected'], undefined);
      if (typeof result['nested'] === 'object' && result['nested'] !== null) {
        assert.equal(Object.getPrototypeOf(result['nested']), Object.prototype);
      }
    }
    assert.equal(
      Object.getOwnPropertyDescriptor(Object.prototype, 'unexpected'),
      undefined,
    );
  });

  void test(`${entry} retains nested settings and array values`, () => {
    assert.deepEqual(
      merge(
        { settings: { first: true }, files: ['a'] },
        { settings: { second: true }, files: ['b'] },
      ),
      { settings: { first: true, second: true }, files: ['a', 'b'] },
    );
  });
}
