import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { saveLayout } from '@/tauri/commands';
import { layoutFixture } from '@/test/fixtures';
import { withSetup } from '@/test/withSetup';
import { useLayoutsStore } from './layouts.store';
import { useUnsavedGuard } from './useUnsavedGuard';

vi.mock('@/tauri/commands', async () => (await import('@/test/commands')).commandsMock());

function dirtyStore() {
  const store = useLayoutsStore();
  store.edit('layout-desk', { ...store.editsFor('layout-desk'), dropWaitSeconds: 9 });
  return store;
}

describe('useUnsavedGuard', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    const store = useLayoutsStore();
    store.layouts = [layoutFixture()];
    store.selectedId = 'layout-desk';
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('runs the move at once when nothing is dirty', () => {
    const move = vi.fn();
    const { result } = withSetup(() => useUnsavedGuard());
    result.guard(move);
    expect(move).toHaveBeenCalledTimes(1);
    expect(result.asking.value).toBe(false);
  });

  it('holds the move and names the layout while dirty, then saves and moves', async () => {
    dirtyStore();
    const move = vi.fn();
    const { result } = withSetup(() => useUnsavedGuard());
    result.guard(move);
    expect(move).not.toHaveBeenCalled();
    expect(result.asking.value).toBe(true);
    expect(result.layoutName.value).toBe('Desk');
    await result.save();
    expect(saveLayout).toHaveBeenCalledWith('layout-desk', expect.objectContaining({ dropWaitSeconds: 9 }));
    expect(move).toHaveBeenCalledTimes(1);
    expect(result.asking.value).toBe(false);
    expect(useLayoutsStore().dirty).toBe(false);
  });

  it('discards and moves, or keeps editing and drops the move', () => {
    dirtyStore();
    const move = vi.fn();
    const { result } = withSetup(() => useUnsavedGuard());
    result.guard(move);
    result.keep();
    expect(move).not.toHaveBeenCalled();
    expect(result.asking.value).toBe(false);
    expect(useLayoutsStore().dirty).toBe(true);

    result.guard(move);
    result.discard();
    expect(move).toHaveBeenCalledTimes(1);
    expect(useLayoutsStore().dirty).toBe(false);
  });

  it('keeps the dialog open with the reason when the save is refused', async () => {
    dirtyStore();
    vi.mocked(saveLayout).mockRejectedValueOnce({ message: 'Keep the drop wait between 0 and 60 seconds.', logPath: null });
    const move = vi.fn();
    const { result } = withSetup(() => useUnsavedGuard());
    result.guard(move);
    await result.save();
    expect(result.error.value).toContain('drop wait');
    expect(result.asking.value).toBe(true);
    expect(move).not.toHaveBeenCalled();
    expect(result.saving.value).toBe(false);
  });
});
