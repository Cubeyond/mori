/** Browser monitoring for durable publication jobs; stopping monitoring never cancels a server job. */
import { api, apiData } from '#lib/api/client';

/** Wait for a persisted operation to complete while allowing the owning dialog to close. */
export async function waitForJob(
  id: string,
  signal: AbortSignal,
): Promise<void> {
  const deadline = performance.now() + 120_000;
  while (performance.now() < deadline) {
    signal.throwIfAborted();
    const job = await apiData(
      api.GET('/v1/admin/jobs/{id}', { params: { path: { id } }, signal }),
    );
    if (job.state === 'complete') return;
    if (job.state !== 'pending')
      throw new Error(job.error ?? 'Publication could not be completed.');
    await new Promise<void>((resolve) => {
      setTimeout(resolve, 750);
    });
  }
  throw new Error(
    'Publication is still processing. Reopen this video in a moment.',
  );
}
