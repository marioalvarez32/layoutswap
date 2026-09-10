import { describe, expect, it } from 'vitest';
import { inventoryFixture } from '@/test/fixtures';
import { capturePreviewMonitors, capturePreviewRows } from './capture';

describe('capturePreviewRows', () => {
  it('lists connected monitors, on for Active and off for Available, and drops Absent ones', () => {
    const inventory = inventoryFixture();
    inventory.monitors.push({ ...inventory.monitors[4]!, devicePath: 'path-tv', reportedName: 'TV', state: 'Absent' });
    const rows = capturePreviewRows(inventory, {});
    expect(rows.map((r) => `${r.label}:${r.on ? 'on' : 'off'}`)).toEqual([
      'MSI MP165 E6:on',
      'MSI MP165 E6:on',
      'KG241Y X1:on',
      'Built-in display:on',
      'VG34VQEL1A:off',
    ]);
  });

  it('formats size, position and primary for on monitors and leaves them blank for off ones', () => {
    const rows = capturePreviewRows(inventoryFixture(), {});
    const acer = rows.find((r) => r.label === 'KG241Y X1')!;
    expect(acer).toMatchObject({ size: '1920×1080 · 60 Hz', position: '0,0', primary: true, detail: 'HDMI' });
    const ultrawide = rows.find((r) => r.label === 'VG34VQEL1A')!;
    expect(ultrawide).toMatchObject({ size: '', position: '', primary: false });
  });

  it('uses the alias fallback', () => {
    const rows = capturePreviewRows(inventoryFixture(), { 'path-acer': 'Side' });
    expect(rows.find((r) => r.devicePath === 'path-acer')).toMatchObject({ label: 'Side', detail: 'KG241Y X1' });
  });

  it('gives the schematic the same monitors, with off ones stripped of their position', () => {
    const monitors = capturePreviewMonitors(inventoryFixture());
    expect(monitors.map((m) => m.on)).toEqual([true, true, true, true, false]);
    expect(monitors[4]).toMatchObject({ reportedName: 'VG34VQEL1A', position: null, size: null, primary: false });
    expect(capturePreviewMonitors(null)).toEqual([]);
  });

  it('is empty before the first probe', () => {
    expect(capturePreviewRows(null, {})).toEqual([]);
  });
});
