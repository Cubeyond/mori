// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Preserve original dependency notices alongside the browser distribution. */
import {
  copyFileSync,
  readFileSync,
  readdirSync,
  lstatSync,
  writeFileSync,
} from 'node:fs';
import { resolve } from 'node:path';
import { execFileSync } from 'node:child_process';
import overrides from './overrides.json' with { type: 'json' };

/** A distributable record contains upstream attribution and text, never local install paths. */
interface DependencyNotice {
  name: string;
  versions: string[];
  license: string;
  author: string;
  homepage: string;
  notices: string[];
  publishedPackages: string[];
}

/** Require the object shape emitted by the package manager without accepting untyped data. */
function object(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Read a required array of strings and fail closed on a changed inventory format. */
function strings(value: unknown): string[] {
  if (
    !Array.isArray(value) ||
    !value.every((entry: unknown) => typeof entry === 'string')
  ) {
    throw new Error('Invalid dependency license inventory');
  }
  return value;
}

/** Collect complete published license/notice files and reviewed omissions for exact versions. */
export function dependencyNotices(inventory: unknown): DependencyNotice[] {
  if (!object(inventory))
    throw new Error('Invalid dependency license inventory');
  const reviewed: Record<string, { source: string; text: string }> = overrides;
  const result: DependencyNotice[] = [];
  for (const group of Object.values(inventory)) {
    if (!Array.isArray(group))
      throw new Error('Invalid dependency license group');
    const packages: unknown[] = group;
    for (const entry of packages) {
      if (
        !object(entry) ||
        typeof entry['name'] !== 'string' ||
        typeof entry['license'] !== 'string'
      ) {
        throw new Error('Invalid dependency license record');
      }
      const name = entry['name'];
      const versions = strings(entry['versions']).toSorted();
      const notices = new Set<string>();
      for (const directory of strings(entry['paths'])) {
        for (const file of readdirSync(directory).toSorted()) {
          const path = resolve(directory, file);
          if (
            /^(licen[sc]e|copying|notice)([._-]|$)/iu.test(file) &&
            lstatSync(path).isFile()
          ) {
            notices.add(readFileSync(path, 'utf8'));
          }
        }
      }
      if (notices.size === 0) {
        for (const version of versions) {
          const notice = reviewed[`${name}@${version}`];
          if (notice === undefined)
            throw new Error(`Missing license notice: ${name}@${version}`);
          notices.add(`Source: ${notice.source}\n\n${notice.text}`);
        }
      }
      result.push({
        name,
        versions,
        license: entry['license'],
        author: typeof entry['author'] === 'string' ? entry['author'] : '',
        homepage:
          typeof entry['homepage'] === 'string' ? entry['homepage'] : '',
        notices: [...notices].toSorted(),
        publishedPackages: versions.map(
          (version) =>
            `https://registry.npmjs.org/${name}/-/${name.split('/').at(-1) ?? ''}-${version}.tgz`,
        ),
      });
    }
  }
  return result.toSorted((first, second) =>
    first.name.localeCompare(second.name, 'en'),
  );
}

if (import.meta.main) {
  const web = resolve(import.meta.dirname, '../..');
  const inventory: unknown = JSON.parse(
    execFileSync('pnpm', ['licenses', 'list', '--prod', '--json'], {
      cwd: web,
      encoding: 'utf8',
    }),
  );
  writeFileSync(
    resolve(web, 'static/THIRD-PARTY-NOTICES.json'),
    JSON.stringify(dependencyNotices(inventory), null, 2) + '\n',
  );
  copyFileSync(resolve(web, '../LICENSE'), resolve(web, 'static/LICENSE.txt'));
  copyFileSync(resolve(web, '../NOTICE'), resolve(web, 'static/NOTICE.txt'));
}
