import { flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { newSwitchRun } from '@/domain/switch';
import { switchLayout } from '@/tauri/commands';
import { layoutFixture } from '@/test/fixtures';
import { withSetup } from '@/test/withSetup';
import { useLayoutsStore } from './layouts.store';
import { useSwitchLayout } from './useSwitchLayout';

vi.mock('@/tauri/commands', async () => (await import('@/test/commands')).commandsMock());

describe('useSwitchLayout', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    const store = useLayoutsStore();
    store.layouts = [layoutFixture(), { ...layoutFixture(), id: 'layout-film', name: 'Film', folder: 'film' }];
    store.selectedId = 'layout-desk';
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('starts the switch for the layout it was given', async () => {
    const { result } = withSetup(() => useSwitchLayout('layout-desk'));
    expect(result.blockedBy.value).toBeNull();
    await result.start();
    expect(switchLayout).toHaveBeenCalledWith('layout-desk');
    expect(result.error.value).toBeNull();
    expect(result.busy.value).toBe(false);
  });

  it('is blocked, by name, while another switch runs', async () => {
    const store = useLayoutsStore();
    store.switchRun = newSwitchRun({ id: 'layout-film', name: 'Film' }, ['Check monitors'], 2, Date.now());
    const { result } = withSetup(() => useSwitchLayout('layout-desk'));
    expect(result.blockedBy.value).toBe('Wait for the switch to Film to finish.');
    await result.start();
    expect(switchLayout).not.toHaveBeenCalled();

    store.switchRun.result = { outcome: 'cancelled', sent: [] };
    await flushPromises();
    expect(result.blockedBy.value).toBeNull();
  });

  it('is blocked while the layout has unsaved changes', async () => {
    const store = useLayoutsStore();
    store.edit('layout-desk', { ...store.editsFor('layout-desk'), dropWaitSeconds: 9 });
    const { result } = withSetup(() => useSwitchLayout('layout-desk'));
    expect(result.blockedBy.value).toBe('Save the layout first.');
    await result.start();
    expect(switchLayout).not.toHaveBeenCalled();
    expect(withSetup(() => useSwitchLayout('layout-film')).result.blockedBy.value).toBeNull();
  });

  it('keeps a refusal where the button is', async () => {
    vi.mocked(switchLayout).mockRejectedValueOnce({ message: 'Wait for the switch to Film to finish, then try again.', logPath: null });
    const { result } = withSetup(() => useSwitchLayout('layout-desk'));
    await result.start();
    expect(result.error.value).toContain('Wait for the switch to Film');
    expect(result.busy.value).toBe(false);
  });
});
