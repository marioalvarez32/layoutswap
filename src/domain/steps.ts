import type { Layout, LayoutEdits, Step, StepSide } from '@/domain/generated/types';

/** The bounds a wait step keeps, the same as the Rust side enforces on save. */
export const WAIT_SECONDS_MIN = 1;
export const WAIT_SECONDS_MAX = 600;
export const DEFAULT_WAIT_SECONDS = 3;

/** The edits of a layout that has none: the same defaults the migration writes. */
export const DEFAULT_EDITS: LayoutEdits = { steps: [], dropWaitSeconds: 5, availableWaitSeconds: 120, onApplyFailure: 'stop' };

/** The step as one sentence, the same one the script prints as its row. */
export function stepSentence(step: Step): string {
  switch (step.kind) {
    case 'wait':
      return `Wait ${step.seconds} ${step.seconds === 1 ? 'second' : 'seconds'}`;
  }
}

/** A fresh wait step on the given side, under the id the caller made for it. */
export function newWaitStep(side: StepSide, id: string): Step {
  return { id, side, kind: 'wait', seconds: DEFAULT_WAIT_SECONDS };
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

/** Replaces the step with `id` by `patch` applied to it. */
export function updateStep(steps: readonly Step[], id: string, patch: Partial<Step>): Step[] {
  return steps.map((s) => (s.id === id ? { ...s, ...patch } as Step : s));
}

/**
 * Moves a step one place up or down among the steps on its side. A step already at
 * the edge stays; a step never crosses the apply this way.
 */
export function moveStep(steps: readonly Step[], id: string, direction: 'up' | 'down'): Step[] {
  const index = steps.findIndex((s) => s.id === id);
  if (index === -1) {
    return [...steps];
  }
  const side = steps[index]!.side;
  const neighbour = direction === 'up'
    ? steps.slice(0, index).map((s, i) => [s, i] as const).reverse().find(([s]) => s.side === side)
    : steps.slice(index + 1).map((s, i) => [s, index + 1 + i] as const).find(([s]) => s.side === side);
  if (!neighbour) {
    return [...steps];
  }
  const next = [...steps];
  const [, j] = neighbour;
  [next[index], next[j]] = [next[j]!, next[index]!];
  return next;
}

/** Whether a step can move in a direction: false at the edge of its side. */
export function canMove(steps: readonly Step[], id: string, direction: 'up' | 'down'): boolean {
  return moveStep(steps, id, direction).some((s, i) => s.id !== steps[i]?.id);
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
