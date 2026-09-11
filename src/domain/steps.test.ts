import { describe, expect, it } from 'vitest';
import type { Step } from '@/domain/generated/types';
import { layoutFixture } from '@/test/fixtures';
import {
  addStep,
  canMove,
  checkWaitSeconds,
  editsOf,
  isDirty,
  moveStep,
  newWaitStep,
  removeStep,
  stepSentence,
  stepsOn,
  updateStep,
} from './steps';

function wait(id: string, side: Step['side'], seconds: number): Step {
  return { id, side, kind: 'wait', seconds };
}

const STEPS: Step[] = [wait('a', 'before', 3), wait('b', 'before', 5), wait('c', 'after', 1)];

describe('stepSentence', () => {
  it('reads a wait step as a sentence', () => {
    expect(stepSentence(wait('a', 'before', 3))).toBe('Wait 3 seconds');
    expect(stepSentence(wait('a', 'before', 1))).toBe('Wait 1 second');
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

describe('removeStep and updateStep', () => {
  it('remove drops the step and update patches it, both returning new arrays', () => {
    expect(removeStep(STEPS, 'b').map((s) => s.id)).toEqual(['a', 'c']);
    const updated = updateStep(STEPS, 'a', { seconds: 9 });
    expect(updated[0]).toEqual(wait('a', 'before', 9));
    expect(STEPS[0]!.seconds).toBe(3);
  });
});

describe('moveStep', () => {
  it('swaps with the neighbour on the same side and stays at the edge', () => {
    expect(moveStep(STEPS, 'b', 'up').map((s) => s.id)).toEqual(['b', 'a', 'c']);
    expect(moveStep(STEPS, 'a', 'down').map((s) => s.id)).toEqual(['b', 'a', 'c']);
    expect(moveStep(STEPS, 'a', 'up').map((s) => s.id)).toEqual(['a', 'b', 'c']);
    // Never crosses the apply.
    expect(moveStep(STEPS, 'b', 'down').map((s) => s.id)).toEqual(['a', 'b', 'c']);
    expect(moveStep(STEPS, 'c', 'up').map((s) => s.id)).toEqual(['a', 'b', 'c']);
    expect(moveStep(STEPS, 'zzz', 'up').map((s) => s.id)).toEqual(['a', 'b', 'c']);
  });

  it('reports whether a move would change anything', () => {
    expect(canMove(STEPS, 'b', 'up')).toBe(true);
    expect(canMove(STEPS, 'b', 'down')).toBe(false);
    expect(canMove(STEPS, 'c', 'up')).toBe(false);
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
