import type { ProgressLine, StepStatus, SwitchResult } from '@/domain/generated/types';

/** A step's chip: the script's four statuses, plus waiting before it reports. */
export type StepChip = 'waiting' | StepStatus;

export interface SwitchStep {
  /** One-based, as the script reports it. */
  step: number;
  name: string;
  status: StepChip;
  /** The step name, or the failure text once it has failed. */
  text: string;
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
  /** A cancel the app refused, shown beside the button. */
  error: string | null;
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
    steps: steps.map((name, i) => ({ step: i + 1, name, status: 'waiting', text: name })),
    log: [],
    result: null,
    finishedAt: null,
    cancelling: false,
    error: null,
  };
}

/**
 * The steps after one progress line: the reported step takes the line's status and
 * text. A line for a step the started event did not list changes nothing.
 */
export function applyProgress(steps: readonly SwitchStep[], line: ProgressLine): SwitchStep[] {
  return steps.map((s) => (s.step === line.step ? { ...s, status: line.status, text: line.text } : s));
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
    default:
      return 'mute';
  }
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

export interface FailureBand {
  /** The next action, as a sentence. */
  action: string;
  /** What went wrong, with the step it happened in. */
  detail: string;
}

/** The band a failed result shows: what to do next first, then what went wrong. */
export function failureBand(result: SwitchResult): FailureBand | null {
  if (result.outcome !== 'failed') {
    return null;
  }
  const action = sentence(result.nextAction || 'Open the log, then try the switch again');
  if (!result.stepName) {
    return { action, detail: sentence(result.reason || `the script exited with exit code ${result.exitCode}`) };
  }
  const reason = result.reason ? `: ${result.reason}` : '';
  return { action, detail: `${result.stepName} failed${reason} (exit code ${result.exitCode}).` };
}

function sentence(text: string): string {
  const trimmed = text.trim();
  const capitalised = trimmed.charAt(0).toUpperCase() + trimmed.slice(1);
  return /[.!?]$/.test(capitalised) ? capitalised : `${capitalised}.`;
}
