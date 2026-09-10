import { describe, expect, it } from 'vitest';
import { layoutFixture } from '@/test/fixtures';
import { formatOnOffCount, toListItem, upsertLayout } from './layouts';

describe('toListItem', () => {
  it('counts on and off monitors from the summary', () => {
    const item = toListItem(layoutFixture());
    expect(item).toEqual({ id: 'layout-desk', name: 'Desk', onCount: 4, offCount: 1 });
  });
});

describe('formatOnOffCount', () => {
  it('reads as on then off with a middle dot', () => {
    expect(formatOnOffCount({ onCount: 3, offCount: 1 })).toBe('3 on · 1 off');
    expect(formatOnOffCount({ onCount: 1, offCount: 0 })).toBe('1 on · 0 off');
  });
});

describe('upsertLayout', () => {
  it('appends a new layout and replaces one with the same id in place', () => {
    const desk = layoutFixture();
    const console_ = { ...layoutFixture(), id: 'layout-console', name: 'Console' };
    const list = upsertLayout([desk], console_);
    expect(list.map((l) => l.name)).toEqual(['Desk', 'Console']);
    const replaced = upsertLayout(list, { ...desk, name: 'Desk 2' });
    expect(replaced.map((l) => l.name)).toEqual(['Desk 2', 'Console']);
  });
});
