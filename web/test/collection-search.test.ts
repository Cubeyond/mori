/** Real cache boundaries preserve complete-collection semantics without typing-time requests. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
  CollectionSearch,
  type PageReader,
} from '../src/lib/features/collection-search.ts';
import { videoFixture } from './support/media.ts';

void test('complete metadata filters immediately with literal server-compatible names and exact counts', async () => {
  let calls = 0;
  const rows = [
    { ...videoFixture(0), name: 'Rare 100%_\\ 森' },
    { ...videoFixture(1), name: 'Common' },
  ];
  const read: PageReader = () => {
    calls++;
    return Promise.resolve({ videos: rows, total: 2, has_more: false });
  };
  const search = new CollectionSearch(read),
    signal = new AbortController().signal;
  await search.read('', 0, 30, signal);
  for (const q of ['rare', '%', '_', '\\', '森']) {
    assert.equal(search.cached(q)?.total, 1);
    assert.equal(
      (await search.read(q, 0, 30, signal)).videos[0]?.id,
      rows[0]?.id,
    );
  }
  assert.equal(search.cached('missing')?.total, 0);
  assert.equal(search.cached('')?.total, 2);
  assert.equal(calls, 1);
});
void test('an incomplete page cannot hide matches beyond it; append eventually enables exact local search', async () => {
  const rows = Array.from({ length: 75 }, (_, i) => ({
    ...videoFixture(i),
    name: i === 74 ? 'Rare' : 'Common',
  }));
  const read: PageReader = (offset, limit, q) => {
    const filtered = rows.filter((v) =>
      v.name.toLowerCase().includes(q.toLowerCase()),
    );
    return Promise.resolve({
      videos: filtered.slice(offset, offset + limit),
      total: filtered.length,
      has_more: offset + limit < filtered.length,
    });
  };
  const search = new CollectionSearch(read),
    signal = new AbortController().signal;
  await search.read('', 0, 30, signal);
  assert.equal(search.cached('Rare'), null);
  assert.equal((await search.read('Rare', 0, 30, signal)).total, 1);
  await search.read('', 30, 30, signal);
  await search.read('', 60, 30, signal);
  assert.equal(search.complete, true);
  assert.equal(search.cached('Rare')?.videos[0]?.id, rows[74]?.id);
  assert.equal(search.cached('', 60)?.videos.length, 15);
});
void test('authoritative refresh renews names and counts; scope changes discard private cached metadata', async () => {
  let rows = [{ ...videoFixture(0), name: 'Before' }];
  const read: PageReader = () =>
    Promise.resolve({ videos: rows, total: rows.length, has_more: false });
  const search = new CollectionSearch(read),
    signal = new AbortController().signal;
  await search.read('', 0, 30, signal);
  rows = [{ ...videoFixture(1), name: 'After' }];
  await search.read('Before', 0, 30, signal, true);
  assert.equal(search.cached('Before')?.total, 0);
  assert.equal(search.cached('After')?.total, 1);
  search.clear();
  assert.equal(search.cached('After'), null);
  assert.equal(search.complete, false);
});
void test('an aborted or invalidated response cannot repopulate the cache', async () => {
  const resolvers: (() => void)[] = [];
  const read: PageReader = async () => {
    await new Promise<void>((resolve) => {
      resolvers.push(resolve);
    });
    return { videos: [videoFixture(0)], total: 1, has_more: false };
  };
  for (const action of ['abort', 'clear']) {
    const search = new CollectionSearch(read),
      controller = new AbortController();
    const result = search.read('', 0, 30, controller.signal);
    if (action === 'abort') controller.abort();
    else search.clear();
    resolvers.shift()?.();
    await assert.rejects(result, { name: 'AbortError' });
    assert.equal(search.cached(''), null);
  }
});
