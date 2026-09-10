import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { isWindowMaximized, loadConfig, saveWindowSize } from '@/tauri/commands';
import { defaultConfig } from '@/test/commands';
import { withSetup } from '@/test/withSetup';
import { useWindowSize } from './useWindowSize';

// vi.mock is hoisted above the imports, so the factory loads its helper itself.
vi.mock('@/tauri/commands', async () => (await import('@/test/commands')).commandsMock());

function resizeTo(width: number, height: number) {
  Object.assign(window, { innerWidth: width, innerHeight: height });
  window.dispatchEvent(new Event('resize'));
}

async function settle() {
  await vi.runAllTimersAsync();
}

describe('useWindowSize', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.mocked(loadConfig).mockResolvedValue(defaultConfig());
    vi.mocked(saveWindowSize).mockResolvedValue(undefined);
    vi.mocked(isWindowMaximized).mockResolvedValue(false);
  });

  afterEach(() => {
    vi.clearAllMocks();
    vi.useRealTimers();
  });

  it('restores the persisted size from the config on start', async () => {
    const { result, unmount } = withSetup(() => useWindowSize());
    await settle();
    expect(result.persisted.value).toEqual({ width: 1280, height: 860 });
    expect(saveWindowSize).not.toHaveBeenCalled();
    unmount();
  });

  it('saves once after a burst of resizes settles, with the final size', async () => {
    const { unmount } = withSetup(() => useWindowSize({ debounceMs: 100 }));
    await settle();

    resizeTo(1300, 860);
    resizeTo(1350, 870);
    resizeTo(1400, 900);
    await settle();

    expect(saveWindowSize).toHaveBeenCalledTimes(1);
    expect(saveWindowSize).toHaveBeenCalledWith({ width: 1400, height: 900 });
    unmount();
  });

  it('does not save when the settled size equals the persisted one', async () => {
    const { unmount } = withSetup(() => useWindowSize());
    await settle();

    resizeTo(1500, 900);
    resizeTo(1280, 860);
    await settle();

    expect(saveWindowSize).not.toHaveBeenCalled();
    unmount();
  });

  it('does not save the size of a maximised window', async () => {
    vi.mocked(isWindowMaximized).mockResolvedValue(true);
    const { result, unmount } = withSetup(() => useWindowSize());
    await settle();

    resizeTo(2560, 1400);
    await settle();

    expect(saveWindowSize).not.toHaveBeenCalled();
    expect(result.persisted.value).toEqual({ width: 1280, height: 860 });
    unmount();
  });

  it('reports a failed save as an error message and keeps the last persisted size', async () => {
    vi.mocked(saveWindowSize).mockRejectedValueOnce({ message: 'Check that the folder is writable.', logPath: null });
    const { result, unmount } = withSetup(() => useWindowSize());
    await settle();

    resizeTo(1000, 700);
    await settle();

    expect(result.error.value).toBe('Check that the folder is writable.');
    expect(result.persisted.value).toEqual({ width: 1280, height: 860 });
    unmount();
  });

  it('reports a failed config load and still persists later resizes', async () => {
    vi.mocked(loadConfig).mockRejectedValueOnce({ message: 'Update layoutswap.', logPath: null });
    const { result, unmount } = withSetup(() => useWindowSize());
    await settle();
    expect(result.error.value).toBe('Update layoutswap.');

    resizeTo(1100, 800);
    await settle();
    expect(saveWindowSize).toHaveBeenCalledWith({ width: 1100, height: 800 });
    unmount();
  });

  it('stops listening once its scope is disposed', async () => {
    const { unmount } = withSetup(() => useWindowSize());
    await settle();
    unmount();

    resizeTo(1600, 1000);
    await settle();
    expect(saveWindowSize).not.toHaveBeenCalled();
  });
});
