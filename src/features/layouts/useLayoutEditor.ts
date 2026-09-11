import { computed, ref, toValue, type MaybeRefOrGetter } from 'vue';
import { errorMessage } from '@/domain/errors';
import type { LayoutEdits, Step, StepSide } from '@/domain/generated/types';
import { addStep as appendStep, checkAvailableWait, checkDropWait, newWaitStep } from '@/domain/steps';
import { useLayoutsStore } from './layouts.store';

/**
 * The editable side of one layout's detail: the draft the store holds for it, whether
 * it is dirty, and Save and Discard with their in-flight and error state.
 */
export function useLayoutEditor(layoutId: MaybeRefOrGetter<string>) {
  const store = useLayoutsStore();

  const saving = ref(false);
  const error = ref<string | null>(null);

  const edits = computed(() => store.editsFor(toValue(layoutId)));
  const dirty = computed(() => store.isDraftDirty(toValue(layoutId)));

  /** Changes some of the edits, leaving the rest as they are. */
  function patch(changes: Partial<LayoutEdits>) {
    store.edit(toValue(layoutId), { ...edits.value, ...changes });
  }

  function setSteps(steps: Step[]) {
    patch({ steps });
  }

  /** Adds a wait step on a side and returns its id, so the editor can expand it. */
  function addStep(side: StepSide): string {
    const step = newWaitStep(side, newStepId());
    setSteps(appendStep(edits.value.steps, step));
    return step.id;
  }

  /** The rules the timing fields show while out of bounds. */
  const dropWaitRule = computed(() => checkDropWait(edits.value.dropWaitSeconds));
  const availableWaitRule = computed(() => checkAvailableWait(edits.value.availableWaitSeconds));

  /** Saves the draft; a refusal lands in `error` and the draft stays. */
  async function save() {
    if (saving.value || !dirty.value) {
      return;
    }
    saving.value = true;
    error.value = null;
    try {
      await store.saveDraft();
    } catch (cause) {
      error.value = errorMessage(cause);
    } finally {
      saving.value = false;
    }
  }

  function discard() {
    error.value = null;
    store.discardDraft();
  }

  return { edits, dirty, saving, error, patch, setSteps, addStep, dropWaitRule, availableWaitRule, save, discard };
}

/** A step id: random, so two steps never collide, the one effect this file holds. */
function newStepId(): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID().replace(/-/g, '');
  }
  return `${Date.now().toString(16)}${Math.floor(Math.random() * 0xffffffff).toString(16)}`;
}
