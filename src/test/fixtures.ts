import type { Inventory, Layout, Monitor } from '@/domain/generated/types';

const GPU = 'NVIDIA GeForce RTX 5070 Laptop GPU';
const GPU_PATH = String.raw`\\?\PCI#VEN_10DE&DEV_2D18#0#{5b45201d-f2f2-4f3b-85bb-30ff1f953599}`;

function monitor(overrides: Partial<Monitor> & Pick<Monitor, 'devicePath' | 'reportedName' | 'connector' | 'state'>): Monitor {
  return {
    gpu: GPU,
    gpuDevicePath: GPU_PATH,
    gdiName: null,
    position: null,
    size: null,
    refreshHz: null,
    rotation: null,
    scalePercent: null,
    primary: false,
    inputSource: null,
    inputSourceName: null,
    ddcCi: 'notRead',
    ...overrides,
  };
}

/**
 * The reference machine as the probe reports it: four monitors on, the ultrawide
 * plugged in but off, and two identical MSI panels.
 */
export function inventoryFixture(): Inventory {
  return {
    probedAt: '2026-09-09T14:32:05.1234567-05:00',
    monitors: [
      monitor({ devicePath: 'path-msi-2', reportedName: 'MSI MP165 E6', connector: 'USB-C DisplayPort 2', state: 'Active', gdiName: String.raw`\\.\DISPLAY5`, position: { x: 0, y: -1080 }, size: { width: 1920, height: 1080 }, refreshHz: 60, rotation: 0, scalePercent: 100 }),
      monitor({ devicePath: 'path-msi-1', reportedName: 'MSI MP165 E6', connector: 'USB-C DisplayPort 1', state: 'Active', gdiName: String.raw`\\.\DISPLAY4`, position: { x: -1920, y: 0 }, size: { width: 1920, height: 1080 }, refreshHz: 60, rotation: 0, scalePercent: 100 }),
      monitor({ devicePath: 'path-acer', reportedName: 'KG241Y X1', connector: 'HDMI', state: 'Active', gdiName: String.raw`\\.\DISPLAY2`, position: { x: 0, y: 0 }, size: { width: 1920, height: 1080 }, refreshHz: 60, rotation: 0, scalePercent: 100, primary: true, inputSource: 0x11, inputSourceName: 'HDMI 1', ddcCi: 'answered' }),
      monitor({ devicePath: 'path-builtin', reportedName: 'Built-in display', connector: 'Built-in', state: 'Active', gdiName: String.raw`\\.\DISPLAY1`, position: { x: 1920, y: 0 }, size: { width: 2560, height: 1600 }, refreshHz: 165, rotation: 0, scalePercent: 150, ddcCi: 'notAnswering' }),
      monitor({ devicePath: 'path-ultrawide', reportedName: 'VG34VQEL1A', connector: 'DisplayPort', state: 'Available' }),
    ],
    arrangement: {
      paths: 'QUFB',
      modes: 'QkJC',
      sourceAdapters: [GPU_PATH, GPU_PATH, GPU_PATH, GPU_PATH],
      targetAdapters: [GPU_PATH, GPU_PATH, GPU_PATH, GPU_PATH],
      modeAdapters: [GPU_PATH, GPU_PATH, GPU_PATH, GPU_PATH, GPU_PATH, GPU_PATH, GPU_PATH, GPU_PATH],
    },
  };
}

/** The layout a capture of `inventoryFixture()` produces. */
export function layoutFixture(): Layout {
  const inventory = inventoryFixture();
  return {
    id: 'layout-desk',
    name: 'Desk',
    folder: 'desk',
    capturedAt: '2026-09-09T14:33:00.0000000-05:00',
    script: { templateVersion: 1, renderedAt: '2026-09-09T14:33:01.0000000-05:00' },
    arrangement: inventory.arrangement,
    summary: {
      monitors: inventory.monitors
        .filter((m) => m.state !== 'Absent')
        .map((m) => {
          const on = m.state === 'Active';
          return {
            devicePath: m.devicePath,
            reportedName: m.reportedName,
            connector: m.connector,
            on,
            position: on ? m.position : null,
            size: on ? m.size : null,
            refreshHz: on ? m.refreshHz : null,
            rotation: on ? m.rotation : null,
            scalePercent: on ? m.scalePercent : null,
            primary: on && m.primary,
          };
        }),
    },
  };
}
