import { describe, expect, it } from 'vitest';
import type { ProgressLine, SwitchResult } from '@/domain/generated/types';
import {
  appendLog,
  applyProgress,
  canCancel,
  failureBand,
  formatSeconds,
  newSwitchRun,
  stepTone,
  switchHeadline,
  type SwitchRun,
} from './switch';

const STEPS = ['Check monitors', 'Apply arrangement', 'Verify'];

function run(overrides: Partial<SwitchRun> = {}): SwitchRun {
  return { ...newSwitchRun({ id: 'layout-desk', name: 'Desk' }, STEPS, 2, 1_000), ...overrides };
}

function line(step: number, status: ProgressLine['status'], text: string): ProgressLine {
  return { step, of: 3, status, text };
}

describe('newSwitchRun', () => {
  it('lists every step as waiting under its name', () => {
    const r = run();
    expect(r.steps.map((s) => [s.step, s.name, s.status])).toEqual([
      [1, 'Check monitors', 'waiting'],
      [2, 'Apply arrangement', 'waiting'],
      [3, 'Verify', 'waiting'],
    ]);
    expect(r.log).toEqual([]);
    expect(r.result).toBeNull();
  });
});

describe('applyProgress', () => {
  it('moves the reported step through its statuses and keeps the failure text', () => {
    let steps = run().steps;
    steps = applyProgress(steps, line(1, 'running', 'Check monitors'));
    expect(steps[0]).toMatchObject({ status: 'running', text: 'Check monitors' });
    steps = applyProgress(steps, line(1, 'failed', 'Check monitors: press the input button on Ultrawide; 1 Absent'));
    expect(steps[0]).toMatchObject({ status: 'failed', text: 'Check monitors: press the input button on Ultrawide; 1 Absent' });
    expect(steps[1]!.status).toBe('waiting');
  });

  it('ignores a step the started event did not list', () => {
    const steps = applyProgress(run().steps, { step: 4, of: 4, status: 'running', text: 'Refresh RDP profiles' });
    expect(steps).toEqual(run().steps);
  });

  it('returns a new array', () => {
    const before = run().steps;
    const after = applyProgress(before, line(1, 'done', 'Check monitors'));
    expect(after).not.toBe(before);
    expect(before[0]!.status).toBe('waiting');
  });
});

describe('appendLog', () => {
  it('keeps the last five non-empty lines, oldest first', () => {
    let log: string[] = [];
    for (const text of ['', 'a', 'b', '   ', 'c', 'd', 'e', 'f']) {
      log = appendLog(log, text);
    }
    expect(log).toEqual(['b', 'c', 'd', 'e', 'f']);
  });
});

describe('canCancel', () => {
  it('is allowed until the apply step reports', () => {
    const r = run();
    expect(canCancel(r)).toBe(true);
    r.steps = applyProgress(r.steps, line(1, 'done', 'Check monitors'));
    expect(canCancel(r)).toBe(true);
    r.steps = applyProgress(r.steps, line(2, 'running', 'Apply arrangement'));
    expect(canCancel(r)).toBe(false);
  });

  it('is off once a cancel is in flight or the script has exited', () => {
    expect(canCancel(run({ cancelling: true }))).toBe(false);
    expect(canCancel(run({ result: { outcome: 'cancelled' } }))).toBe(false);
  });
});

describe('stepTone', () => {
  it('maps each chip to a tone', () => {
    expect(stepTone('waiting')).toBe('mute');
    expect(stepTone('running')).toBe('accent');
    expect(stepTone('done')).toBe('good');
    expect(stepTone('failed')).toBe('crit');
    expect(stepTone('skipped')).toBe('mute');
  });
});

describe('formatSeconds', () => {
  it('shows one decimal and never goes negative', () => {
    expect(formatSeconds(11_400)).toBe('11.4 s');
    expect(formatSeconds(0)).toBe('0.0 s');
    expect(formatSeconds(-5)).toBe('0.0 s');
    expect(formatSeconds(999)).toBe('1.0 s');
  });
});

describe('switchHeadline', () => {
  it('reads Switching with the elapsed time while the script runs', () => {
    expect(switchHeadline(run(), 7_100)).toEqual({ title: 'Switching to Desk', aside: '7.1 s elapsed', tone: 'accent' });
  });

  it('reads Applied in N s with the duration the app measured', () => {
    const r = run({ result: { outcome: 'applied', durationMs: 11_400 }, finishedAt: 20_000 });
    expect(switchHeadline(r, 99_000)).toEqual({ title: 'Switched to Desk', aside: 'Applied in 11.4 s', tone: 'good' });
  });

  it('says how long a failed or cancelled switch ran', () => {
    const failed: SwitchResult = { outcome: 'failed', step: 1, stepName: 'Check monitors', nextAction: 'x', reason: 'y', exitCode: 2, logPath: 'C:/x/switch.log' };
    expect(switchHeadline(run({ result: failed, finishedAt: 4_200 }), 0)).toEqual({ title: 'Could not switch to Desk', aside: 'Stopped after 3.2 s', tone: 'crit' });
    expect(switchHeadline(run({ result: { outcome: 'cancelled' }, finishedAt: 2_000 }), 0)).toEqual({ title: 'Switch to Desk cancelled', aside: 'Stopped after 1.0 s', tone: 'mute' });
  });
});

describe('failureBand', () => {
  it('puts the next action first as a sentence, then the step and reason', () => {
    const band = failureBand({
      outcome: 'failed',
      step: 1,
      stepName: 'Check monitors',
      nextAction: 'press the input button on Ultrawide, or plug it in, then switch again',
      reason: '1 Absent',
      exitCode: 2,
      logPath: 'C:/x/switch.log',
    });
    expect(band).toEqual({
      action: 'Press the input button on Ultrawide, or plug it in, then switch again.',
      detail: 'Check monitors failed: 1 Absent (exit code 2).',
    });
  });

  it('explains a script that stopped outside a step', () => {
    const band = failureBand({ outcome: 'failed', step: null, stepName: '', nextAction: 'Wait for the switch to Film to finish, then try again.', reason: 'the script exited with exit code 2', exitCode: 2, logPath: '' });
    expect(band).toEqual({
      action: 'Wait for the switch to Film to finish, then try again.',
      detail: 'The script exited with exit code 2.',
    });
  });

  it('is empty for an applied or cancelled result', () => {
    expect(failureBand({ outcome: 'applied', durationMs: 1 })).toBeNull();
    expect(failureBand({ outcome: 'cancelled' })).toBeNull();
  });
});
