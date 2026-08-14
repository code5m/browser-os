import type { LogicalRect } from './types';

/**
 * Convert a DOMRect to a LogicalRect.
 *
 * DOMRect already uses CSS pixels, which are logical pixels.
 * This function just extracts the relevant fields.
 */
export function domRectToLogical(rect: DOMRect): LogicalRect {
  return {
    x: rect.x,
    y: rect.y,
    width: rect.width,
    height: rect.height,
  };
}

/**
 * Check if a rect is valid (non-zero size).
 */
export function isValidRect(rect: LogicalRect): boolean {
  return rect.width > 0 && rect.height > 0;
}

/**
 * Check if two rects are approximately equal.
 */
export function rectsEqual(a: LogicalRect, b: LogicalRect, epsilon = 0.5): boolean {
  return (
    Math.abs(a.x - b.x) < epsilon &&
    Math.abs(a.y - b.y) < epsilon &&
    Math.abs(a.width - b.width) < epsilon &&
    Math.abs(a.height - b.height) < epsilon
  );
}

/**
 * Clamp a rect to fit within a container rect.
 */
export function clampRect(rect: LogicalRect, container: LogicalRect): LogicalRect {
  const x = Math.max(rect.x, container.x);
  const y = Math.max(rect.y, container.y);
  const right = Math.min(rect.x + rect.width, container.x + container.width);
  const bottom = Math.min(rect.y + rect.height, container.y + container.height);

  return {
    x,
    y,
    width: Math.max(0, right - x),
    height: Math.max(0, bottom - y),
  };
}

/**
 * Scale a rect by a factor.
 */
export function scaleRect(rect: LogicalRect, factor: number): LogicalRect {
  return {
    x: rect.x * factor,
    y: rect.y * factor,
    width: rect.width * factor,
    height: rect.height * factor,
  };
}

/**
 * Get the rect of an element relative to another element.
 */
export function getRelativeRect(element: HTMLElement, container: HTMLElement): LogicalRect {
  const elementRect = element.getBoundingClientRect();
  const containerRect = container.getBoundingClientRect();

  return {
    x: elementRect.x - containerRect.x,
    y: elementRect.y - containerRect.y,
    width: elementRect.width,
    height: elementRect.height,
  };
}
