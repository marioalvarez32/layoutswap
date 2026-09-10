import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';
import { captureLayout } from '@/tauri/commands';
import { inventoryFixture, layoutFixture } from '@/test/fixtures';
import { withSetup } from '@/test/withSetup';
import { useLayoutsStore } from './layouts.store';
import { useSaveLayout } from './useSaveLayout';

vi.mock('@/tauri/commands', async () => (await import('@/test/commands')).commandsMock());

async function flush() {
  await nextTick();
  await new Promise((resolve) => setTimeout(resolve, 0));
}

describe('useSaveLayout', () => {
  const onSaved = vi.fn();

  beforeEach(() => {
    setActivePinia(createPinia());
    const store = useLayoutsStore();
    store.inventory = inventoryFixture();
    store.layouts = [layoutFixture()];
    vi.mocked(captureLayout).mockResolvedValue({ outcome: 'saved', layout: { ...layoutFixture(), id: 'layout-new', name: 'Film' } });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('cannot save an empty name and says why', () => {
    const { result } = withSetup(() => useSaveLayout({ onSaved }));
    expect(result.canSave.value).toBe(false);
    expect(result.check.value.message).toBe('Give the layout a name.');
  });

  it('cannot save before the probe has run', () => {
    useLayoutsStore().inventory = null;
    const { result } = withSetup(() => useSaveLayout({ onSaved }));
    result.name.value = 'Film';
    expect(result.canSave.value).toBe(false);
  });

  it('captures a new name, selects it and reports it saved', async () => {
    const { result } = withSetup(() => useSaveLayout({ onSaved }));
    result.name.value = ' Film ';
    await result.save();
    expect(captureLayout).toHaveBeenCalledWith('Film', null);
    expect(onSaved).toHaveBeenCalledWith(expect.objectContaining({ id: 'layout-new', name: 'Film' }));
    const store = useLayoutsStore();
    expect(store.layouts.map((l) => l.name)).toEqual(['Desk', 'Film']);
    expect(store.selectedId).toBe('layout-new');
  });

  it('asks before replacing a layout that already has the name', async () => {
    const { result } = withSetup(() => useSaveLayout({ onSaved }));
    result.name.value = 'desk';
    await result.save();
    expect(captureLayout).not.toHaveBeenCalled();
    expect(result.conflict.value).toEqual({ id: 'layout-desk', name: 'Desk' });

    vi.mocked(captureLayout).mockResolvedValueOnce({ outcome: 'saved', layout: { ...layoutFixture(), name: 'desk' } });
    await result.confirmReplace();
    expect(captureLayout).toHaveBeenCalledWith('desk', 'layout-desk');
    expect(onSaved).toHaveBeenCalled();
    expect(result.conflict.value).toBeNull();
  });

  it('drops the replace question when the name changes or is cancelled', async () => {
    const { result } = withSetup(() => useSaveLayout({ onSaved }));
    result.name.value = 'Desk';
    await result.save();
    expect(result.conflict.value).not.toBeNull();
    result.cancelReplace();
    expect(result.conflict.value).toBeNull();

    await result.save();
    result.name.value = 'Desk 2';
    await flush();
    expect(result.conflict.value).toBeNull();
  });

  it('turns a conflict Rust reports into the replace question', async () => {
    vi.mocked(captureLayout).mockResolvedValueOnce({ outcome: 'nameTaken', id: 'other', name: 'Film' });
    const { result } = withSetup(() => useSaveLayout({ onSaved }));
    result.name.value = 'Film';
    await result.save();
    expect(result.conflict.value).toEqual({ id: 'other', name: 'Film' });
    expect(onSaved).not.toHaveBeenCalled();
  });

  it('shows a failed capture as an error and keeps the name', async () => {
    vi.mocked(captureLayout).mockRejectedValueOnce({ message: 'Wait for layoutswap to finish reading the monitors, then try again.', logPath: null });
    const { result } = withSetup(() => useSaveLayout({ onSaved }));
    result.name.value = 'Film';
    await result.save();
    expect(result.error.value).toContain('Wait for layoutswap');
    expect(result.name.value).toBe('Film');
    expect(result.saving.value).toBe(false);
  });
});
