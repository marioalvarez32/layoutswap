import type { Inventory, Monitor, MonitorState, Point, Size } from '@/domain/generated/types';

/** The fields every monitor-like record carries, whether from a probe or a summary. */
export interface MonitorIdentity {
  devicePath: string;
  reportedName: string;
  connector: string;
}

export interface MonitorDisplay {
  /** The alias, or the reported name when no alias is set. */
  label: string;
  /** What tells this monitor apart: the reported name under an alias, else the connector. */
  detail: string;
}

/**
 * The alias fallback: a monitor shows its alias wherever it appears; without one it
 * shows the reported name plus connector, so two identical panels stay distinguishable.
 */
export function monitorDisplay(aliases: Readonly<Record<string, string>>, monitor: MonitorIdentity): MonitorDisplay {
  const alias = aliases[monitor.devicePath]?.trim();
  if (alias) {
    return { label: alias, detail: monitor.reportedName };
  }
  return { label: monitor.reportedName, detail: monitor.connector };
}

/**
 * A short name per monitor for a chip row: the label alone, or "label · detail" when
 * another monitor in the same list shares the label, so two identical panels read apart.
 */
export function chipLabels(displays: readonly MonitorDisplay[]): string[] {
  const counts = new Map<string, number>();
  for (const d of displays) {
    counts.set(d.label, (counts.get(d.label) ?? 0) + 1);
  }
  return displays.map((d) => ((counts.get(d.label) ?? 0) > 1 ? `${d.label} · ${d.detail}` : d.label));
}

/** "3440×1440 · 120 Hz". */
export function formatSize(size: Size | null, refreshHz: number | null): string {
  if (!size) {
    return '';
  }
  const dims = `${size.width}×${size.height}`;
  return refreshHz === null ? dims : `${dims} · ${formatRefresh(refreshHz)}`;
}

export function formatRefresh(refreshHz: number): string {
  const rounded = Math.round(refreshHz * 100) / 100;
  return `${Number.isInteger(rounded) ? rounded : rounded.toFixed(2).replace(/\.?0+$/, '')} Hz`;
}

/** "0,0". */
export function formatPosition(position: Point | null): string {
  return position ? `${position.x},${position.y}` : '';
}

export function formatRotation(rotation: number | null): string {
  switch (rotation) {
    case 90:
      return 'portrait';
    case 180:
      return 'landscape, flipped';
    case 270:
      return 'portrait, flipped';
    default:
      return 'landscape';
  }
}

export function formatScale(scalePercent: number | null): string {
  return scalePercent === null ? '' : `${scalePercent}%`;
}

/** The one-line spec of an on monitor: "0,0 · 3440×1440 · 120 Hz · landscape · 100%". */
export function formatSpec(monitor: {
  position: Point | null;
  size: Size | null;
  refreshHz: number | null;
  rotation: number | null;
  scalePercent: number | null;
}): string {
  return [
    formatPosition(monitor.position),
    formatSize(monitor.size, monitor.refreshHz),
    formatRotation(monitor.rotation),
    formatScale(monitor.scalePercent),
  ].filter((part) => part.length > 0).join(' · ');
}

/** The one-line note the state chip carries (DESIGN.md, Q2). */
export function monitorStateNote(state: MonitorState): string {
  switch (state) {
    case 'Active':
      return '';
    case 'Available':
      return 'Plugged in and showing the PC, but Windows is not drawing to it.';
    case 'Absent':
      return 'Unplugged, powered off, or showing another device: press its input button or plug it in.';
  }
}

/**
 * What the live state means for a monitor in a layout, with the physical action first
 * when one is needed. Empty when nothing needs saying.
 */
export function inLayoutHint(label: string, on: boolean, state: MonitorState): string {
  if (state === 'Active') {
    return '';
  }
  if (state === 'Available') {
    return on
      ? 'Available: the switch will turn it on.'
      : 'Off in this layout. To include it, turn it on in Windows Settings, then Save current layout again.';
  }
  return on
    ? `${label} is Absent: press its input button or plug it in, then switch again.`
    : `${label} is Absent. It is off in this layout, so a switch does not need it.`;
}

/** The latest probe's entry for a monitor, or null when the probe did not list it or has not run. */
export function liveMonitor(inventory: Inventory | null, devicePath: string): Monitor | null {
  return inventory?.monitors.find((m) => m.devicePath === devicePath) ?? null;
}

/**
 * A monitor's state right now. A monitor the latest probe did not list is Absent;
 * with no probe yet there is nothing to say.
 */
export function liveState(inventory: Inventory | null, devicePath: string): MonitorState | null {
  if (!inventory) {
    return null;
  }
  return liveMonitor(inventory, devicePath)?.state ?? 'Absent';
}

/** The tooltip that carries the code behind an input source name (DESIGN.md, Copy voice). */
export function inputSourceTooltip(code: number): string {
  return `Input source code 0x${code.toString(16).toUpperCase().padStart(2, '0')}`;
}
