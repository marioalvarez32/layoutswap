import { onScopeDispose, ref, toValue, watch, type MaybeRefOrGetter } from 'vue';

/**
 * The milliseconds since `startedAt`, ticking while `running` is true and frozen
 * when it turns false. The timer is cleared with the scope.
 */
export function useElapsed(
  startedAt: MaybeRefOrGetter<number>,
  running: MaybeRefOrGetter<boolean>,
  options: { tickMs?: number; now?: () => number } = {},
) {
  const tickMs = options.tickMs ?? 100;
  const now = options.now ?? Date.now;

  const elapsedMs = ref(0);
  let timer: ReturnType<typeof setInterval> | null = null;

  function update() {
    elapsedMs.value = Math.max(0, now() - toValue(startedAt));
  }

  function stop() {
    if (timer !== null) {
      clearInterval(timer);
      timer = null;
    }
  }

  watch(
    () => toValue(running),
    (isRunning) => {
      stop();
      update();
      if (isRunning) {
        timer = setInterval(update, tickMs);
      }
    },
    { immediate: true },
  );
  watch(() => toValue(startedAt), update);

  onScopeDispose(stop);

  return { elapsedMs };
}
