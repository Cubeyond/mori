/** Match the dashboard convention: the current page followed by the product name. */
export function pageTitle(name?: string | null): string {
  const label = name?.trim() ?? '';
  return label.length > 0 ? `${label} – Mori` : 'Mori';
}
