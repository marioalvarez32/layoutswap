<script setup lang="ts">
import { onMounted, ref, watch } from 'vue';
import Button from '@/ui/Button.vue';

const props = defineProps<{
  open: boolean;
  layoutName: string;
  /** Save is in flight. */
  saving?: boolean;
  /** A save the app refused; the dialog stays so the user can keep editing. */
  error?: string | null;
}>();

const emit = defineEmits<{
  save: [];
  discard: [];
  keep: [];
}>();

const dialog = ref<HTMLDialogElement | null>(null);

// A native dialog is modal only through showModal(); where the host lacks it (tests),
// the open attribute still shows it.
function sync() {
  const el = dialog.value;
  if (!el) {
    return;
  }
  if (props.open && !el.open) {
    if (typeof el.showModal === 'function') {
      el.showModal();
    } else {
      el.setAttribute('open', '');
    }
  } else if (!props.open && el.open) {
    if (typeof el.close === 'function') {
      el.close();
    } else {
      el.removeAttribute('open');
    }
  }
}

onMounted(sync);
watch(() => props.open, sync);
</script>

<template>
  <dialog ref="dialog" class="save-changes" @cancel.prevent="emit('keep')">
    <form method="dialog" class="body" @submit.prevent="emit('save')">
      <h3>Save changes to {{ layoutName }}?</h3>
      <p class="muted">
        The steps you changed are not saved. Saving regenerates the layout's script.
      </p>
      <p v-if="error" class="band crit" role="alert">
        {{ error }}
      </p>
      <div class="actions">
        <Button
          variant="primary"
          type="submit"
          class="save"
          :disabled="saving"
        >
          Save
        </Button>
        <Button class="discard" :disabled="saving" @click="emit('discard')">
          Discard
        </Button>
        <Button class="keep" :disabled="saving" @click="emit('keep')">
          Keep editing
        </Button>
      </div>
    </form>
  </dialog>
</template>

<style scoped>
.save-changes {
  width: var(--prose-w);
  max-width: 100%;
  padding: 0;
  border: var(--hairline) solid var(--line-strong);
  border-radius: var(--r-lg);
  background: var(--surface);
  color: var(--ink);
}

.save-changes::backdrop {
  background: var(--ink);
  opacity: 0.4;
}

.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-6);
}

h3 {
  margin: 0;
  font-family: var(--display);
  font-size: var(--text-lg);
  font-weight: 600;
}

.muted {
  margin: 0;
  font-size: var(--text-md);
  color: var(--ink-2);
}

.band {
  margin: 0;
  padding: var(--space-4);
  border: var(--hairline) solid;
  border-radius: var(--r);
  font-size: var(--text-md);
}

.band.crit {
  border-color: var(--crit);
  background: var(--crit-soft);
  color: var(--crit);
}

.actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
}
</style>
