import { describe, expect, it } from 'vitest';
import { scriptIndicator, stateOf } from './script-state';

describe('scriptIndicator', () => {
  it('reads as good when the script is current', () => {
    expect(scriptIndicator('current')).toEqual({ tone: 'good', text: 'Script up to date' });
  });

  it('puts the action first when the script is stale or missing', () => {
    expect(scriptIndicator('stale')?.tone).toBe('warn');
    expect(scriptIndicator('stale')?.text).toMatch(/^Regenerate the script/);
    expect(scriptIndicator('missing')?.text).toMatch(/^Regenerate the script/);
    expect(scriptIndicator('missing')?.text).toContain('missing');
  });

  it('says nothing before the state is known', () => {
    expect(scriptIndicator(null)).toBeNull();
  });
});

describe('stateOf', () => {
  it('finds the layout in the list and is null when it is not there', () => {
    const statuses = [{ layoutId: 'a', state: 'stale' as const, path: 'C:/x/switch.ps1' }];
    expect(stateOf(statuses, 'a')).toBe('stale');
    expect(stateOf(statuses, 'b')).toBeNull();
    expect(stateOf([], 'a')).toBeNull();
  });
});
