/** Completed byte accounting is independent of mutable selections and live transfer progress. */
export interface UploadResult {
  /** Local result identity distinguishes repeated filenames without retaining File objects. */
  id: string;
  /** Original picker name appears only in the current browser session. */
  name: string;
  /** Bytes selected before optional preparation. */
  originalBytes: number;
  /** Exact prepared payload accepted by the upload operation. */
  uploadedBytes: number;
}
/** One batch summary includes completed files only, including originals that were kept. */
export interface UploadSummary {
  /** Successfully completed originals. */
  count: number;
  /** Sum of original byte counts. */
  originalBytes: number;
  /** Sum of actual upload byte counts. */
  uploadedBytes: number;
  /** Absolute byte reduction, never a quality-setting percentage. */
  savedBytes: number;
  /** Byte-weighted reduction, rather than an average of individual file percentages. */
  savedPercent: number;
}
/** Sum completed payloads without counting queued, failed, removed or retried attempts twice. */
export function summarizeUploads(
  results: readonly UploadResult[],
): UploadSummary {
  let originalBytes = 0,
    uploadedBytes = 0;
  for (const result of results) {
    originalBytes += result.originalBytes;
    uploadedBytes += result.uploadedBytes;
  }
  const savedBytes = Math.max(0, originalBytes - uploadedBytes);
  return {
    count: results.length,
    originalBytes,
    uploadedBytes,
    savedBytes,
    savedPercent:
      originalBytes === 0 ? 0 : Math.round((savedBytes / originalBytes) * 100),
  };
}
