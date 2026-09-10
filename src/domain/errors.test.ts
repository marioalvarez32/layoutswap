import { describe, expect, it } from 'vitest';
import { errorMessage } from './errors';

describe('errorMessage', () => {
  it('uses the message of an AppErrorPayload from Rust', () => {
    expect(errorMessage({ message: 'Update layoutswap.', logPath: null })).toBe('Update layoutswap.');
  });

  it('uses the message of an Error', () => {
    expect(errorMessage(new Error('boom'))).toBe('boom');
  });

  it('turns anything else into text', () => {
    expect(errorMessage('plain')).toBe('plain');
    expect(errorMessage(42)).toBe('42');
  });
});
