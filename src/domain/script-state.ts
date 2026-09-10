import type { ScriptState, ScriptStatus } from '@/domain/generated/types';

export interface ScriptIndicator {
  /** 'good' when current, 'warn' otherwise. */
  tone: 'good' | 'warn';
  text: string;
}

/**
 * The one-line script indicator under a layout's name, with the next action first
 * when one is needed (DESIGN.md, Copy voice). Null before the states have loaded.
 */
export function scriptIndicator(state: ScriptState | null): ScriptIndicator | null {
  switch (state) {
    case 'current':
      return { tone: 'good', text: 'Script up to date' };
    case 'stale':
      return { tone: 'warn', text: 'Regenerate the script: it is older than the app or the layout' };
    case 'missing':
      return { tone: 'warn', text: 'Regenerate the script: it is missing from the layout folder' };
    default:
      return null;
  }
}

/** The state of one layout's script from the list the app returns, or null when unknown. */
export function stateOf(statuses: readonly ScriptStatus[], layoutId: string): ScriptState | null {
  return statuses.find((s) => s.layoutId === layoutId)?.state ?? null;
}
