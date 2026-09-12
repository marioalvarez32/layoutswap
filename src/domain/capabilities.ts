import type { Capabilities, InputSource, Layout, Mode, Monitor } from '@/domain/generated/types';
import { formatSize } from './monitors';
import { inputSourceGroups } from './steps';

/** VCP 0xD6 value a monitor must accept as a write for the app to wake it. */
const POWER_MODE_AWAKE = 1;

export type Tone = 'good' | 'warn' | 'mute';

/** The Wake line of an expanded row (DESIGN.md, 3a). */
export function wakeText(capabilities: Capabilities | null): { text: string; tone: Tone } {
  if (!capabilities?.answered) {
    return { text: 'Unknown', tone: 'mute' };
  }
  return capabilities.powerModes.includes(POWER_MODE_AWAKE)
    ? { text: 'Can be woken by the app', tone: 'good' }
    : { text: 'Needs a button press to wake', tone: 'warn' };
}

/** "3440 x 1440 at 60, 100 Hz", one line per resolution in the stored order (largest first). */
export function modeLines(modes: readonly Mode[]): string[] {
  const lines: { key: string; text: string; rates: number[] }[] = [];
  for (const mode of modes) {
    const key = `${mode.width} x ${mode.height}`;
    const line = lines.find((l) => l.key === key);
    if (line) {
      line.rates.push(mode.hz);
    } else {
      lines.push({ key, text: key, rates: [mode.hz] });
    }
  }
  return lines.map((l) => `${l.text} at ${l.rates.join(', ')} Hz`);
}

/**
 * The accepted inputs by name, in the table's order, an unknown code as hex, each
 * marked when it is the input the monitor shows now.
 */
export function acceptedInputs(
  table: readonly InputSource[],
  capabilities: Capabilities | null,
  currentInput: number | null,
): { name: string; current: boolean }[] {
  return inputSourceGroups(table, capabilities)?.accepted.map((s) => ({ name: s.name, current: s.code === currentInput })) ?? [];
}

/**
 * Why a row shows no capabilities, in the words the design uses, or null when it has
 * them to show. A monitor that is not Active cannot be read at all, whatever is
 * stored for it.
 */
export function unreadReason(monitor: Pick<Monitor, 'state'>, capabilities: Capabilities | null, reading: boolean): string | null {
  if (monitor.state !== 'Active') {
    return 'Switch it on in Windows first';
  }
  if (reading) {
    return 'Reading capabilities';
  }
  if (capabilities === null) {
    return 'Not read yet';
  }
  return capabilities.answered ? null : 'Does not answer over DDC-CI';
}

/** The note on the right of a row: where an Active monitor is used, else the physical fact. */
export function monitorNote(monitor: Pick<Monitor, 'devicePath' | 'state'>, layouts: readonly Layout[]): string {
  switch (monitor.state) {
    case 'Active': {
      const names = layouts
        .filter((l) => l.summary.monitors.some((m) => m.devicePath === monitor.devicePath && m.on))
        .map((l) => l.name);
      return names.length > 0 ? `In ${names.join(', ')}` : 'Not in a layout';
    }
    case 'Available':
      return 'Plugged in, Windows is not drawing to it';
    case 'Absent':
      return 'Press its input button, then Refresh';
  }
}

/** "last seen 1920×1080 · 60 Hz" from the layout that last recorded the monitor on, else "". */
export function lastSeenSize(layouts: readonly Layout[], devicePath: string): string {
  const seen = layouts
    .flatMap((l) => l.summary.monitors.filter((m) => m.devicePath === devicePath && m.on && m.size).map((m) => ({ at: l.updatedAt, m })))
    .sort((a, b) => (a.at < b.at ? 1 : a.at > b.at ? -1 : 0))[0];
  return seen ? `last seen ${formatSize(seen.m.size, seen.m.refreshHz)}` : '';
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** "Read on 11 Sep 2026, 11:52". */
export function formatReadAt(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return 'Read at an unknown time';
  }
  const time = `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`;
  return `Read on ${date.getDate()} ${MONTHS[date.getMonth()]} ${date.getFullYear()}, ${time}`;
}
