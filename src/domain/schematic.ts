import type { Point, Size } from '@/domain/generated/types';
import { chipLabels, formatSize, type MonitorIdentity, monitorDisplay } from './monitors';

/** What the schematic needs from a monitor of a summary or a capture preview. */
export interface SchematicMonitor extends MonitorIdentity {
  on: boolean;
  position: Point | null;
  size: Size | null;
  primary: boolean;
}

/** A rectangle in the schematic's own coordinates, labelled and ready to draw. */
export interface SchematicRect {
  devicePath: string;
  x: number;
  y: number;
  width: number;
  height: number;
  primary: boolean;
  /** The alias fallback, with the connector when another monitor shares the name. */
  label: string;
  /**
   * "primary · 1920×1080" or "1920×1080" when the rectangle is wide enough for it,
   * "primary" when only that fits, empty otherwise.
   */
  detail: string;
  /** Too short for the detail line; only the label is drawn. */
  compact: boolean;
}

export interface Box {
  width: number;
  height: number;
}

/** The picture's size on the canvas (artboard 1d). */
export const SCHEMATIC_BOX: Box = { width: 300, height: 172 };
export const SCHEMATIC_PADDING = 12;
const COMPACT_BELOW = 36;
/** The detail line is 9px monospace inset by 6: roughly this many units per character. */
const DETAIL_UNITS_PER_CHAR = 5.4;
const DETAIL_INSET = 12;

/**
 * The on monitors as scaled rectangles inside `box`, preserving aspect ratio and
 * centring the group. The bounding box is shifted to the origin first, so an
 * arrangement with a monitor above or left of the primary (negative coordinates)
 * draws in place. Off monitors are not drawn, and neither is an on monitor whose
 * position Windows did not report.
 */
export function schematicRects(
  monitors: readonly SchematicMonitor[],
  aliases: Readonly<Record<string, string>> = {},
  box: Box = SCHEMATIC_BOX,
): SchematicRect[] {
  const padding = SCHEMATIC_PADDING;
  const placed = monitors.filter(
    (m): m is SchematicMonitor & { position: Point; size: Size } =>
      m.on && m.position !== null && m.size !== null && m.size.width > 0 && m.size.height > 0,
  );
  if (placed.length === 0) {
    return [];
  }
  const labels = chipLabels(placed.map((m) => monitorDisplay(aliases, m)));
  const minX = Math.min(...placed.map((m) => m.position.x));
  const minY = Math.min(...placed.map((m) => m.position.y));
  const maxX = Math.max(...placed.map((m) => m.position.x + m.size.width));
  const maxY = Math.max(...placed.map((m) => m.position.y + m.size.height));
  const innerWidth = Math.max(box.width - 2 * padding, 1);
  const innerHeight = Math.max(box.height - 2 * padding, 1);
  const scale = Math.min(innerWidth / (maxX - minX), innerHeight / (maxY - minY));
  const offsetX = padding + (innerWidth - (maxX - minX) * scale) / 2;
  const offsetY = padding + (innerHeight - (maxY - minY) * scale) / 2;

  return placed.map((m, i) => {
    const width = round(m.size.width * scale);
    const height = round(m.size.height * scale);
    return {
      devicePath: m.devicePath,
      x: round((m.position.x - minX) * scale + offsetX),
      y: round((m.position.y - minY) * scale + offsetY),
      width,
      height,
      primary: m.primary,
      label: labels[i] ?? m.reportedName,
      detail: detailThatFits(m.primary, formatSize(m.size, null), width),
      compact: height < COMPACT_BELOW,
    };
  });
}

/** The longest of the detail variants that fits the rectangle, so text is never cut. */
function detailThatFits(primary: boolean, size: string, width: number): string {
  const candidates = primary ? [`primary · ${size}`, 'primary'] : [size];
  return candidates.find((text) => text.length * DETAIL_UNITS_PER_CHAR + DETAIL_INSET <= width) ?? '';
}

function round(value: number): number {
  return Math.round(value * 100) / 100;
}
