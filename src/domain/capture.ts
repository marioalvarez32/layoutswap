import type { Inventory, Monitor } from '@/domain/generated/types';
import { formatPosition, formatSize, monitorDisplay, type MonitorDisplay } from './monitors';
import type { SchematicMonitor } from './schematic';

/** One row of "What will be captured" on the Save current layout page. */
export interface CapturePreviewRow extends MonitorDisplay {
  devicePath: string;
  /** Active monitors are captured on, Available ones off. */
  on: boolean;
  size: string;
  position: string;
  primary: boolean;
}

/**
 * What a capture of this probe records: every connected monitor, so Absent ones are
 * left out, with on for Active and off for Available. The same rule as the summary
 * Rust derives, so the preview and the picture show what will be stored.
 */
export function capturePreviewMonitors(inventory: Inventory | null): (SchematicMonitor & Pick<Monitor, 'refreshHz'>)[] {
  if (!inventory) {
    return [];
  }
  return inventory.monitors
    .filter((m) => m.state !== 'Absent')
    .map((m) => {
      const on = m.state === 'Active';
      return {
        devicePath: m.devicePath,
        reportedName: m.reportedName,
        connector: m.connector,
        on,
        position: on ? m.position : null,
        size: on ? m.size : null,
        refreshHz: on ? m.refreshHz : null,
        primary: on && m.primary,
      };
    });
}

/** The rows of the capture preview table, formatted. */
export function capturePreviewRows(inventory: Inventory | null, aliases: Readonly<Record<string, string>>): CapturePreviewRow[] {
  return capturePreviewMonitors(inventory).map((m) => ({
    devicePath: m.devicePath,
    on: m.on,
    ...monitorDisplay(aliases, m),
    size: formatSize(m.size, m.refreshHz),
    position: formatPosition(m.position),
    primary: m.primary,
  }));
}
