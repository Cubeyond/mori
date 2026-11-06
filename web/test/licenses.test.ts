// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: Elastic-2.0

/** Distribution notices must preserve upstream text and never leak build-machine paths. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { dependencyNotices } from '../scripts/legal/generate.ts';

void test('copyright and permission text survives without exposing install paths', () => {
  const directory = mkdtempSync(join(tmpdir(), 'mori-licenses-'));
  try {
    const text =
      'Copyright (c) upstream authors\nPermission is granted under the original terms.\n';
    writeFileSync(join(directory, 'LICENSE.txt'), text);
    const notices = dependencyNotices({
      MIT: [
        {
          name: 'fixture',
          license: 'MIT',
          versions: ['1.0.0'],
          paths: [directory],
        },
      ],
    });
    assert.deepEqual(notices[0]?.notices, [text]);
    assert.ok(!JSON.stringify(notices).includes(directory));
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

void test('missing notices and malformed inventories cannot silently produce a release', () => {
  const directory = mkdtempSync(join(tmpdir(), 'mori-licenses-'));
  try {
    assert.throws(
      () =>
        dependencyNotices({
          MIT: [
            {
              name: 'unknown',
              license: 'MIT',
              versions: ['1'],
              paths: [directory],
            },
          ],
        }),
      /Missing license notice/u,
    );
    if (process.platform !== 'win32') {
      writeFileSync(
        join(directory, 'private.txt'),
        'private build-machine data',
      );
      symlinkSync('private.txt', join(directory, 'LICENSE'));
      assert.throws(
        () =>
          dependencyNotices({
            MIT: [
              {
                name: 'unknown',
                license: 'MIT',
                versions: ['1'],
                paths: [directory],
              },
            ],
          }),
        /Missing license notice/u,
      );
    }
    for (const value of [null, [], { MIT: 'invalid' }, { MIT: [{}] }]) {
      assert.throws(
        () => dependencyNotices(value),
        /Invalid dependency license/u,
      );
    }
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
