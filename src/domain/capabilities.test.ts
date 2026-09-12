import { describe, expect, it } from 'vitest';
import type { Capabilities, Monitor } from '@/domain/generated/types';
import { inputSourcesFixture, inventoryFixture, layoutFixture } from '@/test/fixtures';
import {
  acceptedInputs,
  formatReadAt,
  lastSeenSize,
  modeLines,
  monitorNote,
  unreadReason,
  wakeText,
} from './capabilities';

function entry(overrides: Partial<Capabilities> = {}): Capabilities {
  return {
    readAt: '2026-09-11T11:52:00-05:00',
    answered: true,
    inputCodes: [0x11, 0x0f, 0x1e],
    powerModes: [1, 5],
    modes: [
      { width: 3440, height: 1440, hz: 60 },
      { width: 3440, height: 1440, hz: 100 },
      { width: 1920, height: 1080, hz: 60 },
      { width: 1920, height: 1080, hz: 120 },
      { width: 1920, height: 1080, hz: 144 },
    ],
    raw: '',
    ...overrides,
  };
}

function monitor(devicePath: string): Monitor {
  return inventoryFixture().monitors.find((m) => m.devicePath === devicePath)!;
}

describe('wakeText', () => {
  it('reads the declared power modes', () => {
    expect(wakeText(entry({ powerModes: [1, 2, 4, 5] }))).toEqual({ text: 'Can be woken by the app', tone: 'good' });
    expect(wakeText(entry({ powerModes: [5] }))).toEqual({ text: 'Needs a button press to wake', tone: 'warn' });
    expect(wakeText(entry({ powerModes: [] }))).toEqual({ text: 'Needs a button press to wake', tone: 'warn' });
    expect(wakeText(entry({ answered: false, powerModes: [] }))).toEqual({ text: 'Unknown', tone: 'mute' });
    expect(wakeText(null)).toEqual({ text: 'Unknown', tone: 'mute' });
  });
});

describe('modeLines', () => {
  it('groups the rates under each resolution, largest resolution first', () => {
    expect(modeLines(entry().modes)).toEqual(['3440 x 1440 at 60, 100 Hz', '1920 x 1080 at 60, 120, 144 Hz']);
    expect(modeLines([])).toEqual([]);
  });
});

describe('acceptedInputs', () => {
  it('names the accepted inputs in the table order, an unknown code as hex, the current one marked', () => {
    expect(acceptedInputs(inputSourcesFixture(), entry(), 0x11)).toEqual([
      { name: 'DisplayPort 1', current: false },
      { name: 'HDMI 1', current: true },
      { name: 'Input 0x1E', current: false },
    ]);
    expect(acceptedInputs(inputSourcesFixture(), entry(), null).every((i) => !i.current)).toBe(true);
    expect(acceptedInputs(inputSourcesFixture(), entry({ answered: false, inputCodes: [] }), 0x11)).toEqual([]);
    expect(acceptedInputs(inputSourcesFixture(), null, 0x11)).toEqual([]);
  });
});

describe('unreadReason', () => {
  it('says why a row has no capabilities to show, or nothing when it has', () => {
    expect(unreadReason(monitor('path-acer'), entry(), false)).toBeNull();
    expect(unreadReason(monitor('path-acer'), entry(), true)).toBe('Reading capabilities');
    expect(unreadReason(monitor('path-acer'), null, false)).toBe('Not read yet');
    expect(unreadReason(monitor('path-acer'), null, true)).toBe('Reading capabilities');
    expect(unreadReason(monitor('path-builtin'), entry({ answered: false, inputCodes: [] }), false)).toBe('Does not answer over DDC-CI');
    expect(unreadReason(monitor('path-ultrawide'), null, false)).toBe('Switch it on in Windows first');
    expect(unreadReason(monitor('path-ultrawide'), entry(), false)).toBe('Switch it on in Windows first');
    expect(unreadReason(monitor('path-ultrawide'), null, true)).toBe('Switch it on in Windows first');
  });
});

describe('monitorNote', () => {
  it('names the layouts an Active monitor is on in, and the physical fact for the others', () => {
    const desk = layoutFixture();
    const film = { ...layoutFixture(), id: 'layout-film', name: 'Film', summary: { monitors: desk.summary.monitors.map((m) => ({ ...m, on: m.devicePath === 'path-acer' })) } };
    expect(monitorNote(monitor('path-acer'), [desk, film])).toBe('In Desk, Film');
    expect(monitorNote(monitor('path-builtin'), [desk, film])).toBe('In Desk');
    expect(monitorNote(monitor('path-acer'), [])).toBe('Not in a layout');
    expect(monitorNote(monitor('path-ultrawide'), [desk])).toBe('Plugged in, Windows is not drawing to it');
    expect(monitorNote({ ...monitor('path-ultrawide'), state: 'Absent' }, [desk])).toBe('Press its input button, then Refresh');
  });
});

describe('lastSeenSize', () => {
  it('reads the size from the layout that last saw the monitor on, or nothing', () => {
    const desk = layoutFixture();
    expect(lastSeenSize([desk], 'path-acer')).toBe('last seen 1920×1080 · 60 Hz');
    expect(lastSeenSize([desk], 'path-ultrawide')).toBe('');
    expect(lastSeenSize([], 'path-acer')).toBe('');
  });
});

describe('formatReadAt', () => {
  it('reads as a date and a time', () => {
    expect(formatReadAt('2026-09-11T11:52:00-05:00')).toMatch(/^Read on 11 Sep 2026, \d\d:\d\d$/);
    expect(formatReadAt('not a date')).toBe('Read at an unknown time');
  });
});
