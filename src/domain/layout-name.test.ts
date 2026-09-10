import { describe, expect, it } from 'vitest';
import { checkLayoutName } from './layout-name';

const existing = [
  { id: 'a', name: 'Desk' },
  { id: 'b', name: 'Console' },
];

describe('checkLayoutName', () => {
  it('trims the name', () => {
    expect(checkLayoutName('  Film ', existing)).toEqual({ name: 'Film', ok: true, message: null, conflict: null });
  });

  it('refuses an empty or blank name', () => {
    expect(checkLayoutName('', existing).ok).toBe(false);
    expect(checkLayoutName('   ', existing).message).toBe('Give the layout a name.');
  });

  it('refuses more than 40 characters and allows exactly 40', () => {
    expect(checkLayoutName('x'.repeat(40), existing).ok).toBe(true);
    const check = checkLayoutName('x'.repeat(41), existing);
    expect(check.ok).toBe(false);
    expect(check.message).toBe('Keep the name to 40 characters or fewer.');
  });

  it('reports the layout that already has the name, regardless of case', () => {
    const check = checkLayoutName('desk', existing);
    expect(check.ok).toBe(true);
    expect(check.conflict).toEqual({ id: 'a', name: 'Desk' });
  });

  it('does not count the layout being re-captured as a conflict', () => {
    expect(checkLayoutName('DESK', existing, 'a').conflict).toBeNull();
    expect(checkLayoutName('DESK', existing, 'b').conflict).toEqual({ id: 'a', name: 'Desk' });
  });
});
