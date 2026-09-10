import { computed, ref, watch } from 'vue';
import { errorMessage } from '@/domain/errors';
import type { Layout } from '@/domain/generated/types';
import { checkLayoutName, type NamedLayout } from '@/domain/layout-name';
import { useLayoutsStore } from './layouts.store';

/**
 * The Save current layout flow: the name being typed, its check against the rules,
 * the replace confirmation when the name belongs to another layout, and the capture
 * call. `onSaved` receives the stored layout.
 */
export function useSaveLayout(options: { onSaved: (layout: Layout) => void }) {
  const store = useLayoutsStore();

  const name = ref('');
  const conflict = ref<NamedLayout | null>(null);
  const error = ref<string | null>(null);
  const saving = ref(false);

  const check = computed(() => checkLayoutName(name.value, store.layouts));
  const canSave = computed(() => check.value.ok && !saving.value && store.inventory !== null);

  /** Saves, or asks first when the name belongs to another layout. */
  async function save() {
    if (!canSave.value) {
      return;
    }
    if (check.value.conflict) {
      conflict.value = check.value.conflict;
      return;
    }
    await submit(null);
  }

  async function confirmReplace() {
    if (!conflict.value) {
      return;
    }
    await submit(conflict.value.id);
  }

  function cancelReplace() {
    conflict.value = null;
  }

  async function submit(replaceId: string | null) {
    saving.value = true;
    error.value = null;
    try {
      const outcome = await store.capture(check.value.name, replaceId);
      if (outcome.outcome === 'saved') {
        conflict.value = null;
        options.onSaved(outcome.layout);
      } else {
        // Rust saw a layout this renderer did not know about yet.
        conflict.value = { id: outcome.id, name: outcome.name };
      }
    } catch (cause) {
      error.value = errorMessage(cause);
    } finally {
      saving.value = false;
    }
  }

  // Editing the name withdraws the replace question. Synchronous, so a question raised
  // by save() right after typing is not cleared by the pending watcher.
  watch(name, () => {
    conflict.value = null;
  }, { flush: 'sync' });

  return { name, check, canSave, conflict, error, saving, save, confirmReplace, cancelReplace };
}
