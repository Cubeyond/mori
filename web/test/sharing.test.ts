/** Public sharing contracts protect link semantics and embeddable HTML attributes. */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { shareValue } from '../src/lib/features/sharing.ts';
import type { MediaItem } from '../src/lib/api/client';

/** Full DTO fixture keeps sharing tests aligned with the generated API schema. */
const video: MediaItem = {
  kind: 'video',
  id: 'a'.repeat(32),
  token: 'b'.repeat(32),
  extension: 'mp4',
  name: 'Evening',
  album_id: null,
  width: 1920,
  height: 1080,
  visibility: 'public',
  state: 'published',
  size: 1024,
  duration: 20,
  poster: 1,
  created_at: 0,
  revision: 1,
  last_error: null,
  media_url: 'https://media.laplacian.net/file/' + 'b'.repeat(32) + '.mp4',
  poster_url: null,
};

void test('iframe uses the same direct media URL and permits fullscreen', () => {
  assert.equal(shareValue(video, 'media'), video.media_url);
  const iframe = shareValue(video, 'iframe');
  assert.ok(iframe.includes(`src="${video.media_url}"`));
  assert.ok(iframe.includes('allowfullscreen'));
  assert.ok(iframe.includes('allow="fullscreen *; picture-in-picture *"'));
  assert.ok(iframe.includes('referrerpolicy="no-referrer"'));
});

void test('album and standalone iframe sources preserve their public media paths', () => {
  for (const prefix of ['showcase', 'file']) {
    const media_url = `https://media.laplacian.net/${prefix}/${video.token}.webm`;
    const iframe = shareValue(
      { ...video, extension: 'webm', media_url },
      'iframe',
    );
    assert.ok(iframe.includes(`src="${media_url}"`));
    assert.ok(!iframe.includes('/embed/'));
  }
  const iframe = shareValue({ ...video, width: 0, height: 0 }, 'iframe');
  assert.ok(iframe.includes('aspect-ratio:16 / 9'));
});

void test('edited display names cannot inject markup or iframe attributes', () => {
  const iframe = shareValue(
    { ...video, name: 'A "frame" & <script> with apostrophes\'' },
    'iframe',
  );
  assert.ok(iframe.includes('title="A &quot;frame&quot; &amp; &lt;script>'));
  assert.ok(!iframe.includes('<script>'));
  assert.equal(iframe.match(/<iframe /g)?.length, 1);
  assert.equal(iframe.match(/<\/iframe>/g)?.length, 1);
});

void test('images offer their original address without video iframe markup', () => {
  const image: MediaItem = {
    ...video,
    kind: 'image',
    extension: 'jpg',
    width: 800,
    height: 1200,
    media_url: 'https://media.laplacian.net/nature/' + video.token + '.jpg',
  };
  assert.equal(shareValue(image, 'media'), image.media_url);
  assert.throws(
    () => shareValue(image, 'iframe'),
    /Images share their direct link/,
  );
});
