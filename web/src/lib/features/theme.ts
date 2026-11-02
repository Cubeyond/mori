/** Browser-local themes use upstream palette values through shared semantic CSS tokens. */
export const themes = [
  { id: 'mori', label: 'Mori', dark: true },
  { id: 'catppuccin-latte', label: 'Catppuccin Latte', dark: false },
  { id: 'catppuccin-frappe', label: 'Catppuccin Frappé', dark: true },
  { id: 'catppuccin-macchiato', label: 'Catppuccin Macchiato', dark: true },
  { id: 'catppuccin-mocha', label: 'Catppuccin Mocha', dark: true },
  { id: 'tokyonight', label: 'Tokyo Night', dark: true },
] as const;

/** Only catalog entries can be restored from untrusted browser persistence. */
export type Theme = (typeof themes)[number]['id'];

/** Fall back to Mori for obsolete, missing, or corrupted stored choices. */
export function parseTheme(value: unknown): Theme {
  return themes.find((theme) => theme.id === value)?.id ?? 'mori';
}

/** Apply the palette and native-control color scheme without changing wallpaper or media. */
export function applyTheme(value: Theme): void {
  if (typeof document === 'undefined') return;
  document.documentElement.dataset['theme'] = value;
  document.documentElement.classList.toggle(
    'dark',
    value !== 'catppuccin-latte',
  );
}
