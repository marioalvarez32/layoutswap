import { nextTick, ref } from 'vue';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { withSetup } from '@/test/withSetup';
import { useElapsed } from './useElapsed';

describe('useElapsed', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(10_000);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('ticks while running and freezes when the run ends', async () => {
    const running = ref(true);
    const { result, unmount } = withSetup(() => useElapsed(8_000, running, { tickMs: 100 }));
    expect(result.elapsedMs.value).toBe(2_000);

    vi.advanceTimersByTime(350);
    expect(result.elapsedMs.value).toBe(2_300);

    running.value = false;
    await nextTick();
    const frozen = result.elapsedMs.value;
    vi.advanceTimersByTime(1_000);
    expect(result.elapsedMs.value).toBe(frozen);
    unmount();
  });

  it('stops its timer with the scope', () => {
    const { result, unmount } = withSetup(() => useElapsed(10_000, true, { tickMs: 100 }));
    unmount();
    vi.advanceTimersByTime(500);
    expect(result.elapsedMs.value).toBe(0);
  });

  it('restarts from a new start time', async () => {
    const startedAt = ref(9_000);
    const { result, unmount } = withSetup(() => useElapsed(startedAt, true, { tickMs: 100 }));
    expect(result.elapsedMs.value).toBe(1_000);
    startedAt.value = 10_000;
    await nextTick();
    expect(result.elapsedMs.value).toBe(0);
    unmount();
  });
});
