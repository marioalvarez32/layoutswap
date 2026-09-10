import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { openScript, regenerateScript, scriptStates } from '@/tauri/commands';
import { layoutFixture } from '@/test/fixtures';
import { withSetup } from '@/test/withSetup';
import { useLayoutsStore } from './layouts.store';
import { useLayoutScript } from './useLayoutScript';

vi.mock('@/tauri/commands', async () => (await import('@/test/commands')).commandsMock());

describe('useLayoutScript', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    const store = useLayoutsStore();
    store.layouts = [layoutFixture()];
    store.selectedId = 'layout-desk';
    store.scriptStatuses = [{ layoutId: 'layout-desk', state: 'stale', path: 'C:/x/switch.ps1' }];
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('shows the indicator for the layout it was given, selected or not', () => {
    const { result } = withSetup(() => useLayoutScript('layout-desk'));
    expect(result.indicator.value?.tone).toBe('warn');
    const other = withSetup(() => useLayoutScript('layout-other'));
    expect(other.result.indicator.value).toBeNull();
  });

  it('regenerates and picks up the fresh state', async () => {
    vi.mocked(scriptStates).mockResolvedValue([{ layoutId: 'layout-desk', state: 'current', path: 'C:/x/switch.ps1' }]);
    const { result } = withSetup(() => useLayoutScript('layout-desk'));
    await result.regenerate();
    expect(regenerateScript).toHaveBeenCalledWith('layout-desk');
    expect(result.indicator.value).toEqual({ tone: 'good', text: 'Script up to date' });
    expect(result.error.value).toBeNull();
  });

  it('opens the script', async () => {
    const { result } = withSetup(() => useLayoutScript('layout-desk'));
    await result.open();
    expect(openScript).toHaveBeenCalledWith('layout-desk');
  });

  it('reports a failure where it happened', async () => {
    vi.mocked(openScript).mockRejectedValueOnce({ message: 'Regenerate the script from the layout, then try again.', logPath: null });
    const { result } = withSetup(() => useLayoutScript('layout-desk'));
    await result.open();
    expect(result.error.value).toContain('Regenerate the script');
    expect(result.busy.value).toBe(false);
  });
});
