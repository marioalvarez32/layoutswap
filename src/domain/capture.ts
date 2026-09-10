import type { Inventory } from '@/domain/generated/types';
import { formatPosition, formatSize, monitorDisplay, type MonitorDisplay } from './monitors';

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
 * left out, with the details the summary will keep for on monitors.
 */
export function capturePreviewRows(inventory: Inventory | null, aliases: Readonly<Record<string, string>>): CapturePreviewRow[] {
  if (!inventory) {
    return [];
  }
  return inventory.monitors
    .filter((m) => m.state !== 'Absent')
    .map((m) => {
      const on = m.state === 'Active';
      return {
        devicePath: m.devicePath,
        on,
        ...monitorDisplay(aliases, m),
        size: on ? formatSize(m.size, m.refreshHz) : '',
        position: on ? formatPosition(m.position) : '',
        primary: on && m.primary,
      };
    });
}
