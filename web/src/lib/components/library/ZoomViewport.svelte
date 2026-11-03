<script lang="ts">
  /** Pointer gestures transform pixels while keeping playback and viewer controls fixed. */
  import { onMount, type Snippet } from 'svelte';
  import { SvelteMap } from 'svelte/reactivity';
  import {
    boundView,
    fitMedia,
    zoomAt,
    wheelPixels,
    type View,
  } from '#lib/features/lightbox';
  /** The parent can display scale and provide single-click alternatives to gestures. */
  let {
    width,
    height,
    view = $bindable({ scale: 1, x: 0, y: 0 }),
    doubleTap = false,
    children,
  }: {
    width: number;
    height: number;
    view?: View;
    doubleTap?: boolean;
    children: Snippet<[string]>;
  } = $props();
  /** The entire available stage is measured, independently of the fitted source. */
  let stage: HTMLDivElement;
  let size = $state({ width: 0, height: 0 });
  const fitted = $derived(fitMedia(size, { width, height }));
  /** Active contacts are rebased whenever the pointer count changes. */
  const pointers = new SvelteMap<number, { x: number; y: number }>();
  let previous: { x: number; y: number; distance: number } | undefined;
  let dragged = false,
    suppressUntil = 0,
    lastTap = 0,
    touchedAt = Number.NEGATIVE_INFINITY;
  /** Restore fit without fetching any media bytes. */
  export function reset(): void {
    view = { scale: 1, x: 0, y: 0 };
  }
  /** Zoom buttons anchor at the stage center. */
  export function change(factor: number): void {
    view = zoomAt(view, view.scale * factor, { x: 0, y: 0 }, fitted, size);
  }
  /** Direction buttons and keyboard arrows provide alternatives to dragging. */
  export function pan(x: number, y: number): void {
    view = boundView({ ...view, x: view.x + x, y: view.y + y }, fitted, size);
  }
  /** Playback surfaces accept gestures, but actual buttons, sliders, and links do not. */
  function interactive(target: EventTarget | null): boolean {
    return (
      target instanceof Element &&
      target.closest(
        'button:not(.surface),a,input,select,textarea,[data-no-zoom]',
      ) !== null
    );
  }
  /** Locate a pointer relative to the measured stage center. */
  function point(event: { clientX: number; clientY: number }): {
    x: number;
    y: number;
  } {
    const rect = stage.getBoundingClientRect();
    return {
      x: event.clientX - rect.left - rect.width / 2,
      y: event.clientY - rect.top - rect.height / 2,
    };
  }
  /** One centroid representation supports both one-finger pan and two-finger pinch. */
  function centroid(): { x: number; y: number; distance: number } | undefined {
    const [a, b] = [...pointers.values()];
    if (a === undefined) return undefined;
    return b === undefined
      ? { ...a, distance: 0 }
      : {
          x: (a.x + b.x) / 2,
          y: (a.y + b.y) / 2,
          distance: Math.hypot(a.x - b.x, a.y - b.y),
        };
  }
  /** Track only primary-button contacts on the media plane. */
  function down(event: PointerEvent): void {
    if (event.button !== 0 || interactive(event.target)) return;
    if (pointers.size === 0) dragged = false;
    pointers.set(event.pointerId, point(event));
    previous = centroid();
    if (pointers.size > 1) dragged = true;
    // Capture on the original surface, preserving stationary video clicks.
    if (event.target instanceof Element)
      event.target.setPointerCapture(event.pointerId);
  }
  /** Apply midpoint motion and pointer-anchored scaling in one bounded update. */
  function move(event: PointerEvent): void {
    if (!pointers.has(event.pointerId)) return;
    pointers.set(event.pointerId, point(event));
    const next = centroid(),
      before = previous;
    if (next === undefined || before === undefined) return;
    const dx = next.x - before.x,
      dy = next.y - before.y;
    if (!dragged && Math.hypot(dx, dy) < 3 && next.distance === 0) return;
    dragged = true;
    if (next.distance > 0 && before.distance > 0) {
      const ratio = next.distance / before.distance;
      view = zoomAt(view, view.scale * ratio, before, fitted, size);
    }
    view = boundView({ ...view, x: view.x + dx, y: view.y + dy }, fitted, size);
    previous = next;
  }
  /** Rebase the remaining finger; cancel never creates a tap or play action. */
  function up(event: PointerEvent): void {
    if (!pointers.has(event.pointerId)) return;
    const anchor = point(event);
    pointers.delete(event.pointerId);
    previous = centroid();
    if (event.pointerType === 'touch') touchedAt = performance.now();
    if (dragged || event.type !== 'pointerup') {
      suppressUntil = performance.now() + 400;
      lastTap = 0;
    } else if (
      doubleTap &&
      event.pointerType === 'touch' &&
      pointers.size === 0
    ) {
      const now = performance.now();
      if (lastTap > 0 && now - lastTap < 280) {
        toggle(anchor);
        lastTap = 0;
      } else lastTap = now;
    }
  }
  /** Double click toggles fit and native resolution, with a useful minimum 2× step. */
  function toggle(anchor: { x: number; y: number }): void {
    view = zoomAt(
      view,
      view.scale > 1.01 ? 1 : Math.max(2, width / Math.max(1, fitted.width)),
      anchor,
      fitted,
      size,
    );
  }
  onMount(() => {
    const observer = new ResizeObserver(([entry]) => {
      if (entry === undefined) return;
      size = {
        width: entry.contentRect.width,
        height: entry.contentRect.height,
      };
      view = boundView(view, fitMedia(size, { width, height }), size);
    });
    observer.observe(stage);
    /** Wheel shortcuts outside this plane, including modifier shortcuts, stay native. */
    const wheel = (event: WheelEvent): void => {
      if (
        interactive(event.target) ||
        event.metaKey ||
        event.altKey ||
        Math.abs(event.deltaX) > Math.abs(event.deltaY)
      )
        return;
      event.preventDefault();
      view = zoomAt(
        view,
        view.scale *
          Math.exp(
            -wheelPixels(event.deltaY, event.deltaMode, size.height) * 0.002,
          ),
        point(event),
        fitted,
        size,
      );
    };
    /** A completed pan or pinch must never also toggle playback. */
    const click = (event: MouseEvent): void => {
      if (!interactive(event.target) && performance.now() < suppressUntil) {
        event.preventDefault();
        event.stopPropagation();
      }
    };
    /** Image double click is independent of video playback behavior. */
    const dblclick = (event: MouseEvent): void => {
      if (
        doubleTap &&
        performance.now() - touchedAt > 500 &&
        !interactive(event.target)
      ) {
        event.preventDefault();
        toggle(point(event));
      }
    };
    stage.addEventListener('wheel', wheel, { passive: false });
    stage.addEventListener('pointerdown', down);
    stage.addEventListener('pointermove', move);
    stage.addEventListener('pointerup', up);
    stage.addEventListener('pointercancel', up);
    stage.addEventListener('lostpointercapture', up);
    stage.addEventListener('click', click, true);
    stage.addEventListener('dblclick', dblclick);
    return () => {
      observer.disconnect();
      pointers.clear();
      stage.removeEventListener('wheel', wheel);
      stage.removeEventListener('pointerdown', down);
      stage.removeEventListener('pointermove', move);
      stage.removeEventListener('pointerup', up);
      stage.removeEventListener('pointercancel', up);
      stage.removeEventListener('lostpointercapture', up);
      stage.removeEventListener('click', click, true);
      stage.removeEventListener('dblclick', dblclick);
    };
  });
</script>

<div class="zoom-stage" class:zoomed={view.scale > 1} bind:this={stage}>
  <div
    class="fit-frame"
    style:width={String(fitted.width) + 'px'}
    style:height={String(fitted.height) + 'px'}
  >
    {@render children(
      `translate3d(${String(view.x)}px, ${String(view.y)}px, 0) scale(${String(view.scale)})`,
    )}
  </div>
</div>

<style>
  .zoom-stage {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    overflow: hidden;
    touch-action: none;
    user-select: none;
  }
  .zoomed {
    cursor: grab;
  }
  .zoomed:active {
    cursor: grabbing;
  }
  .fit-frame {
    flex: none;
  }
</style>
