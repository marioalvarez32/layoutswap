import type { Capabilities, InputSource, Layout, LayoutEdits, Step, StepSide, SummaryMonitor, WaitRule } from '@/domain/generated/types';
import { monitorDisplay, chipLabels } from './monitors';

/** The bounds a wait step keeps, the same as the Rust side enforces on save. */
export const WAIT_SECONDS_MIN = 1;
export const WAIT_SECONDS_MAX = 600;
export const DEFAULT_WAIT_SECONDS = 3;
/** The drop wait's bounds, in seconds. */
export const DROP_WAIT_SECONDS_MAX = 60;
/** The Available wait's bounds, in seconds. */
export const AVAILABLE_WAIT_SECONDS_MAX = 600;
/** The largest VCP code 0x60 value a monitor can hold. */
export const INPUT_SOURCE_MAX = 0xff;
/** The label for a step's monitor the layout does not know. */
export const UNKNOWN_MONITOR = 'unknown monitor';

/** The edits of a layout that has none: the same defaults the migration writes. */
export const DEFAULT_EDITS: LayoutEdits = { steps: [], dropWaitSeconds: 5, availableWaitSeconds: 120, onApplyFailure: 'stop' };

/** Names a monitor by device path: the layout's label, or "unknown monitor". */
export type LabelOf = (devicePath: string) => string;

/** "HDMI 1" for a known code, "Input 0x1E" for any other, as the Rust side names them. */
export function inputSourceName(table: readonly InputSource[], code: number): string {
  return table.find((s) => s.code === code)?.name ?? `Input 0x${code.toString(16).toUpperCase().padStart(2, '0')}`;
}

/** The input select's two groups when the monitor's capabilities are known. */
export interface InputSourceGroups {
  /** The inputs the monitor declares, in the table's order, then any code the table lacks. */
  accepted: InputSource[];
  /** The rest of the table, offered as not declared: a monitor may accept them anyway. */
  others: InputSource[];
}

/**
 * Orders the input table for a monitor from its capabilities: accepted inputs first,
 * the rest after. Null without an entry, one that did not answer, or one that
 * declares no input at all, so the select stays the plain table.
 */
export function inputSourceGroups(table: readonly InputSource[], capabilities: Capabilities | null): InputSourceGroups | null {
  if (!capabilities?.answered || capabilities.inputCodes.length === 0) {
    return null;
  }
  const declared = new Set(capabilities.inputCodes);
  const accepted = table.filter((s) => declared.has(s.code));
  for (const code of capabilities.inputCodes) {
    if (!table.some((s) => s.code === code)) {
      accepted.push({ code, name: inputSourceName(table, code) });
    }
  }
  return { accepted, others: table.filter((s) => !declared.has(s.code)) };
}

/**
 * The step as one sentence, the same one the script prints as its row: "Wait 3
 * seconds", "Send HDMI 1 to Ultrawide, then wait until Ultrawide drops".
 */
export function stepSentence(step: Step, labelOf: LabelOf, table: readonly InputSource[]): string {
  switch (step.kind) {
    case 'wait':
      return `Wait ${step.seconds} ${step.seconds === 1 ? 'second' : 'seconds'}`;
    case 'sendInput': {
      const monitor = labelOf(step.devicePath);
      const tail = step.wait === 'drop'
        ? `, then wait until ${monitor} shows ${inputSourceName(table, step.inputSource)} or drops`
        : step.wait === 'available'
          ? `, then wait until ${monitor} is Available`
          : '';
      return `Send ${inputSourceName(table, step.inputSource)} to ${monitor}${tail}`;
    }
  }
}

/** The label rule for a step's monitor, from the layout's monitors and the aliases. */
export function stepLabeller(aliases: Readonly<Record<string, string>>, monitors: readonly SummaryMonitor[]): LabelOf {
  const displays = monitors.map((m) => monitorDisplay(aliases, m));
  const labels = chipLabels(displays);
  return (devicePath) => {
    const index = monitors.findIndex((m) => m.devicePath === devicePath);
    return index === -1 ? UNKNOWN_MONITOR : (labels[index] ?? UNKNOWN_MONITOR);
  };
}

/** A fresh wait step on the given side, under the id the caller made for it. */
export function newWaitStep(side: StepSide, id: string): Step {
  return { id, side, kind: 'wait', seconds: DEFAULT_WAIT_SECONDS };
}

/** A fresh send step: the first monitor of the layout and the first input of the table. */
export function newSendStep(side: StepSide, id: string, devicePath: string, inputSource: number): Step {
  return { id, side, kind: 'sendInput', devicePath, inputSource, wait: 'none' };
}

/**
 * The same step under the other kind, keeping its id and side. The fields of the new
 * kind start at their defaults; a step already of that kind is returned as it is.
 */
export function withKind(step: Step, kind: Step['kind'], defaults: { devicePath: string; inputSource: number }): Step {
  if (step.kind === kind) {
    return step;
  }
  return kind === 'wait'
    ? newWaitStep(step.side, step.id)
    : newSendStep(step.side, step.id, defaults.devicePath, defaults.inputSource);
}

/**
 * What the editor says under a send step: the monitor is off after the apply, so the
 * step will be skipped; or the layout does not know the monitor at all.
 */
export function sendStepNote(step: Step, layout: Pick<Layout, 'summary'>, labelOf: LabelOf): string | null {
  if (step.kind !== 'sendInput') {
    return null;
  }
  const monitor = layout.summary.monitors.find((m) => m.devicePath === step.devicePath);
  if (!monitor) {
    return 'This monitor is not in the layout any more, so the step will be skipped.';
  }
  if (step.side === 'after' && !monitor.on) {
    return `${labelOf(step.devicePath)} is off after the apply, so this step will be skipped. Move it before the apply.`;
  }
  return null;
}

/** The wait a rule takes, as the row shows beside it: "up to 5 s", or nothing. */
export function waitRuleHint(wait: WaitRule, dropWaitSeconds: number, availableWaitSeconds: number): string {
  switch (wait) {
    case 'drop':
      return `up to ${dropWaitSeconds} s`;
    case 'available':
      return `up to ${availableWaitSeconds} s`;
    case 'none':
      return '';
  }
}

/** The steps on one side of the apply, in order. */
export function stepsOn(steps: readonly Step[], side: StepSide): Step[] {
  return steps.filter((s) => s.side === side);
}

/** Appends a step after the last one on its side, so it lands where the Add button sits. */
export function addStep(steps: readonly Step[], step: Step): Step[] {
  const last = steps.map((s) => s.side).lastIndexOf(step.side);
  const at = last === -1 ? (step.side === 'before' ? 0 : steps.length) : last + 1;
  return [...steps.slice(0, at), step, ...steps.slice(at)];
}

export function removeStep(steps: readonly Step[], id: string): Step[] {
  return steps.filter((s) => s.id !== id);
}

/** Replaces the step with `id` by `next`. */
export function replaceStep(steps: readonly Step[], id: string, next: Step): Step[] {
  return steps.map((s) => (s.id === id ? next : s));
}

/**
 * Moves a step one row up or down the timeline. Among the steps on its side it swaps
 * with its neighbour; at the edge of its side it crosses the apply, so the last step
 * before becomes the first step after and the other way round. A step at the very
 * top or bottom stays.
 */
export function moveStep(steps: readonly Step[], id: string, direction: 'up' | 'down'): Step[] {
  const index = steps.findIndex((s) => s.id === id);
  if (index === -1) {
    return [...steps];
  }
  const step = steps[index]!;
  const side = step.side;
  const neighbour = direction === 'up'
    ? steps.slice(0, index).map((s, i) => [s, i] as const).reverse().find(([s]) => s.side === side)
    : steps.slice(index + 1).map((s, i) => [s, index + 1 + i] as const).find(([s]) => s.side === side);
  if (neighbour) {
    const next = [...steps];
    const [, j] = neighbour;
    [next[index], next[j]] = [next[j]!, next[index]!];
    return next;
  }
  // At the edge of its side: cross the apply, keeping the order on both sides.
  const crossesUp = direction === 'up' && side === 'after';
  const crossesDown = direction === 'down' && side === 'before';
  if (!crossesUp && !crossesDown) {
    return [...steps];
  }
  const moved: Step = { ...step, side: crossesUp ? 'before' : 'after' };
  const rest = steps.filter((s) => s.id !== id);
  const before = rest.filter((s) => s.side === 'before');
  const after = rest.filter((s) => s.side === 'after');
  return [...before, moved, ...after];
}

/** Whether a step can move in a direction: false only at the top or bottom of the timeline. */
export function canMove(steps: readonly Step[], id: string, direction: 'up' | 'down'): boolean {
  const next = moveStep(steps, id, direction);
  return next.some((s, i) => s.id !== steps[i]?.id || s.side !== steps[i]?.side);
}

/** The editable part of a layout, as the editor starts from it. */
export function editsOf(layout: Layout): LayoutEdits {
  return {
    steps: layout.steps.map((s) => ({ ...s })),
    dropWaitSeconds: layout.dropWaitSeconds,
    availableWaitSeconds: layout.availableWaitSeconds,
    onApplyFailure: layout.onApplyFailure,
  };
}

/** Whether saving `edits` would change the layout. */
export function isDirty(edits: LayoutEdits, layout: Layout): boolean {
  return JSON.stringify(edits) !== JSON.stringify(editsOf(layout));
}

/** The rule a seconds field shows while typing; null when the value is fine. */
export function checkWaitSeconds(value: number): string | null {
  if (!Number.isInteger(value) || value < WAIT_SECONDS_MIN || value > WAIT_SECONDS_MAX) {
    return `Keep a wait step between ${WAIT_SECONDS_MIN} and ${WAIT_SECONDS_MAX} seconds.`;
  }
  return null;
}

/** The rule the drop wait field shows while typing; null when the value is fine. */
export function checkDropWait(value: number): string | null {
  if (!Number.isInteger(value) || value < 0 || value > DROP_WAIT_SECONDS_MAX) {
    return `Keep the drop wait between 0 and ${DROP_WAIT_SECONDS_MAX} seconds.`;
  }
  return null;
}

/** The rule the Available wait field shows while typing; null when the value is fine. */
export function checkAvailableWait(value: number): string | null {
  if (!Number.isInteger(value) || value < 1 || value > AVAILABLE_WAIT_SECONDS_MAX) {
    return `Keep the Available wait between 1 and ${AVAILABLE_WAIT_SECONDS_MAX} seconds.`;
  }
  return null;
}

/** The rule the "Other code" field shows; null when the value is a code a monitor can hold. */
export function checkInputCode(value: number): string | null {
  if (!Number.isInteger(value) || value < 1 || value > INPUT_SOURCE_MAX) {
    return 'Keep the input source code between 0x01 and 0xFF.';
  }
  return null;
}

/** "0x1E" for a code, as the "Other code" field shows it. */
export function formatInputCode(code: number): string {
  return `0x${code.toString(16).toUpperCase().padStart(2, '0')}`;
}

/** The code a typed value means: "0x1E", "1E" and "30" all read as 30; NaN when none. */
export function parseInputCode(text: string): number {
  const trimmed = text.trim();
  if (/^0x[0-9a-f]+$/i.test(trimmed)) {
    return Number.parseInt(trimmed.slice(2), 16);
  }
  if (/^[0-9a-f]+$/i.test(trimmed) && /[a-f]/i.test(trimmed)) {
    return Number.parseInt(trimmed, 16);
  }
  if (/^[0-9]+$/.test(trimmed)) {
    return Number.parseInt(trimmed, 10);
  }
  return Number.NaN;
}
