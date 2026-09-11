import { computed, ref, toValue, type MaybeRefOrGetter } from 'vue';
import { errorMessage } from '@/domain/errors';
import { scriptIndicator, stateOf } from '@/domain/script-state';
import { openLog, openScript } from '@/tauri/commands';
import { useLayoutsStore } from './layouts.store';

/**
 * The generated script of one layout as the detail shows it: the indicator line, and
 * the Open script, Open log, Regenerate script and Re-capture arrangement actions
 * with their in-flight and error state.
 */
export function useLayoutScript(layoutId: MaybeRefOrGetter<string>) {
  const store = useLayoutsStore();

  const busy = ref(false);
  const error = ref<string | null>(null);

  const indicator = computed(() => scriptIndicator(stateOf(store.scriptStatuses, toValue(layoutId))));

  async function open() {
    await run(() => openScript(toValue(layoutId)));
  }

  async function openSwitchLog() {
    await run(() => openLog(toValue(layoutId)));
  }

  async function regenerate() {
    await run(() => store.regenerateScript(toValue(layoutId)));
  }

  /** Re-captures the arrangement Windows shows now into this layout, steps kept. */
  async function recapture() {
    await run(() => store.recapture(toValue(layoutId)));
  }

  async function run(action: () => Promise<void>) {
    if (busy.value) {
      return;
    }
    busy.value = true;
    error.value = null;
    try {
      await action();
    } catch (cause) {
      error.value = errorMessage(cause);
    } finally {
      busy.value = false;
    }
  }

  return { indicator, busy, error, open, openSwitchLog, regenerate, recapture };
}
