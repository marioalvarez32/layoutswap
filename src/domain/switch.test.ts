import { describe, expect, it } from 'vitest';
import type { ProgressLine, SwitchResult } from '@/domain/generated/types';
import {
  appendLog,
  applyProgress,
  canCancel,
  cancelledNote,
  needsYouBand,
  stepChipLabel,
  describeVerifyFailure,
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

function line(step: number, status: ProgressLine['status'], text: string, parts: ProgressLine['parts'] = null): ProgressLine {
  return { step, of: 3, status, text, parts };
}

function failed(overrides: Partial<Extract<SwitchResult, { outcome: 'failed' }>> = {}): SwitchResult {
  return {
    outcome: 'failed',
    step: 1,
    stepName: 'Check monitors',
    nextAction: 'x',
    reason: 'y',
    exitCode: 2,
    logPath: 'C:/x/switch.log',
    explanation: { kind: 'none' },
    ...overrides,
  };
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
    expect(r.notice).toBeNull();
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
    const steps = applyProgress(run().steps, { step: 4, of: 4, status: 'running', text: 'Refresh RDP profiles', parts: null });
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
    expect(canCancel(run({ result: { outcome: 'cancelled', sent: [] } }))).toBe(false);
  });
});

describe('stepTone', () => {
  it('maps each chip to a tone', () => {
    expect(stepTone('waiting')).toBe('mute');
    expect(stepTone('running')).toBe('accent');
    expect(stepTone('done')).toBe('good');
    expect(stepTone('failed')).toBe('crit');
    expect(stepTone('skipped')).toBe('mute');
    expect(stepTone('needsYou')).toBe('warn');
  });
});

describe('stepChipLabel', () => {
  it('names the needs-you chip in words', () => {
    expect(stepChipLabel('needsYou')).toBe('needs you');
    expect(stepChipLabel('done')).toBe('done');
  });
});

describe('needsYouBand', () => {
  it('reads the action and the countdown from the row that needs you', () => {
    const r = run();
    const parts = { name: 'Waiting until Ultrawide is Available', action: 'press the input button on Ultrawide, or turn the other device off', detail: '92 s left of 120 s' };
    r.steps = applyProgress(r.steps, line(1, 'needsYou', 'Waiting until Ultrawide is Available: press the input button on Ultrawide, or turn the other device off; 92 s left of 120 s', parts));
    expect(r.steps[0]).toMatchObject({ status: 'needsYou', text: 'Check monitors', parts });
    expect(needsYouBand(r)).toEqual({
      action: 'Press the input button on Ultrawide, or turn the other device off.',
      waiting: 'Waiting until Ultrawide is Available · 92 s left of 120 s',
    });
    expect(canCancel(r)).toBe(true);
    r.steps = applyProgress(r.steps, line(1, 'done', 'Check monitors'));
    expect(needsYouBand(r)).toBeNull();
    expect(needsYouBand(run({ result: { outcome: 'cancelled', sent: [] } }))).toBeNull();
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
    expect(switchHeadline(run({ result: failed(), finishedAt: 4_200 }), 0)).toEqual({ title: 'Could not switch to Desk', aside: 'Stopped after 3.2 s', tone: 'crit' });
    expect(switchHeadline(run({ result: { outcome: 'cancelled', sent: [] }, finishedAt: 2_000 }), 0)).toEqual({ title: 'Switch to Desk cancelled', aside: 'Stopped after 1.0 s', tone: 'mute' });
  });
});

describe('describeVerifyFailure', () => {
  it('reads like the script log line', () => {
    expect(describeVerifyFailure({ kind: 'misplaced', label: 'Side', actual: { x: 3440, y: 0 }, expected: { x: 3440, y: 180 } })).toBe('Side landed at 3440,0 instead of 3440,180');
    expect(describeVerifyFailure({ kind: 'wrongSize', label: 'Side', actual: { width: 1920, height: 1200 }, expected: { width: 1920, height: 1080 } })).toBe('Side is 1920x1200 instead of 1920x1080');
    expect(describeVerifyFailure({ kind: 'notOn', label: 'Side' })).toBe('Side is not on');
    expect(describeVerifyFailure({ kind: 'onButShouldBeOff', label: 'Ultrawide' })).toBe('Ultrawide is on but should be off');
    expect(describeVerifyFailure({ kind: 'extra', name: 'TV' })).toBe('an extra monitor is on: TV');
  });
});

describe('failureBand', () => {
  it('leads a check failure with the physical action and lists the Absent monitors', () => {
    const band = failureBand(run({
      result: failed({
        nextAction: 'press the input button on Side and Ultrawide, or plug them in, then switch again',
        reason: '2 Absent',
        explanation: { kind: 'absent', monitors: ['Side', 'Ultrawide'] },
      }),
    }));
    expect(band).toEqual({
      action: 'Press the input button on Side and Ultrawide, or plug them in, then switch again.',
      detail: 'Absent: Side, Ultrawide. Check monitors stopped the switch (exit code 2).',
      offerDisplaySettings: false,
      warnings: [],
    });
  });

  it('leads a verify failure with Settings > Display and says where the monitor landed', () => {
    const band = failureBand(run({
      layoutName: 'Console',
      result: failed({
        step: 3,
        stepName: 'Verify',
        nextAction: 'arrange the monitors in Windows Settings > Display, then save the layout again',
        reason: 'Side landed at 3440,0 instead of 3440,180',
        exitCode: 1,
        explanation: {
          kind: 'verify',
          failures: [{ kind: 'misplaced', label: 'Side', actual: { x: 3440, y: 0 }, expected: { x: 3440, y: 180 } }],
          warnings: ['Side runs at 75 Hz instead of 60 Hz'],
        },
      }),
    }));
    expect(band).toEqual({
      action: 'Arrange it in Settings > Display, then Save current layout again.',
      detail: 'Switch to Console applied, but Side landed at 3440,0 instead of 3440,180. layoutswap does not move monitors.',
      offerDisplaySettings: true,
      warnings: ['Side runs at 75 Hz instead of 60 Hz'],
    });
  });

  it('still leads a verify failure with Settings > Display when the probe found nothing, from the script line', () => {
    const band = failureBand(run({
      result: failed({
        step: 3,
        stepName: 'Verify',
        nextAction: 'arrange the monitors in Windows Settings > Display, then save the layout again',
        reason: 'Side landed at 3440,0 instead of 3440,180',
        exitCode: 1,
        explanation: { kind: 'verify', failures: [], warnings: [] },
      }),
    }));
    expect(band).toMatchObject({
      action: 'Arrange it in Settings > Display, then Save current layout again.',
      detail: 'Switch to Desk applied, but Side landed at 3440,0 instead of 3440,180. layoutswap does not move monitors.',
      offerDisplaySettings: true,
    });
  });

  it('falls back to the script line for other steps', () => {
    const band = failureBand(run({
      result: failed({
        step: 2,
        stepName: 'Apply arrangement',
        nextAction: 'try the switch again',
        reason: 'Windows could not apply the arrangement, bad configuration (Windows error 1610)',
        exitCode: 1,
      }),
    }));
    expect(band).toEqual({
      action: 'Try the switch again.',
      detail: 'Apply arrangement failed: Windows could not apply the arrangement, bad configuration (Windows error 1610) (exit code 1).',
      offerDisplaySettings: false,
      warnings: [],
    });
  });

  it('explains a script that stopped outside a step', () => {
    const band = failureBand(run({
      result: failed({ step: null, stepName: '', nextAction: 'Wait for the switch to Film to finish, then try again.', reason: 'the script exited with exit code 2' }),
    }));
    expect(band).toEqual({
      action: 'Wait for the switch to Film to finish, then try again.',
      detail: 'The script exited with exit code 2.',
      offerDisplaySettings: false,
      warnings: [],
    });
  });

  it('names the sends that ran before a cancel', () => {
    expect(cancelledNote(run({ result: { outcome: 'cancelled', sent: [] } }))).toBeNull();
    expect(cancelledNote(run({ result: { outcome: 'cancelled', sent: ['Send HDMI 1 to Ultrawide, then wait until Ultrawide drops'] } }))).toBe('Sent before the cancel: Send HDMI 1 to Ultrawide, then wait until Ultrawide drops. The monitor may be showing another device now.');
    expect(cancelledNote(run())).toBeNull();
  });

  it('is empty while running and for an applied or cancelled result', () => {
    expect(failureBand(run())).toBeNull();
    expect(failureBand(run({ result: { outcome: 'applied', durationMs: 1 } }))).toBeNull();
    expect(failureBand(run({ result: { outcome: 'cancelled', sent: [] } }))).toBeNull();
  });
});
