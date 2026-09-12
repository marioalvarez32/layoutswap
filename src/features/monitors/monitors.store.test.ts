import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Capabilities } from '@/domain/generated/types';
import { loadConfig, readCapabilities, readMissingCapabilities } from '@/tauri/commands';
import { useMonitorsStore } from './monitors.store';

vi.mock('@/tauri/commands', async () => (await import('@/test/commands')).commandsMock());

function entry(overrides: Partial<Capabilities> = {}): Capabilities {
  return {
    readAt: '2026-09-11T11:52:00-05:00',
    answered: true,
    inputCodes: [0x11, 0x12, 0x0f],
    powerModes: [1, 5],
    modes: [{ width: 3440, height: 1440, hz: 100 }],
    raw: '(prot(monitor)vcp(60(11 12 0F) D6(01 05)))',
    ...overrides,
  };
}

describe('monitors store', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('loads the capabilities from the config', async () => {
    const store = useMonitorsStore();
    vi.mocked(loadConfig).mockResolvedValueOnce({ schemaVersion: 3, window: { width: 1280, height: 860 }, aliases: {}, capabilities: { 'path-acer': entry() }, layouts: [] });
    await store.load();
    expect(store.capabilities['path-acer']?.powerModes).toEqual([1, 5]);
  });

  it('keeps what the first-sight read returns and leaves the map alone when nothing was read', async () => {
    const store = useMonitorsStore();
    store.capabilities = { 'path-acer': entry() };
    vi.mocked(readMissingCapabilities).mockResolvedValueOnce(null);
    await store.readMissing();
    expect(Object.keys(store.capabilities)).toEqual(['path-acer']);

    vi.mocked(readMissingCapabilities).mockResolvedValueOnce({ 'path-acer': entry(), 'path-msi': entry({ powerModes: [5] }) });
    await store.readMissing();
    expect(Object.keys(store.capabilities)).toEqual(['path-acer', 'path-msi']);
    expect(store.reading).toBe(false);
    expect(store.readError).toBeNull();
  });

  it('re-check replaces the entries and shows that a read is running until it returns', async () => {
    const store = useMonitorsStore();
    store.capabilities = { 'path-acer': entry() };
    let finish: (value: Record<string, Capabilities>) => void = () => {};
    vi.mocked(readCapabilities).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    const pending = store.reCheck();
    expect(store.reading).toBe(true);
    await store.reCheck();
    expect(readCapabilities).toHaveBeenCalledTimes(1);
    finish({ 'path-acer': entry({ powerModes: [5] }) });
    await pending;
    expect(store.reading).toBe(false);
    expect(store.capabilities['path-acer']?.powerModes).toEqual([5]);
  });

  it('keeps the entries and records the message when a read fails', async () => {
    const store = useMonitorsStore();
    store.capabilities = { 'path-acer': entry() };
    vi.mocked(readCapabilities).mockRejectedValueOnce(new Error('Re-check the monitors once they are all showing a picture: the capabilities read exited with exit code 1 (x).'));
    await store.reCheck();
    expect(store.readError).toContain('Re-check the monitors');
    expect(Object.keys(store.capabilities)).toEqual(['path-acer']);
    expect(store.reading).toBe(false);
  });
});
