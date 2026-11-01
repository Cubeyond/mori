/** Shared, reference-counted scroll lock for native top-layer dialogs. */

/** Number of mounted open dialogs, including a brief handover between editors. */
let locks = 0;
/** Original inline styles restored after the final modal closes. */
let originalOverflow = '',
  originalPadding = '';

/** Freeze background scrolling while preserving its exact pre-dialog content width. */
export function lockPageScroll(): () => void {
  const root = document.documentElement;
  const body = document.body;
  if (locks === 0) {
    originalOverflow = root.style.overflow;
    originalPadding = body.style.paddingRight;
    const gutter = window.innerWidth - root.clientWidth;
    const padding = Number.parseFloat(getComputedStyle(body).paddingRight) || 0;
    body.style.paddingRight = `${String(padding + gutter)}px`;
    root.style.overflow = 'hidden';
  }
  locks++;
  let released = false;
  return () => {
    if (released) return;
    released = true;
    locks--;
    if (locks === 0) {
      root.style.overflow = originalOverflow;
      body.style.paddingRight = originalPadding;
    }
  };
}
