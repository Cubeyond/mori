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

/** Mori uses Everforest Dark Hard when a browser has no valid saved theme. */
export function parseTheme(value: unknown): Theme {
  return themes.find((theme) => theme.id === value)?.id ?? 'mori';
}

/** Apply the palette and native-control color scheme without changing wallpaper or media. */
export function applyTheme(value: Theme, accent: Accent): void {
  if (typeof document === 'undefined') return;
  document.documentElement.dataset['theme'] = value;
  document.documentElement.dataset['accent'] = accent;
  document.documentElement.classList.toggle(
    'dark',
    value !== 'catppuccin-latte',
  );
}

/** Accent names map to each theme's own palette rather than replacing page surfaces. */
export const accents = [
  'blue',
  'lavender',
  'mauve',
  'pink',
  'peach',
  'green',
  'teal',
  'red',
] as const;
/** Validated local accent choice. */
export type Accent = (typeof accents)[number];
/** New and incomplete preferences inherit Mori's leaf-green accent. */
export function parseAccent(value: unknown): Accent {
  return accents.find((accent) => accent === value) ?? 'green';
}
