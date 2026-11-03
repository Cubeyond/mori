/** Scoped metadata reuse keeps complete collections searchable without a network round trip. */
import type { components } from '../api/generated/schema';
/** Both album endpoints add metadata to the generated media-page contract. */
export type CollectionPage = components['schemas']['VideoPage'] & {
  /** Absent from the administrator's global gallery. */
  album?: components['schemas']['Album'];
};
/** The caller binds authorization and collection scope, never this cache. */
export type PageReader = (
  offset: number,
  limit: number,
  q: string,
  signal: AbortSignal,
) => Promise<CollectionPage>;
/** SQLite LIKE folds ASCII only; literal punctuation and Unicode retain the server's semantics. */
function normalize(value: string): string {
  return value.replace(/[A-Z]/gu, (letter) => letter.toLowerCase());
}
/** Query entries expire quickly; complete catalogs are refreshed by the existing page poll. */
interface Snapshot {
  /** Contiguous metadata window beginning at offset zero. */
  page: CollectionPage;
  /** Monotonic creation time, independent of wall-clock changes. */
  time: number;
}
/** One in-memory instance belongs to one authorized collection and is cleared on scope changes. */
export class CollectionSearch {
  /** A complete unfiltered catalog permits exact local filtering and counts. */
  private catalog: CollectionPage | null = null;
  /** Bounded recent windows make clearing/repeating a query immediate. */
  private readonly windows = new Map<string, Snapshot>();
  /** Discard in-flight work even if its transport ignores abort. */
  private generation = 0;
  /** The existing generated endpoint remains authoritative. */
  private readonly reader: PageReader;
  /** Bind the already authorized generated endpoint for this collection. */
  constructor(reader: PageReader) {
    this.reader = reader;
  }
  /** Seed only an unfiltered window returned for this exact route and session. */
  seed(page: CollectionPage): void {
    this.remember('', page);
  }
  /** Drop all metadata after mutation, navigation, or authorization changes. */
  clear(): void {
    this.generation++;
    this.catalog = null;
    this.windows.clear();
  }
  /** A complete catalog requires no input debounce or API request. */
  get complete(): boolean {
    return this.catalog !== null;
  }
  /** Return exact cached results only; an incomplete loaded page is never treated as the collection. */
  cached(query: string, offset = 0, depth = 30): CollectionPage | null {
    const q = normalize(query.trim());
    if (this.catalog !== null) {
      const rows = this.catalog.videos.filter((item) =>
        normalize(item.name).includes(q),
      );
      return {
        ...this.catalog,
        videos: rows.slice(offset, offset + depth),
        total: rows.length,
        has_more: offset + depth < rows.length,
      };
    }
    const snapshot = this.windows.get(q);
    if (snapshot === undefined || performance.now() - snapshot.time > 15000)
      return null;
    const page = snapshot.page;
    if (offset + depth > page.videos.length && page.has_more) return null;
    return {
      ...page,
      videos: page.videos.slice(offset, offset + depth),
      has_more: page.has_more || offset + depth < page.videos.length,
    };
  }
  /** Read a bounded window; background refreshes renew a complete catalog rather than one query. */
  async read(
    query: string,
    offset: number,
    depth: number,
    signal: AbortSignal,
    authoritative = false,
  ): Promise<CollectionPage> {
    signal.throwIfAborted();
    if (!authoritative) {
      const cached = this.cached(query, offset, depth);
      if (cached !== null) return cached;
    }
    const generation = this.generation,
      full = authoritative && this.catalog !== null;
    const q = full ? '' : query.trim(),
      start = full ? 0 : offset;
    const target = full ? Math.max(30, this.catalog?.total ?? 0) : depth;
    if (authoritative) this.windows.clear();
    let position = start,
      page: CollectionPage | undefined;
    const rows: CollectionPage['videos'] = [];
    do {
      page = await this.reader(
        position,
        Math.min(60, target - rows.length),
        q,
        signal,
      );
      signal.throwIfAborted();
      if (generation !== this.generation)
        throw new DOMException('Collection changed', 'AbortError');
      rows.push(...page.videos);
      position += page.videos.length;
    } while (page.has_more && rows.length < target && page.videos.length > 0);
    const result = { ...page, videos: rows };
    const previous = this.windows.get(normalize(q));
    if (start === 0) this.remember(q, result);
    else if (previous?.page.videos.length === start)
      this.remember(q, {
        ...result,
        videos: [...previous.page.videos, ...rows],
      });
    if (full) {
      const cached = this.cached(query, offset, depth);
      if (cached !== null) return cached;
      if (query.trim() !== '') return this.read(query, offset, depth, signal);
    }
    return result;
  }
  /** Never infer completeness from counts alone or retain an unbounded query history. */
  private remember(query: string, page: CollectionPage): void {
    const q = normalize(query.trim());
    this.windows.delete(q);
    this.windows.set(q, { page, time: performance.now() });
    if (q === '')
      this.catalog =
        !page.has_more && page.videos.length === page.total ? page : null;
    if (this.windows.size > 8) {
      const oldest = this.windows.keys().next().value;
      if (oldest !== undefined) this.windows.delete(oldest);
    }
  }
}
