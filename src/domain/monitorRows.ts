import type { Capabilities, InputSource, Inventory, Layout, MonitorState } from '@/domain/generated/types';
import { acceptedInputs, formatReadAt, lastSeenSize, modeLines, monitorNote, unreadReason, wakeText } from './capabilities';
import { formatPosition, formatSize, monitorDisplay, monitorStateNote, stateLabel } from './monitors';

/** One row of the Monitors screen, everything the row shows already in words (DESIGN.md, 3a and 3b). */
export interface MonitorRowView {
  devicePath: string;
  alias: string;
  reportedName: string;
  stateText: string;
  tone: 'good' | 'warn' | 'crit';
  stateNote: string;
  note: string;
  gpu: string;
  connector: string;
  position: string;
  size: string;
  /** The current input source by name, or "unknown". */
  input: string;
  inputKnown: boolean;
  /** The row has capabilities to show; else `muted` says why not. */
  canExpand: boolean;
  expanded: boolean;
  muted: string | null;
  reading: boolean;
  accepts: { name: string; current: boolean }[];
  wake: { text: string; tone: 'good' | 'warn' | 'mute' };
  modes: string[];
  readAt: string;
}

export interface MonitorRowsContext {
  inventory: Inventory | null;
  layouts: readonly Layout[];
  aliases: Readonly<Record<string, string>>;
  inputSources: readonly InputSource[];
  capabilities: Readonly<Record<string, Capabilities>>;
  /** A capabilities read is running. */
  reading: boolean;
  /** The device paths whose capabilities are open. */
  expanded: ReadonlySet<string>;
}

const TONES: Record<MonitorState, 'good' | 'warn' | 'crit'> = { Active: 'good', Available: 'warn', Absent: 'crit' };

/** The rows of the Monitors screen from the latest probe and what the app keeps about each monitor. */
export function monitorRows(ctx: MonitorRowsContext): MonitorRowView[] {
  return (ctx.inventory?.monitors ?? []).map((m) => {
    const entry = ctx.capabilities[m.devicePath] ?? null;
    const muted = unreadReason(m, entry, ctx.reading);
    return {
      devicePath: m.devicePath,
      alias: monitorDisplay(ctx.aliases, m).label,
      reportedName: m.reportedName,
      stateText: stateLabel(m.state, m.asleep),
      tone: TONES[m.state],
      stateNote: monitorStateNote(m.state, m.asleep),
      note: monitorNote(m, ctx.layouts),
      gpu: m.gpu || 'unknown',
      connector: m.connector,
      position: m.position ? formatPosition(m.position) : 'not Active',
      size: m.size ? formatSize(m.size, m.refreshHz) : lastSeenSize(ctx.layouts, m.devicePath) || 'not Active',
      input: m.inputSourceName ?? 'unknown',
      inputKnown: m.inputSourceName !== null,
      canExpand: muted === null,
      expanded: ctx.expanded.has(m.devicePath),
      muted,
      reading: muted === 'Reading capabilities',
      accepts: acceptedInputs(ctx.inputSources, entry, m.inputSource),
      wake: wakeText(entry),
      modes: modeLines(entry?.modes ?? []),
      readAt: entry ? formatReadAt(entry.readAt) : '',
    };
  });
}
