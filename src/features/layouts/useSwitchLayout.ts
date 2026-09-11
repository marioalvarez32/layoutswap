import { computed, ref, toValue, type MaybeRefOrGetter } from 'vue';
import { errorMessage } from '@/domain/errors';
import { useLayoutsStore } from './layouts.store';

/**
 * The Switch action of one layout's detail: starts the switch through the store, is
 * blocked while another switch runs, and keeps a refusal where the button is.
 */
export function useSwitchLayout(layoutId: MaybeRefOrGetter<string>) {
  const store = useLayoutsStore();

  const busy = ref(false);
  const error = ref<string | null>(null);

  /** Why Switch is disabled, or null when it can start. */
  const blockedBy = computed(() => {
    const running = store.switching;
    if (!running) {
      return null;
    }
    return `Wait for the switch to ${running.layoutName} to finish.`;
  });

  /** Starts the switch and resolves when the script has exited. A refusal lands in `error`. */
  async function start() {
    if (busy.value || blockedBy.value) {
      return;
    }
    busy.value = true;
    error.value = null;
    try {
      await store.switchTo(toValue(layoutId));
    } catch (cause) {
      error.value = errorMessage(cause);
    } finally {
      busy.value = false;
    }
  }

  return { start, busy, error, blockedBy };
}
