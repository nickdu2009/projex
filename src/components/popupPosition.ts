/**
 * Popup positioning for Tiptap suggestion popups.
 *
 * Bypasses suggestion's clientRect() (unreliable in Tauri WebView with scroll containers)
 * and uses ProseMirror's coordsAtPos() to get accurate viewport coordinates.
 */
import type { Editor } from '@tiptap/core';

export interface PopupHandle {
  /** Re-position the popup (call on every suggestion update) */
  reposition: () => void;
  /** Update the anchor document position for future repositions */
  setAnchorPos: (pos: number) => void;
  /** Remove the popup from the DOM */
  destroy: () => void;
  /** The wrapper element (for appending ReactRenderer content) */
  element: HTMLDivElement;
}

const GAP = 4;
const MAX_HEIGHT = 320;

function isScrollable(el: HTMLElement) {
  const style = window.getComputedStyle(el);
  const overflowY = style.overflowY;
  return (
    (overflowY === 'auto' || overflowY === 'scroll' || overflowY === 'overlay') &&
    el.scrollHeight > el.clientHeight + 1
  );
}

function findScrollContainer(start: HTMLElement): HTMLElement | null {
  let el: HTMLElement | null = start;
  while (el) {
    if (isScrollable(el)) return el;
    el = el.parentElement;
  }
  return null;
}

/**
 * Create a popup positioned at the editor's current cursor.
 *
 * @param editor – the Tiptap editor instance
 */
export function createSuggestionPopup(editor: Editor, initialAnchorPos: number): PopupHandle {
  let anchorPos = initialAnchorPos;

  const anchorRoot = editor.view.dom as HTMLElement;
  const scrollContainer = findScrollContainer(anchorRoot) ?? document.body;

  // Create a relative overlay at the top-left of the scroll container's content box.
  // This makes absolutely positioned popups stable even when the app uses a nested scroll container.
  const overlay = document.createElement('div');
  overlay.style.position = 'relative';
  overlay.style.zIndex = '9999';
  overlay.style.width = '0';
  overlay.style.height = '0';
  overlay.style.pointerEvents = 'none';
  // Put overlay early to avoid being affected by later stacking contexts inside content.
  scrollContainer.prepend(overlay);

  const popup = document.createElement('div');
  popup.style.position = 'absolute';
  popup.style.pointerEvents = 'auto';
  overlay.appendChild(popup);

  const onScroll = () => reposition();
  const onResize = () => reposition();
  if (scrollContainer !== document.body) {
    scrollContainer.addEventListener('scroll', onScroll, { passive: true });
  }
  window.addEventListener('resize', onResize, { passive: true });

  function reposition() {
    const { view } = editor;
    // coordsAtPos returns accurate viewport-relative coordinates
    const coords = view.coordsAtPos(anchorPos);

    // Compute available space within the scroll container's visible viewport.
    const containerRect =
      scrollContainer === document.body
        ? {
            top: 0,
            left: 0,
            width: window.innerWidth,
            height: window.innerHeight,
          }
        : scrollContainer.getBoundingClientRect();

    const style = scrollContainer === document.body ? null : window.getComputedStyle(scrollContainer);
    const padTop = style ? parseFloat(style.paddingTop || '0') : 0;
    const padBottom = style ? parseFloat(style.paddingBottom || '0') : 0;
    const padLeft = style ? parseFloat(style.paddingLeft || '0') : 0;
    const borderTop = style ? parseFloat(style.borderTopWidth || '0') : 0;
    const borderLeft = style ? parseFloat(style.borderLeftWidth || '0') : 0;

    // Align to the scroll container's content box (children are laid out inside it).
    const contentViewportTop = containerRect.top + borderTop + padTop;
    const contentViewportHeight =
      scrollContainer === document.body ? window.innerHeight : scrollContainer.clientHeight - padTop - padBottom;
    const viewportTop = contentViewportTop;
    const viewportBottom = contentViewportTop + contentViewportHeight;

    const spaceBelow = viewportBottom - coords.bottom - GAP;
    const spaceAbove = coords.top - viewportTop - GAP;

    // Convert viewport coords → scroll container content coords.
    // This keeps the popup stable when the main content scrolls (Mantine AppShell).
    const scrollTop = scrollContainer === document.body ? window.scrollY : scrollContainer.scrollTop;
    const scrollLeft = scrollContainer === document.body ? window.scrollX : scrollContainer.scrollLeft;
    const contentOriginLeft = containerRect.left + borderLeft + padLeft;
    const contentOriginTop = containerRect.top + borderTop + padTop;
    const leftInContent = coords.left - contentOriginLeft + scrollLeft;
    const topInContent = coords.top - contentOriginTop + scrollTop;
    const bottomInContent = coords.bottom - contentOriginTop + scrollTop;

    popup.style.left = `${leftInContent}px`;

    // Measure actual popup content height
    const contentH = popup.firstElementChild
      ? (popup.firstElementChild as HTMLElement).scrollHeight
      : MAX_HEIGHT;
    const neededH = Math.min(contentH, MAX_HEIGHT);

    if (spaceBelow >= neededH) {
      // Place below cursor
      popup.style.top = `${bottomInContent + GAP}px`;
    } else if (spaceAbove >= neededH) {
      // Flip above cursor
      popup.style.top = `${topInContent - GAP - neededH}px`;
    } else if (spaceAbove > spaceBelow) {
      // Above, constrained
      const h = Math.min(spaceAbove, MAX_HEIGHT);
      popup.style.top = `${topInContent - GAP - h}px`;
      const child = popup.firstElementChild as HTMLElement | null;
      if (child) child.style.maxHeight = `${h}px`;
    } else {
      // Below, constrained
      popup.style.top = `${bottomInContent + GAP}px`;
      const child = popup.firstElementChild as HTMLElement | null;
      if (child) child.style.maxHeight = `${spaceBelow}px`;
    }
  }

  // Initial position (use rAF to let React render the content first)
  requestAnimationFrame(reposition);

  return {
    reposition,
    setAnchorPos: (pos: number) => {
      anchorPos = pos;
    },
    destroy: () => {
      window.removeEventListener('resize', onResize);
      if (scrollContainer !== document.body) {
        scrollContainer.removeEventListener('scroll', onScroll);
      }
      overlay.remove();
    },
    element: popup,
  };
}
