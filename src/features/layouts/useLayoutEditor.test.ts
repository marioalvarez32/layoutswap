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

  it('sets the drop wait and shows its rule out of bounds', () => {
    const { result } = withSetup(() => useLayoutEditor('layout-desk'));
    result.setDropWait(9);
    expect(result.edits.value.dropWaitSeconds).toBe(9);
    expect(result.dirty.value).toBe(true);
    expect(result.dropWaitRule.value).toBeNull();
    result.setDropWait(61);
    expect(result.dropWaitRule.value).toContain('between 0 and 60');
    result.setAvailableWait(90);
    expect(result.edits.value.availableWaitSeconds).toBe(90);
    expect(result.availableWaitRule.value).toBeNull();
    result.setAvailableWait(0);
    expect(result.availableWaitRule.value).toContain('between 1 and 600');
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
    result.setSteps([{ id: 's1', side: 'before', kind: 'wait', seconds: 0 }]);
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
