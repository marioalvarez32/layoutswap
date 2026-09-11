import { describe, expect, it } from 'vitest';
import type { InputSource, Step, WaitRule } from '@/domain/generated/types';
import { layoutFixture } from '@/test/fixtures';
import {
  addStep,
  canMove,
  checkAvailableWait,
  checkDropWait,
  checkInputCode,
  checkWaitSeconds,
  editsOf,
  formatInputCode,
  inputSourceName,
  isDirty,
  moveStep,
  newSendStep,
  newWaitStep,
  parseInputCode,
  removeStep,
  replaceStep,
  sendStepNote,
  stepLabeller,
  stepSentence,
  stepsOn,
  waitRuleHint,
  withKind,
} from './steps';

const TABLE: InputSource[] = [
  { code: 0x0f, name: 'DisplayPort 1' },
  { code: 0x11, name: 'HDMI 1' },
];

function wait(id: string, side: Step['side'], seconds: number): Step {
  return { id, side, kind: 'wait', seconds };
}

function send(id: string, side: Step['side'], devicePath: string, inputSource: number, wait: WaitRule = 'none'): Step {
  return { id, side, kind: 'sendInput', devicePath, inputSource, wait };
}

const STEPS: Step[] = [wait('a', 'before', 3), wait('b', 'before', 5), wait('c', 'after', 1)];
const labelOf = (path: string) => (path === 'path-ultrawide' ? 'Ultrawide' : 'unknown monitor');

describe('stepSentence', () => {
  it('reads a wait step as a sentence', () => {
    expect(stepSentence(wait('a', 'before', 3), labelOf, TABLE)).toBe('Wait 3 seconds');
    expect(stepSentence(wait('a', 'before', 1), labelOf, TABLE)).toBe('Wait 1 second');
  });

  it('reads a send step with its input, monitor and wait rule', () => {
    expect(stepSentence(send('s', 'before', 'path-ultrawide', 0x11, 'drop'), labelOf, TABLE)).toBe('Send HDMI 1 to Ultrawide, then wait until Ultrawide shows HDMI 1 or drops');
    expect(stepSentence(send('s', 'after', 'path-ultrawide', 0x0f, 'available'), labelOf, TABLE)).toBe('Send DisplayPort 1 to Ultrawide, then wait until Ultrawide is Available');
    expect(stepSentence(send('s', 'after', 'gone', 0x1e), labelOf, TABLE)).toBe('Send Input 0x1E to unknown monitor');
  });
});

describe('inputSourceName and the code fields', () => {
  it('names a code from the table or as hex', () => {
    expect(inputSourceName(TABLE, 0x11)).toBe('HDMI 1');
    expect(inputSourceName(TABLE, 0x1e)).toBe('Input 0x1E');
    expect(formatInputCode(0x1e)).toBe('0x1E');
    expect(formatInputCode(5)).toBe('0x05');
  });

  it('reads a typed code in hex or decimal', () => {
    expect(parseInputCode('0x1E')).toBe(30);
    expect(parseInputCode('1e')).toBe(30);
    expect(parseInputCode('30')).toBe(30);
    expect(parseInputCode(' 0x11 ')).toBe(17);
    expect(parseInputCode('')).toBeNaN();
    expect(parseInputCode('zz')).toBeNaN();
  });

  it('bounds the code', () => {
    expect(checkInputCode(0x11)).toBeNull();
    expect(checkInputCode(0xff)).toBeNull();
    expect(checkInputCode(0)).toContain('0x01 and 0xFF');
    expect(checkInputCode(0x100)).not.toBeNull();
    expect(checkInputCode(Number.NaN)).not.toBeNull();
  });
});

describe('stepLabeller', () => {
  it('names monitors by the layout rule and the rest unknown', () => {
    const layout = layoutFixture();
    const label = stepLabeller({ 'path-acer': 'Side' }, layout.summary.monitors);
    expect(label('path-acer')).toBe('Side');
    expect(label('path-msi-1')).toBe('MSI MP165 E6 · USB-C DisplayPort 1');
    expect(label('nope')).toBe('unknown monitor');
  });
});

describe('newSendStep, withKind and sendStepNote', () => {
  it('starts a send step with no wait and converts kinds keeping id and side', () => {
    const step = newSendStep('after', 'fixed', 'path-ultrawide', 0x11);
    expect(step).toEqual({ id: 'fixed', side: 'after', kind: 'sendInput', devicePath: 'path-ultrawide', inputSource: 0x11, wait: 'none' });
    expect(withKind(step, 'wait', { devicePath: 'x', inputSource: 1 })).toEqual(newWaitStep('after', 'fixed'));
    expect(withKind(wait('w', 'before', 4), 'sendInput', { devicePath: 'path-acer', inputSource: 0x0f })).toEqual(newSendStep('before', 'w', 'path-acer', 0x0f));
    expect(withKind(step, 'sendInput', { devicePath: 'x', inputSource: 1 })).toBe(step);
  });

  it('warns about a send step whose monitor is off after the apply, or unknown', () => {
    const layout = layoutFixture();
    expect(sendStepNote(send('s', 'after', 'path-ultrawide', 0x11), layout, labelOf)).toBe('Ultrawide is off after the apply, so this step will be skipped. Move it before the apply.');
    expect(sendStepNote(send('s', 'before', 'path-ultrawide', 0x11), layout, labelOf)).toBeNull();
    expect(sendStepNote(send('s', 'after', 'path-acer', 0x11), layout, labelOf)).toBeNull();
    expect(sendStepNote(send('s', 'before', 'gone', 0x11), layout, labelOf)).toContain('not in the layout any more');
    expect(sendStepNote(wait('w', 'after', 1), layout, labelOf)).toBeNull();
  });
});

describe('waitRuleHint and checkDropWait', () => {
  it('shows the timeout a rule takes', () => {
    expect(waitRuleHint('drop', 5, 120)).toBe('up to 5 s');
    expect(waitRuleHint('available', 5, 120)).toBe('up to 120 s');
    expect(waitRuleHint('none', 5, 120)).toBe('');
    expect(checkDropWait(0)).toBeNull();
    expect(checkDropWait(60)).toBeNull();
    expect(checkDropWait(61)).toContain('between 0 and 60');
    expect(checkDropWait(-1)).not.toBeNull();
    expect(checkAvailableWait(120)).toBeNull();
    expect(checkAvailableWait(0)).toContain('between 1 and 600');
    expect(checkAvailableWait(601)).not.toBeNull();
  });
});

describe('newWaitStep', () => {
  it('starts at three seconds under the given id', () => {
    expect(newWaitStep('after', 'fixed')).toEqual({ id: 'fixed', side: 'after', kind: 'wait', seconds: 3 });
  });
});

describe('stepsOn', () => {
  it('splits the steps by side, in order', () => {
    expect(stepsOn(STEPS, 'before').map((s) => s.id)).toEqual(['a', 'b']);
    expect(stepsOn(STEPS, 'after').map((s) => s.id)).toEqual(['c']);
  });
});

describe('addStep', () => {
  it('lands after the last step on its side', () => {
    expect(addStep(STEPS, wait('d', 'before', 1)).map((s) => s.id)).toEqual(['a', 'b', 'd', 'c']);
    expect(addStep(STEPS, wait('d', 'after', 1)).map((s) => s.id)).toEqual(['a', 'b', 'c', 'd']);
    expect(addStep([wait('c', 'after', 1)], wait('d', 'before', 1)).map((s) => s.id)).toEqual(['d', 'c']);
    expect(addStep([], wait('d', 'after', 1)).map((s) => s.id)).toEqual(['d']);
  });
});

describe('removeStep and replaceStep', () => {
  it('remove drops the step and replace swaps it, both returning new arrays', () => {
    expect(removeStep(STEPS, 'b').map((s) => s.id)).toEqual(['a', 'c']);
    const replaced = replaceStep(STEPS, 'a', wait('a', 'before', 9));
    expect(replaced[0]).toEqual(wait('a', 'before', 9));
    expect(STEPS[0]).toEqual(wait('a', 'before', 3));
  });
});

describe('moveStep', () => {
  it('swaps with the neighbour on the same side', () => {
    expect(moveStep(STEPS, 'b', 'up').map((s) => s.id)).toEqual(['b', 'a', 'c']);
    expect(moveStep(STEPS, 'a', 'down').map((s) => s.id)).toEqual(['b', 'a', 'c']);
    expect(moveStep(STEPS, 'zzz', 'up').map((s) => s.id)).toEqual(['a', 'b', 'c']);
  });

  it('crosses the apply at the edge of a side and stays at the ends of the timeline', () => {
    const down = moveStep(STEPS, 'b', 'down');
    expect(down.map((s) => [s.id, s.side])).toEqual([['a', 'before'], ['b', 'after'], ['c', 'after']]);
    const up = moveStep(STEPS, 'c', 'up');
    expect(up.map((s) => [s.id, s.side])).toEqual([['a', 'before'], ['b', 'before'], ['c', 'before']]);
    expect(moveStep(STEPS, 'a', 'up')).toEqual(STEPS);
    expect(moveStep(STEPS, 'c', 'down')).toEqual(STEPS);
    expect(STEPS[1]!.side).toBe('before');
  });

  it('reports whether a move would change anything', () => {
    expect(canMove(STEPS, 'b', 'up')).toBe(true);
    expect(canMove(STEPS, 'b', 'down')).toBe(true);
    expect(canMove(STEPS, 'c', 'up')).toBe(true);
    expect(canMove(STEPS, 'a', 'up')).toBe(false);
    expect(canMove(STEPS, 'c', 'down')).toBe(false);
  });
});

describe('editsOf and isDirty', () => {
  it('copies the editable fields and notices any change', () => {
    const layout = layoutFixture();
    const edits = editsOf(layout);
    expect(edits).toEqual({ steps: [], dropWaitSeconds: 5, availableWaitSeconds: 120, onApplyFailure: 'stop' });
    expect(isDirty(edits, layout)).toBe(false);
    expect(isDirty({ ...edits, steps: [wait('a', 'before', 3)] }, layout)).toBe(true);
    expect(isDirty({ ...edits, dropWaitSeconds: 6 }, layout)).toBe(true);
    expect(isDirty({ ...edits, onApplyFailure: 'extend' }, layout)).toBe(true);
    edits.steps.push(wait('x', 'after', 1));
    expect(layout.steps).toHaveLength(0);
  });
});

describe('checkWaitSeconds', () => {
  it('bounds the field', () => {
    expect(checkWaitSeconds(3)).toBeNull();
    expect(checkWaitSeconds(600)).toBeNull();
    expect(checkWaitSeconds(0)).toContain('between 1 and 600');
    expect(checkWaitSeconds(601)).not.toBeNull();
    expect(checkWaitSeconds(2.5)).not.toBeNull();
    expect(checkWaitSeconds(Number.NaN)).not.toBeNull();
  });
});
