import { describe, expect, it } from 'vitest';
import { inventoryFixture } from '@/test/fixtures';
import {
  chipLabels,
  formatPosition,
  formatRefresh,
  formatSize,
  formatSpec,
  inLayoutHint,
  inputSourceTooltip,
  liveMonitor,
  liveState,
  monitorDisplay,
  monitorStateNote,
  stateLabel,
} from './monitors';

const panel = { devicePath: 'path-1', reportedName: 'MSI MP165 E6', connector: 'USB-C DisplayPort 2' };

describe('monitorDisplay', () => {
  it('shows the alias with the reported name under it', () => {
    expect(monitorDisplay({ 'path-1': 'Portrait' }, panel)).toEqual({ label: 'Portrait', detail: 'MSI MP165 E6' });
  });

  it('falls back to the reported name plus connector when no alias is set', () => {
    expect(monitorDisplay({}, panel)).toEqual({ label: 'MSI MP165 E6', detail: 'USB-C DisplayPort 2' });
    expect(monitorDisplay({ 'path-1': '   ' }, panel).label).toBe('MSI MP165 E6');
  });
});

describe('chipLabels', () => {
  it('adds the detail only where two monitors share a label', () => {
    const labels = chipLabels([
      { label: 'MSI MP165 E6', detail: 'USB-C DisplayPort 1' },
      { label: 'MSI MP165 E6', detail: 'USB-C DisplayPort 2' },
      { label: 'KG241Y X1', detail: 'HDMI 1' },
    ]);
    expect(labels).toEqual(['MSI MP165 E6 · USB-C DisplayPort 1', 'MSI MP165 E6 · USB-C DisplayPort 2', 'KG241Y X1']);
  });
});

describe('formatting', () => {
  it('formats size with refresh', () => {
    expect(formatSize({ width: 3440, height: 1440 }, 120)).toBe('3440×1440 · 120 Hz');
    expect(formatSize({ width: 1920, height: 1080 }, null)).toBe('1920×1080');
    expect(formatSize(null, 60)).toBe('');
  });

  it('keeps fractional refresh rates short', () => {
    expect(formatRefresh(59.94)).toBe('59.94 Hz');
    expect(formatRefresh(60.0)).toBe('60 Hz');
    expect(formatRefresh(164.9999)).toBe('165 Hz');
  });

  it('formats position as x,y', () => {
    expect(formatPosition({ x: -596, y: -1440 })).toBe('-596,-1440');
    expect(formatPosition(null)).toBe('');
  });

  it('formats the full spec line of an on monitor', () => {
    expect(formatSpec({ position: { x: 0, y: 0 }, size: { width: 3440, height: 1440 }, refreshHz: 120, rotation: 0, scalePercent: 100 }))
      .toBe('0,0 · 3440×1440 · 120 Hz · landscape · 100%');
    expect(formatSpec({ position: { x: 1920, y: 0 }, size: { width: 1080, height: 1920 }, refreshHz: 60, rotation: 90, scalePercent: 125 }))
      .toBe('1920,0 · 1080×1920 · 60 Hz · portrait · 125%');
  });
});

describe('state copy', () => {
  it('explains Available and Absent, and says nothing for Active', () => {
    expect(monitorStateNote('Active')).toBe('');
    expect(monitorStateNote('Active', true)).toContain('not showing a picture');
    expect(monitorStateNote('Available')).toContain('Windows is not drawing to it');
    expect(monitorStateNote('Absent')).toContain('press its input button');
  });

  it('puts the physical action first for an Absent monitor the layout needs', () => {
    expect(inLayoutHint('TV', true, 'Absent')).toBe('TV is Absent: press its input button or plug it in, then switch again.');
    expect(inLayoutHint('TV', false, 'Absent')).toContain('does not need it');
    expect(inLayoutHint('Side', true, 'Available')).toContain('turn it on');
    expect(inLayoutHint('Side', false, 'Available')).toContain('Save current layout again');
    expect(inLayoutHint('Side', true, 'Active')).toBe('');
  });
});

describe('liveMonitor', () => {
  it('finds the probe entry by device path, or nothing', () => {
    const inventory = inventoryFixture();
    expect(liveMonitor(inventory, 'path-acer')?.inputSourceName).toBe('HDMI 1');
    expect(liveMonitor(inventory, 'never-seen')).toBeNull();
    expect(liveMonitor(null, 'path-acer')).toBeNull();
  });
});

describe('inputSourceTooltip', () => {
  it('carries the code as two hex digits', () => {
    expect(inputSourceTooltip(0x11)).toBe('Input source code 0x11');
    expect(inputSourceTooltip(5)).toBe('Input source code 0x05');
  });
});

describe('liveState', () => {
  it('reads the state from the latest probe and treats an unlisted monitor as Absent', () => {
    const inventory = inventoryFixture();
    expect(liveState(inventory, inventory.monitors[0]!.devicePath)).toBe('Active');
    expect(liveState(inventory, 'never-seen')).toBe('Absent');
    expect(liveState(null, 'anything')).toBeNull();
  });
});

describe('stateLabel', () => {
  it('adds the asleep suffix to Active only', () => {
    expect(stateLabel('Active', true)).toBe('Active, asleep');
    expect(stateLabel('Active', false)).toBe('Active');
    expect(stateLabel('Available', false)).toBe('Available');
    expect(stateLabel('Absent', false)).toBe('Absent');
  });
});
