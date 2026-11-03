/** Final savings must reflect actual completed bytes, not encoder quality or transient progress. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { summarizeUploads } from '../src/lib/features/upload-results.ts';
void test('mixed optimized and original uploads use a byte-weighted total and retain every completion', () => {
  const results = [
    { id: 'a', name: 'photo.jpg', originalBytes: 1000, uploadedBytes: 200 },
    { id: 'b', name: 'photo.jpg', originalBytes: 9000, uploadedBytes: 9000 },
  ];
  assert.deepEqual(summarizeUploads(results), {
    count: 2,
    originalBytes: 10000,
    uploadedBytes: 9200,
    savedBytes: 800,
    savedPercent: 8,
  });
  assert.equal(results[0]?.uploadedBytes, 200);
  assert.deepEqual(summarizeUploads([]), {
    count: 0,
    originalBytes: 0,
    uploadedBytes: 0,
    savedBytes: 0,
    savedPercent: 0,
  });
});
