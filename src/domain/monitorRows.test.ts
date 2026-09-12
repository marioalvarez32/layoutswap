import { describe, expect, it } from 'vitest';
import type { Capabilities } from '@/domain/generated/types';
import { inputSourcesFixture, inventoryFixture, layoutFixture } from '@/test/fixtures';
import { monitorRows, type MonitorRowsContext } from './monitorRows';

function entry(overrides: Partial<Capabilities> = {}): Capabilities {
  return {
    readAt: '2026-09-11T11:52:00-05:00',
    answered: true,
    inputCodes: [0x11, 0x12],
    powerModes: [1, 5],
    modes: [{ width: 1920, height: 1080, hz: 60 }],
    raw: '',
    ...overrides,
  };
}

function context(overrides: Partial<MonitorRowsContext> = {}): MonitorRowsContext {
  return {
    inventory: inventoryFixture(),
    layouts: [layoutFixture()],
    aliases: { 'path-acer': 'Side' },
    inputSources: inputSourcesFixture(),
    capabilities: { 'path-acer': entry() },
    reading: false,
    expanded: new Set(),
    ...overrides,
  };
}

function row(rows: ReturnType<typeof monitorRows>, devicePath: string) {
  return rows.find((r) => r.devicePath === devicePath)!;
}

describe('monitorRows', () => {
  it('composes a row from the probe, the aliases, the layouts and the capabilities', () => {
    const rows = monitorRows(context());
    expect(rows).toHaveLength(5);
    const acer = row(rows, 'path-acer');
    expect(acer).toMatchObject({
      alias: 'Side',
      reportedName: 'KG241Y X1',
      stateText: 'Active',
      tone: 'good',
      note: 'In Desk',
      connector: 'HDMI',
      position: '0,0',
      size: '1920×1080 · 60 Hz',
      input: 'HDMI 1',
      inputKnown: true,
      canExpand: true,
      expanded: false,
      muted: null,
      wake: { text: 'Can be woken by the app', tone: 'good' },
      modes: ['1920 x 1080 at 60 Hz'],
    });
    expect(acer.accepts).toEqual([{ name: 'HDMI 1', current: true }, { name: 'HDMI 2', current: false }]);
    expect(acer.readAt).toMatch(/^Read on 11 Sep 2026/);
  });

  it('reads a monitor that is not Active from the layouts and says it cannot be read', () => {
    const ultrawide = row(monitorRows(context()), 'path-ultrawide');
    expect(ultrawide).toMatchObject({
      alias: 'VG34VQEL1A',
      stateText: 'Available',
      tone: 'warn',
      note: 'Plugged in, Windows is not drawing to it',
      position: 'not Active',
      size: 'not Active',
      input: 'unknown',
      inputKnown: false,
      canExpand: false,
      muted: 'Switch it on in Windows first',
      accepts: [],
      wake: { text: 'Unknown', tone: 'mute' },
      readAt: '',
    });
  });

  it('marks the rows being read and the ones that are open', () => {
    const rows = monitorRows(context({ reading: true, expanded: new Set(['path-acer']) }));
    expect(row(rows, 'path-acer')).toMatchObject({ canExpand: false, muted: 'Reading capabilities', reading: true, expanded: true });
    expect(row(rows, 'path-builtin')).toMatchObject({ muted: 'Reading capabilities', reading: true });
    expect(row(rows, 'path-ultrawide')).toMatchObject({ muted: 'Switch it on in Windows first', reading: false });
  });

  it('has no rows before the first probe', () => {
    expect(monitorRows(context({ inventory: null }))).toEqual([]);
  });
});
