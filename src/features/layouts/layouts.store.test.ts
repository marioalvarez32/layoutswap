import { flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { SwitchEvent, SwitchResult } from '@/domain/generated/types';
import { cancelSwitch, onSwitchEvent, openDisplaySettings, openLog, saveDiagnostics, switchLayout } from '@/tauri/commands';
import { layoutFixture } from '@/test/fixtures';
import { useLayoutsStore } from './layouts.store';

vi.mock('@/tauri/commands', async () => (await import('@/test/commands')).commandsMock());

const STARTED: SwitchEvent = { kind: 'started', layoutId: 'layout-desk', steps: ['Check monitors', 'Apply arrangement', 'Verify'], applyStep: 2 };

/** Wires the mocks so a test can push events and end the script when it wants. */
function scriptedSwitch() {
  let emit: (event: SwitchEvent) => void = () => {};
  const unlisten = vi.fn();
  vi.mocked(onSwitchEvent).mockImplementation(async (handler) => {
    emit = handler;
    return unlisten;
  });
  let finish!: (result: SwitchResult) => void;
  vi.mocked(switchLayout).mockImplementation(() => new Promise<SwitchResult>((resolve) => {
    finish = resolve;
  }));
  return { emit: (event: SwitchEvent) => emit(event), finish: (result: SwitchResult) => finish(result), unlisten };
}

describe('layouts store: switch', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.useFakeTimers();
    vi.setSystemTime(10_000);
    const store = useLayoutsStore();
    store.layouts = [layoutFixture(), { ...layoutFixture(), id: 'layout-film', name: 'Film', folder: 'film' }];
    store.selectedId = 'layout-desk';
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  it('opens the run on the started event, follows progress and log, and keeps the result', async () => {
    const store = useLayoutsStore();
    const script = scriptedSwitch();
    const pending = store.switchTo('layout-desk');
    await flushPromises();
    expect(onSwitchEvent).toHaveBeenCalledTimes(1);
    expect(switchLayout).toHaveBeenCalledWith('layout-desk');
    expect(store.switchRun).toBeNull();
    expect(store.switching).toBeNull();

    script.emit(STARTED);
    expect(store.switchRun).toMatchObject({ layoutId: 'layout-desk', layoutName: 'Desk', startedAt: 10_000, applyStep: 2 });
    expect(store.switching).toEqual({ layoutId: 'layout-desk', layoutName: 'Desk' });
    expect(store.switchRun!.steps.map((s) => s.name)).toEqual(['Check monitors', 'Apply arrangement', 'Verify']);

    script.emit({ kind: 'log', text: '=== Switch to Desk ===' });
    script.emit({ kind: 'progress', line: { step: 1, of: 3, status: 'running', text: 'Check monitors' } });
    script.emit({ kind: 'log', text: '  Side: Available' });
    script.emit({ kind: 'progress', line: { step: 1, of: 3, status: 'done', text: 'Check monitors' } });
    expect(store.switchRun!.steps[0]).toMatchObject({ status: 'done' });
    expect(store.switchRun!.log).toEqual(['=== Switch to Desk ===', '  Side: Available']);

    vi.setSystemTime(21_400);
    script.finish({ outcome: 'applied', durationMs: 11_400 });
    await pending;
    expect(store.switchRun!.result).toEqual({ outcome: 'applied', durationMs: 11_400 });
    expect(store.switchRun!.finishedAt).toBe(21_400);
    expect(store.switching).toBeNull();
    expect(script.unlisten).toHaveBeenCalledTimes(1);
  });

  it('rethrows a refusal and shows no run', async () => {
    const store = useLayoutsStore();
    const unlisten = vi.fn();
    vi.mocked(onSwitchEvent).mockResolvedValue(unlisten);
    vi.mocked(switchLayout).mockRejectedValue({ message: 'Wait for the switch to Film to finish, then try again.', logPath: null });
    await expect(store.switchTo('layout-desk')).rejects.toMatchObject({ message: expect.stringContaining('Film') });
    expect(store.switchRun).toBeNull();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('ends the run as failed when the command rejects after the script started', async () => {
    const store = useLayoutsStore();
    const unlisten = vi.fn();
    let emit: (event: SwitchEvent) => void = () => {};
    vi.mocked(onSwitchEvent).mockImplementation(async (handler) => {
      emit = handler;
      return unlisten;
    });
    let reject!: (cause: unknown) => void;
    vi.mocked(switchLayout).mockImplementation(() => new Promise<SwitchResult>((_, r) => {
      reject = r;
    }));
    const pending = store.switchTo('layout-desk');
    await flushPromises();
    emit(STARTED);
    reject({ message: 'Open layoutswap again and try once more: something inside the app failed (x).', logPath: null });
    await expect(pending).resolves.toBeUndefined();
    expect(store.switchRun!.result).toMatchObject({ outcome: 'failed', step: null, nextAction: expect.stringContaining('Open layoutswap again') });
    expect(store.switching).toBeNull();
    expect(unlisten).toHaveBeenCalledTimes(1);
  });

  it('cancels through the command and resolves the run as cancelled', async () => {
    const store = useLayoutsStore();
    const script = scriptedSwitch();
    const pending = store.switchTo('layout-desk');
    await flushPromises();
    script.emit(STARTED);

    await store.cancelSwitch();
    expect(cancelSwitch).toHaveBeenCalledTimes(1);
    expect(store.switchRun!.cancelling).toBe(true);

    script.finish({ outcome: 'cancelled' });
    await pending;
    expect(store.switchRun!.result).toEqual({ outcome: 'cancelled' });
    expect(store.switchRun!.cancelling).toBe(false);
  });

  it('keeps a refused cancel on the run and leaves it running', async () => {
    const store = useLayoutsStore();
    const script = scriptedSwitch();
    void store.switchTo('layout-desk');
    await flushPromises();
    script.emit(STARTED);
    script.emit({ kind: 'progress', line: { step: 2, of: 3, status: 'running', text: 'Apply arrangement' } });
    vi.mocked(cancelSwitch).mockRejectedValueOnce({ message: 'Wait for the switch to Desk to finish: the arrangement is already being applied and cannot be stopped.', logPath: null });

    await store.cancelSwitch();
    expect(store.switchRun!.error).toContain('cannot be stopped');
    expect(store.switchRun!.cancelling).toBe(false);
    expect(store.switchRun!.result).toBeNull();
  });

  it('runs the result actions against the finished run and keeps what they said', async () => {
    const store = useLayoutsStore();
    const script = scriptedSwitch();
    const pending = store.switchTo('layout-desk');
    await flushPromises();
    script.emit(STARTED);
    await store.resultAction('openLog');
    expect(openLog).not.toHaveBeenCalled();

    script.finish({ outcome: 'failed', step: 1, stepName: 'Check monitors', nextAction: 'x', reason: 'y', exitCode: 2, logPath: 'C:/x/switch.log', explanation: { kind: 'absent', monitors: ['Ultrawide'] } });
    await pending;
    await store.resultAction('openLog');
    expect(openLog).toHaveBeenCalledWith('layout-desk');
    await store.resultAction('openDisplaySettings');
    expect(openDisplaySettings).toHaveBeenCalledTimes(1);
    await store.resultAction('saveDiagnostics');
    expect(saveDiagnostics).toHaveBeenCalledWith('layout-desk');
    expect(store.switchRun!.notice).toBe('Diagnostics saved to C:/Users/x/Desktop/layoutswap-diagnostics-desk-20260910-183012.zip');
    expect(store.resultActionBusy).toBe(false);

    vi.mocked(saveDiagnostics).mockResolvedValueOnce(null);
    await store.resultAction('saveDiagnostics');
    expect(store.switchRun!.notice).toBeNull();

    vi.mocked(openLog).mockRejectedValueOnce({ message: 'Switch to the layout once, then open its log: C:/x/switch.log is not on disk yet.', logPath: null });
    await store.resultAction('openLog');
    expect(store.switchRun!.error).toContain('Switch to the layout once');
  });

  it('does nothing when there is no run to cancel', async () => {
    const store = useLayoutsStore();
    await store.cancelSwitch();
    expect(cancelSwitch).not.toHaveBeenCalled();
  });

  it('drops a finished run on Back or when another layout is selected, never a running one', async () => {
    const store = useLayoutsStore();
    const script = scriptedSwitch();
    const pending = store.switchTo('layout-desk');
    await flushPromises();
    script.emit(STARTED);
    store.select('layout-film');
    expect(store.switchRun).not.toBeNull();
    store.select('layout-desk');

    script.finish({ outcome: 'applied', durationMs: 5_000 });
    await pending;
    store.dismissSwitch();
    expect(store.switchRun).toBeNull();

    const again = scriptedSwitch();
    const secondRun = store.switchTo('layout-desk');
    await flushPromises();
    again.emit(STARTED);
    again.finish({ outcome: 'cancelled' });
    await secondRun;
    store.select('layout-film');
    expect(store.switchRun).toBeNull();
  });
});
