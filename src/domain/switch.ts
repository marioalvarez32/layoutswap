import type { LineParts, ProgressLine, StepStatus, SwitchResult, VerifyFailure } from '@/domain/generated/types';

/** A step's chip: the script's statuses, plus waiting before it reports. */
export type StepChip = 'waiting' | StepStatus;

export interface SwitchStep {
  /** One-based, as the script reports it. */
  step: number;
  name: string;
  status: StepChip;
  /** The step name, or the failure text once it has failed. */
  text: string;
  /** The split text of a failed or needs-you line, as the parser gave it. */
  parts: LineParts | null;
}

/** One switch as the progress screen shows it, from the started event to the result. */
export interface SwitchRun {
  layoutId: string;
  layoutName: string;
  /** Epoch milliseconds when the script started. */
  startedAt: number;
  /** The one-based step from which Cancel is disabled. */
  applyStep: number;
  steps: SwitchStep[];
  /** The last few log lines, oldest first. */
  log: string[];
  result: SwitchResult | null;
  finishedAt: number | null;
  /** A cancel has been sent and the script has not exited yet. */
  cancelling: boolean;
  /** The last action on this screen the app refused, shown beside the buttons. */
  error: string | null;
  /** The last thing an action on this screen did, such as where diagnostics went. */
  notice: string | null;
}

export const LOG_TAIL_LINES = 5;

export function newSwitchRun(
  layout: { id: string; name: string },
  steps: readonly string[],
  applyStep: number,
  startedAt: number,
): SwitchRun {
  return {
    layoutId: layout.id,
    layoutName: layout.name,
    startedAt,
    applyStep,
    steps: steps.map((name, i) => ({ step: i + 1, name, status: 'waiting', text: name, parts: null })),
    log: [],
    result: null,
    finishedAt: null,
    cancelling: false,
    error: null,
    notice: null,
  };
}

/**
 * The steps after one progress line: the reported step takes the line's status, its
 * parts, and its text, except that a needs-you row keeps its name as text since the
 * band carries the details. A line for a step the started event did not list changes
 * nothing.
 */
export function applyProgress(steps: readonly SwitchStep[], line: ProgressLine): SwitchStep[] {
  return steps.map((s) => (s.step === line.step
    ? { ...s, status: line.status, text: line.status === 'needsYou' ? s.name : line.text, parts: line.parts }
    : s));
}

/** The log tail with `text` appended: the last few non-empty lines. */
export function appendLog(log: readonly string[], text: string): string[] {
  if (text.trim() === '') {
    return [...log];
  }
  return [...log, text].slice(-LOG_TAIL_LINES);
}

/** Cancel is available until the apply step reports, and only while the script runs. */
export function canCancel(run: SwitchRun): boolean {
  if (run.result !== null || run.cancelling) {
    return false;
  }
  return run.steps.every((s) => s.step < run.applyStep || s.status === 'waiting');
}

export type ChipTone = 'accent' | 'mute' | 'good' | 'warn' | 'crit';

export function stepTone(status: StepChip): ChipTone {
  switch (status) {
    case 'running':
      return 'accent';
    case 'done':
      return 'good';
    case 'failed':
      return 'crit';
    case 'needsYou':
      return 'warn';
    default:
      return 'mute';
  }
}

/** The chip's words: the status as the design names it. */
export function stepChipLabel(status: StepChip): string {
  return status === 'needsYou' ? 'needs you' : status;
}

export interface NeedsYouBand {
  /** The physical action, as a sentence. */
  action: string;
  /** What the script waits for and the seconds left: "Waiting until X is Available · 92 s left of 120 s". */
  waiting: string;
}

/**
 * The band shown while a row needs you, from the parts the parser split off the
 * row's latest needs-you line. Null when no row needs you.
 */
export function needsYouBand(run: SwitchRun): NeedsYouBand | null {
  if (run.result !== null) {
    return null;
  }
  const row = run.steps.find((s) => s.status === 'needsYou');
  if (!row?.parts) {
    return null;
  }
  return {
    action: sentence(row.parts.action),
    waiting: row.parts.detail ? `${row.parts.name} · ${row.parts.detail}` : row.parts.name,
  };
}

/** "11.4 s": one decimal, the data-font duration everywhere a switch is timed. */
export function formatSeconds(ms: number): string {
  return `${(Math.max(0, ms) / 1000).toFixed(1)} s`;
}

export interface SwitchHeadline {
  title: string;
  /** The timing line beside the title. */
  aside: string;
  tone: 'accent' | 'good' | 'crit' | 'mute';
}

/** The title and timing of the progress screen in each of its states. */
export function switchHeadline(run: SwitchRun, elapsedMs: number): SwitchHeadline {
  const name = run.layoutName;
  const stopped = `Stopped after ${formatSeconds((run.finishedAt ?? run.startedAt) - run.startedAt)}`;
  switch (run.result?.outcome) {
    case undefined:
      return { title: `Switching to ${name}`, aside: `${formatSeconds(elapsedMs)} elapsed`, tone: 'accent' };
    case 'applied':
      return { title: `Switched to ${name}`, aside: `Applied in ${formatSeconds(run.result.durationMs)}`, tone: 'good' };
    case 'failed':
      return { title: `Could not switch to ${name}`, aside: stopped, tone: 'crit' };
    case 'cancelled':
      return { title: `Switch to ${name} cancelled`, aside: stopped, tone: 'mute' };
  }
}

/** The same line the script and the Rust side print for a verify failure. */
export function describeVerifyFailure(failure: VerifyFailure): string {
  switch (failure.kind) {
    case 'notOn':
      return `${failure.label} is not on`;
    case 'onButShouldBeOff':
      return `${failure.label} is on but should be off`;
    case 'misplaced':
      return `${failure.label} landed at ${failure.actual.x},${failure.actual.y} instead of ${failure.expected.x},${failure.expected.y}`;
    case 'wrongSize':
      return `${failure.label} is ${failure.actual.width}x${failure.actual.height} instead of ${failure.expected.width}x${failure.expected.height}`;
    case 'extra':
      return `an extra monitor is on: ${failure.name}`;
  }
}

/** The line a cancelled result shows when a send step had already run. */
export function cancelledNote(run: SwitchRun): string | null {
  const result = run.result;
  if (result?.outcome !== 'cancelled' || result.sent.length === 0) {
    return null;
  }
  return `Sent before the cancel: ${result.sent.join('; ')}. The monitor may be showing another device now.`;
}

export interface FailureBand {
  /** The next action, as a sentence. */
  action: string;
  /** What went wrong, with the step it happened in. */
  detail: string;
  /** Whether the fix lives in Windows Settings > Display. */
  offerDisplaySettings: boolean;
  /** Refresh, rotation and scale differences verify noted without failing. */
  warnings: string[];
}

/**
 * The band a failed result shows: what to do next first, then what went wrong.
 * A verify failure leads with Settings > Display and says that layoutswap does not
 * move monitors; a check failure leads with the physical action the script named.
 */
export function failureBand(run: SwitchRun): FailureBand | null {
  const result = run.result;
  if (result?.outcome !== 'failed') {
    return null;
  }
  const exit = `(exit code ${result.exitCode})`;
  if (result.explanation.kind === 'verify') {
    // The probe's own reading when it has one, else the script's line.
    const landed = result.explanation.failures.length > 0
      ? result.explanation.failures.map(describeVerifyFailure).join('; ')
      : result.reason || 'the arrangement is not what the layout says';
    return {
      action: 'Arrange it in Settings > Display, then Save current layout again.',
      detail: `Switch to ${run.layoutName} applied, but ${landed}. layoutswap does not move monitors.`,
      offerDisplaySettings: true,
      warnings: result.explanation.warnings,
    };
  }
  const action = sentence(result.nextAction || 'Open the log, then try the switch again');
  if (result.explanation.kind === 'absent' && result.explanation.monitors.length > 0) {
    return {
      action,
      detail: `Absent: ${result.explanation.monitors.join(', ')}. ${result.stepName} stopped the switch ${exit}.`,
      offerDisplaySettings: false,
      warnings: [],
    };
  }
  if (!result.stepName) {
    return {
      action,
      detail: sentence(result.reason || `the script exited with exit code ${result.exitCode}`),
      offerDisplaySettings: false,
      warnings: [],
    };
  }
  const reason = result.reason ? `: ${result.reason}` : '';
  return { action, detail: `${result.stepName} failed${reason} ${exit}.`, offerDisplaySettings: false, warnings: [] };
}

function sentence(text: string): string {
  const trimmed = text.trim();
  const capitalised = trimmed.charAt(0).toUpperCase() + trimmed.slice(1);
  return /[.!?]$/.test(capitalised) ? capitalised : `${capitalised}.`;
}
