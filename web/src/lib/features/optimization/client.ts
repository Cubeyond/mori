/** Lazy local preparation with bounded workers, disk-backed resumes and original-file fallback. */
import {
  optimizationConcurrency,
  parsePreparedCopy,
  worthwhile,
  type OptimizationMode,
  type PreparedCopy,
  type OptimizationRequest,
} from '#lib/features/optimization/model';
import {
  hasPreparedStorage,
  readPrepared,
  removePrepared,
} from '#lib/features/optimization/storage';
/** Cache attempts across pauses without encoding the same original twice in one tab. */
const copies = new Map<File, PreparedCopy | null>();
/** The selected original and its exact optional disk-backed upload representation. */
export interface PreparedUpload {
  /** Payload may differ from the original only before a multipart reservation is created. */
  file: File;
  /** Persist this reference with the reservation to resume identical bytes after reload. */
  copy?: PreparedCopy;
}
/** Release one completed/cancelled file, or all local work after an explicit batch cancellation. */
export async function discardPrepared(file?: File): Promise<void> {
  const selected = file === undefined ? [...copies.keys()] : [file];
  await Promise.all(
    selected.map(async (original) => {
      const copy = copies.get(original);
      copies.delete(original);
      if (copy != null) await removePrepared(copy);
    }),
  );
}
/** One transfer batch bounds total preparation time and prevents simultaneous video encoders. */
export class OptimizationBatch {
  /** At most two workers, scaled down on smaller machines. */
  private readonly limit = optimizationConcurrency(
    navigator.hardwareConcurrency || 1,
  );
  /** File workers share a finite preparation allowance rather than a per-file unbounded delay. */
  private remainingMs = 20_000;
  /** Occupied local workers exclude network upload workers. */
  private active = 0;
  /** A single hardware/software video pipeline avoids competing decoders. */
  private videoActive = false;
  /** Waiters are woken on release and removed on cancellation. */
  private readonly waiters = new Set<() => void>();
  /** Capture preferences and authoritative allowed formats once for this transfer. */
  private readonly mode: OptimizationMode;
  /** Server policy controls whether optimized formats can be uploaded. */
  private readonly extensions: readonly string[];
  /** Capture preferences and authoritative allowed formats once for this transfer. */
  constructor(mode: OptimizationMode, extensions: readonly string[]) {
    this.mode = mode;
    this.extensions = extensions;
  }
  /** Wait without polling; an interrupted queue must never acquire a preparation slot. */
  private async acquire(video: boolean, signal: AbortSignal): Promise<void> {
    while (this.active >= this.limit || (video && this.videoActive)) {
      await new Promise<void>((resolve, reject) => {
        /** Detach both wake paths regardless of which one wins. */
        const cleanup = (): void => {
          this.waiters.delete(wake);
          signal.removeEventListener('abort', abort);
        };
        /** Retry capacity checks when another worker releases its slot. */
        const wake = (): void => {
          cleanup();
          resolve();
        };
        /** Pausing must stop preparation rather than silently starting the original upload. */
        const abort = (): void => {
          cleanup();
          reject(new DOMException('Upload paused', 'AbortError'));
        };
        this.waiters.add(wake);
        signal.addEventListener('abort', abort, { once: true });
        if (signal.aborted) abort();
      });
    }
    signal.throwIfAborted();
    this.active++;
    if (video) this.videoActive = true;
  }
  /** Prefer an existing exact copy; unsupported or unprofitable work uses the original. */
  async prepare(
    file: File,
    kind: 'image' | 'video',
    signal: AbortSignal,
  ): Promise<PreparedUpload> {
    signal.throwIfAborted();
    const cached = copies.get(file);
    if (cached !== undefined) {
      const payload = cached === null ? null : await readPrepared(cached);
      if (payload !== null && cached !== null)
        return { file: payload, copy: cached };
      if (cached === null) return { file };
      copies.delete(file);
    }
    const extension = kind === 'image' ? 'webp' : 'mp4';
    if (
      this.mode === 'original' ||
      !this.extensions.includes(extension) ||
      !hasPreparedStorage() ||
      typeof Worker === 'undefined' ||
      file.size < (kind === 'image' ? 65_536 : 1_048_576) ||
      (kind === 'image' && file.size > 32 << 20)
    )
      return { file };
    await this.acquire(kind === 'video', signal);
    const budgetMs = Math.min(
      this.remainingMs,
      kind === 'image' ? 3000 : 12_000,
    );
    this.remainingMs -= budgetMs;
    const started = performance.now();
    try {
      if (budgetMs < 500) return { file };
      const memory: unknown =
        'deviceMemory' in navigator ? navigator.deviceMemory : undefined;
      const request: OptimizationRequest = {
        file,
        kind,
        mode: this.mode,
        // A private destination is assigned before encoding; only finalized size is persisted.
        copy: {
          id: crypto.randomUUID().replaceAll('-', ''),
          extension,
          size: 16,
          created_at: Date.now(),
        },
        maxPixels:
          typeof memory === 'number' && memory <= 4 ? 12_000_000 : 24_000_000,
        budgetMs,
      };
      const copy = await this.process(request, signal);
      copies.set(file, copy);
      const payload = copy === null ? null : await readPrepared(copy);
      if (copy !== null && payload !== null) return { file: payload, copy };
      if (copy !== null) {
        await removePrepared(copy);
        copies.set(file, null);
      }
      return { file };
    } finally {
      this.remainingMs += Math.max(0, budgetMs - (performance.now() - started));
      this.active--;
      if (kind === 'video') this.videoActive = false;
      for (const wake of [...this.waiters]) wake();
    }
  }
  /** A hard deadline also covers blocked browser decoders; partial disk output is always removed. */
  private async process(
    request: OptimizationRequest,
    signal: AbortSignal,
  ): Promise<PreparedCopy | null> {
    let worker: Worker | undefined;
    let result: PreparedCopy | null = null;
    try {
      worker = new Worker(new URL('./worker.ts', import.meta.url), {
        type: 'module',
      });
      const activeWorker = worker;
      result = await new Promise<PreparedCopy | null>((resolve, reject) => {
        /** Settle one result and release timeout and abort handlers together. */
        const finish = (copy: PreparedCopy | null, aborted = false): void => {
          clearTimeout(timer);
          signal.removeEventListener('abort', abort);
          if (aborted) reject(new DOMException('Upload paused', 'AbortError'));
          else resolve(copy);
        };
        /** Cancel rather than falling back to an upload the user has just paused. */
        const abort = (): void => {
          finish(null, true);
        };
        const timer = setTimeout(() => {
          finish(null);
        }, request.budgetMs);
        activeWorker.onerror = () => {
          finish(null);
        };
        activeWorker.onmessage = (event: MessageEvent<unknown>) => {
          const value = event.data;
          if (
            value === null ||
            typeof value !== 'object' ||
            !('type' in value) ||
            value.type !== 'done' ||
            !('copy' in value)
          )
            return;
          const copy = parsePreparedCopy(value.copy);
          finish(
            copy !== null &&
              copy.id === request.copy.id &&
              copy.extension === request.copy.extension &&
              worthwhile(request.file.size, copy.size)
              ? copy
              : null,
          );
        };
        signal.addEventListener('abort', abort, { once: true });
        if (signal.aborted) abort();
        else {
          try {
            activeWorker.postMessage(request);
          } catch {
            finish(null);
          }
        }
      });
      return result;
    } catch {
      signal.throwIfAborted();
      return null;
    } finally {
      worker?.terminate();
      if (result === null) await removePrepared(request.copy);
    }
  }
}
