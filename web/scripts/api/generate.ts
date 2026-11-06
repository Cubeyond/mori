// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Generate browser contracts from Rust OpenAPI, including actual binary body types. */
import { writeFile } from 'node:fs/promises';
import openapiTS, { astToString } from 'openapi-typescript';
import ts from 'typescript';

const schema = new URL('../../../api/openapi.yml', import.meta.url);
const output = new URL(
  '../../src/lib/api/generated/schema.d.ts',
  import.meta.url,
);
const declarations = await openapiTS(schema, {
  /** Binary OpenAPI request bodies are browser Blobs, not JSON strings. */
  transform(value) {
    if (value.format === 'binary')
      return ts.factory.createTypeReferenceNode('Blob');
    return undefined;
  },
});
await writeFile(output, astToString(declarations));
