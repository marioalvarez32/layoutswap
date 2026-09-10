import { describe, expect, it } from 'vitest';
import { formatOnOffCount } from './layouts';

describe('formatOnOffCount', () => {
  it('reads as on then off with a middle dot', () => {
    expect(formatOnOffCount({ onCount: 3, offCount: 1 })).toBe('3 on · 1 off');
    expect(formatOnOffCount({ onCount: 1, offCount: 0 })).toBe('1 on · 0 off');
  });
});
