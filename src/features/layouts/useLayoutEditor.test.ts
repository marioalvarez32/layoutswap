import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { newWaitStep } from '@/domain/steps';
import { saveLayout, scriptStates } from '@/tauri/commands';
import { layoutFixture } from '@/test/fixtures';
import { withSetup } from '@/test/withSetup';
import { useLayoutsStore } from './layouts.store';
import { useLayoutEditor } from './useLayoutEditor';

vi.mock('@/tauri/commands', async () => (await import('@/test/commands')).commandsMock());

describe('useLayoutEditor', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    const store = useLayoutsStore();
    store.layouts = [layoutFixture()];
    store.selectedId = 'layout-desk';
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('starts clean from the layout and turns dirty when the steps change', () => {
    const { result } = withSetup(() => useLayoutEditor('layout-desk'));
    expect(result.dirty.value).toBe(false);
    expect(result.edits.value.steps).toEqual([]);
    result.setSteps([newWaitStep('before', 's1')]);
    expect(result.dirty.value).toBe(true);
    expect(result.edits.value.steps[0]?.id).toBe('s1');
  });

  it('adds a wait step with a fresh id on the side asked', () => {
    const { result } = withSetup(() => useLayoutEditor('layout-desk'));
    const id = result.addStep('after');
    expect(id.length).toBeGreaterThan(8);
    expect(result.edits.value.steps).toEqual([{ id, side: 'after', kind: 'wait', seconds: 3 }]);
    expect(result.addStep('after')).not.toBe(id);
  });

  it('saves the draft through the command and picks up the stored layout', async () => {
    const step = newWaitStep('before', 's1');
    vi.mocked(saveLayout).mockResolvedValueOnce({ ...layoutFixture(), steps: [step], updatedAt: '2026-09-10T20:00:00-05:00' });
    vi.mocked(scriptStates).mockResolvedValueOnce([{ layoutId: 'layout-desk', state: 'current', path: 'C:/x/switch.ps1' }]);
    const { result } = withSetup(() => useLayoutEditor('layout-desk'));
    result.setSteps([step]);
    await result.save();
    expect(saveLayout).toHaveBeenCalledWith('layout-desk', { steps: [step], dropWaitSeconds: 5, availableWaitSeconds: 120, onApplyFailure: 'stop' });
    expect(result.dirty.value).toBe(false);
    expect(useLayoutsStore().layouts[0]?.steps).toEqual([step]);
    expect(result.error.value).toBeNull();
  });

  it('keeps the draft and the refusal when the save is refused', async () => {
    vi.mocked(saveLayout).mockRejectedValueOnce({ message: 'Keep a wait step between 1 and 600 seconds.', logPath: null });
    const { result } = withSetup(() => useLayoutEditor('layout-desk'));
    result.setSteps([{ ...newWaitStep('before', 's1'), seconds: 0 }]);
    await result.save();
    expect(result.error.value).toContain('between 1 and 600');
    expect(result.dirty.value).toBe(true);
    expect(result.saving.value).toBe(false);
  });

  it('discards back to the layout', () => {
    const { result } = withSetup(() => useLayoutEditor('layout-desk'));
    result.setSteps([newWaitStep('after', 's1')]);
    result.discard();
    expect(result.dirty.value).toBe(false);
    expect(result.edits.value.steps).toEqual([]);
  });

  it('does nothing when there is nothing to save', async () => {
    const { result } = withSetup(() => useLayoutEditor('layout-desk'));
    await result.save();
    expect(saveLayout).not.toHaveBeenCalled();
  });
});
