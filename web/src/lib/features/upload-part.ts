// SPDX-FileCopyrightText: 2026 Cubeyond
// SPDX-License-Identifier: AGPL-3.0-or-later

/** Direct S3 transfer with byte progress and immediate cancellation; no application proxy. */
export function putUploadPart(
  url: string,
  chunk: Blob,
  checksum: string,
  signal: AbortSignal,
  onprogress: (bytes: number) => void,
): Promise<void> {
  signal.throwIfAborted();
  return new Promise<void>((resolve, reject) => {
    const request = new XMLHttpRequest();
    /** Release the listener on every terminal path, including retries. */
    function finish(error?: Error): void {
      signal.removeEventListener('abort', abort);
      request.upload.onprogress = null;
      request.onload = request.onerror = request.onabort = null;
      if (error === undefined) resolve();
      else reject(error);
    }
    /** Abort the active socket rather than merely hiding its progress. */
    function abort(): void {
      request.abort();
      finish(new DOMException('Upload paused', 'AbortError'));
    }
    request.upload.onprogress = (event) => {
      onprogress(event.loaded);
    };
    request.onload = () => {
      finish(
        request.status >= 200 && request.status < 300
          ? undefined
          : new Error(`Part upload failed (${String(request.status)})`),
      );
    };
    request.onerror = () => {
      finish(
        new Error(
          'Could not reach Amazon S3. Check your connection and retry.',
        ),
      );
    };
    request.onabort = () => {
      finish(new DOMException('Upload paused', 'AbortError'));
    };
    request.open('PUT', url);
    request.setRequestHeader('x-amz-checksum-sha256', checksum);
    signal.addEventListener('abort', abort, { once: true });
    request.send(chunk);
  });
}
