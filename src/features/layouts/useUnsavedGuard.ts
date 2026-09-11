import { computed, ref } from 'vue';
import { errorMessage } from '@/domain/errors';
import { useLayoutsStore } from './layouts.store';

/**
 * Leaving a layout with unsaved steps asks first. `guard` runs a move at once when
 * nothing is dirty, else holds it until the user answers Save, Discard or Keep editing.
 * A refused save keeps the dialog open with the reason.
 */
export function useUnsavedGuard() {
  const store = useLayoutsStore();

  const pendingMove = ref<(() => void) | null>(null);
  const saving = ref(false);
  const error = ref<string | null>(null);

  const asking = computed(() => pendingMove.value !== null);
  const layoutName = computed(() => store.dirtyLayoutName);

  function guard(move: () => void) {
    if (store.dirty) {
      error.value = null;
      pendingMove.value = move;
    } else {
      move();
    }
  }

  async function save() {
    if (saving.value) {
      return;
    }
    saving.value = true;
    try {
      await store.saveDraft();
      const move = pendingMove.value;
      pendingMove.value = null;
      move?.();
    } catch (cause) {
      error.value = errorMessage(cause);
    } finally {
      saving.value = false;
    }
  }

  function discard() {
    store.discardDraft();
    const move = pendingMove.value;
    pendingMove.value = null;
    move?.();
  }

  function keep() {
    pendingMove.value = null;
  }

  return { asking, layoutName, saving, error, guard, save, discard, keep };
}
