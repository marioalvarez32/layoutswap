import type { WindowSize } from '@/domain/generated/types';

/**
 * The window-size persistence rule. Given the size last written to the config and the
 * size the window shows now, returns the size to write, or null when nothing should be
 * written: the window is minimised (a zero dimension) or nothing changed. Fractional
 * logical pixels are rounded so the config holds whole numbers.
 */
export function windowSizeToPersist(
  persisted: WindowSize | null,
  observed: WindowSize,
): WindowSize | null {
  const next = { width: Math.round(observed.width), height: Math.round(observed.height) };
  if (next.width <= 0 || next.height <= 0) {
    return null;
  }
  if (persisted && persisted.width === next.width && persisted.height === next.height) {
    return null;
  }
  return next;
}
