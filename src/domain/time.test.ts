import { describe, expect, it } from 'vitest';
import { describeCaptureTime, describeProbeAge, formatProbeTime } from './time';

const now = new Date(2026, 8, 9, 14, 40, 0);

describe('formatProbeTime', () => {
  it('says today for a probe from the same day', () => {
    expect(formatProbeTime(new Date(2026, 8, 9, 14, 32, 5).toISOString(), now)).toBe('14:32 today');
  });

  it('names the day for an older probe', () => {
    expect(formatProbeTime(new Date(2026, 8, 8, 9, 5, 0).toISOString(), now)).toBe('09:05, 8 Sep');
  });

  it('copes with a timestamp it cannot parse', () => {
    expect(formatProbeTime('nope', now)).toBe('unknown');
  });
});

describe('describeProbeAge', () => {
  it('is "just now" for the first minute', () => {
    expect(describeProbeAge(new Date(2026, 8, 9, 14, 39, 30).toISOString(), now)).toBe('read from Windows just now');
  });

  it('names the time after that', () => {
    expect(describeProbeAge(new Date(2026, 8, 9, 14, 32, 0).toISOString(), now)).toBe('read from Windows at 14:32');
  });
});

describe('describeCaptureTime', () => {
  it('reads as a sentence fragment', () => {
    expect(describeCaptureTime(new Date(2026, 8, 9, 14, 32, 0).toISOString())).toBe('Captured 9 Sep at 14:32');
  });
});
