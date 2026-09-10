<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, onMounted } from 'vue';
import { capturePreviewMonitors, capturePreviewRows } from '@/domain/capture';
import type { Layout } from '@/domain/generated/types';
import { describeProbeAge } from '@/domain/time';
import Button from '@/ui/Button.vue';
import Chip from '@/ui/Chip.vue';
import ArrangementSchematic from './ArrangementSchematic.vue';
import { useLayoutsStore } from './layouts.store';
import { useSaveLayout } from './useSaveLayout';

const emit = defineEmits<{
  cancel: [];
  saved: [layout: Layout];
}>();

const store = useLayoutsStore();
const { inventory, aliases, probing, probeError } = storeToRefs(store);
const { name, check, canSave, conflict, error, saving, save, confirmReplace, cancelReplace } = useSaveLayout({
  onSaved: (layout) => emit('saved', layout),
});

const rows = computed(() => capturePreviewRows(inventory.value, aliases.value));
// The picture shows what the capture will record, so it is drawn from the same
// monitors the summary will hold, not from the raw probe.
const schematicMonitors = computed(() => capturePreviewMonitors(inventory.value));

const readAt = computed(() => (inventory.value ? describeProbeAge(inventory.value.probedAt) : 'reading from Windows'));
const switchLabel = computed(() => `"Switch to ${check.value.name || 'Desk'}"`);

onMounted(() => {
  void store.probe();
});
</script>

<template>
  <form class="page" @submit.prevent="save">
    <header class="header">
      <h2>Save current layout</h2>
    </header>

    <div class="body">
      <label class="field">
        <span class="label">Name</span>
        <input
          v-model="name"
          type="text"
          class="input"
          autocomplete="off"
          :aria-invalid="!check.ok && name.length > 0 ? 'true' : undefined"
        >
        <span v-if="!check.ok && name.length > 0" class="rule" role="alert">{{ check.message }}</span>
        <span v-else class="hint">Used on the shortcut and in {{ switchLabel }}.</span>
      </label>

      <div
        v-if="conflict"
        class="band warn"
        role="alertdialog"
        aria-label="Replace layout"
      >
        <p class="band-text">
          A layout called {{ conflict.name }} already exists. Replace it with the arrangement Windows shows now?
        </p>
        <div class="band-actions">
          <Button variant="primary" :disabled="saving" @click="confirmReplace">
            Replace {{ conflict.name }}
          </Button>
          <Button @click="cancelReplace">
            Keep it
          </Button>
        </div>
      </div>

      <p v-if="error" class="band crit" role="alert">
        {{ error }}
      </p>
      <p v-if="probeError" class="band crit" role="alert">
        {{ probeError }}
      </p>

      <section class="capture">
        <div class="capture-head">
          <span class="label">What will be captured</span>
          <span class="read-at">{{ probing ? 'reading from Windows' : readAt }}</span>
        </div>
        <ArrangementSchematic
          :monitors="schematicMonitors"
          :aliases="aliases"
          empty-text="No monitor is on"
        />
        <div class="table">
          <div v-for="row in rows" :key="row.devicePath" class="row">
            <div class="who">
              <Chip :tone="row.on ? 'accent' : 'mute'">
                {{ row.on ? 'On' : 'Off' }}
              </Chip>
              <span class="name">{{ row.label }}</span>
              <span class="detail">{{ row.detail }}</span>
            </div>
            <div class="data">
              {{ row.size }}
            </div>
            <div class="data">
              {{ row.position }}
            </div>
            <div class="primary">
              {{ row.primary ? 'Primary' : '' }}
            </div>
          </div>
          <p v-if="rows.length === 0 && !probing" class="row-note">
            No monitors reported yet.
          </p>
          <p class="foot">
            Arrangement is edited in Windows Settings &gt; Display before saving.
          </p>
        </div>
      </section>
    </div>

    <footer class="actions">
      <Button @click="emit('cancel')">
        Cancel
      </Button>
      <Button variant="primary" type="submit" :disabled="!canSave">
        Save layout
      </Button>
    </footer>
  </form>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.header {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  padding: var(--space-6) var(--space-7) var(--space-5);
  border-bottom: var(--hairline) solid var(--line);
}

h2 {
  margin: 0;
  font-family: var(--display);
  font-size: var(--text-xl);
  font-weight: 600;
  letter-spacing: var(--tracking-tight);
}

.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  flex: 1;
  min-height: 0;
  max-width: var(--form-w);
  padding: var(--space-6) var(--space-7);
  overflow: auto;
}

.field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.label {
  font-size: var(--text-md);
  font-weight: 600;
}

.input {
  height: var(--control-h);
  padding: 0 var(--space-4);
  border: var(--hairline) solid var(--line-strong);
  border-radius: var(--r);
  background: var(--surface);
  font-size: var(--text-base);
}

.input:focus {
  border-color: var(--accent);
}

.hint {
  font-size: var(--text-sm);
  color: var(--ink-3);
}

.rule {
  font-size: var(--text-sm);
  color: var(--crit);
}

.band {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  margin: 0;
  padding: var(--space-4);
  border: var(--hairline) solid;
  border-radius: var(--r);
  font-size: var(--text-md);
}

.band.warn {
  border-color: var(--warn);
  background: var(--warn-soft);
}

.band.crit {
  border-color: var(--crit);
  background: var(--crit-soft);
  color: var(--crit);
}

.band-text {
  margin: 0;
}

.band-actions {
  display: flex;
  gap: var(--space-3);
}

.capture {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.capture-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
}

.read-at {
  font-family: var(--data);
  font-size: var(--text-xs);
  color: var(--ink-3);
}

.table {
  border: var(--hairline) solid var(--line);
  border-radius: var(--r);
  overflow: hidden;
}

.row {
  display: grid;
  grid-template-columns: 1fr var(--col-size) var(--col-position) var(--col-primary);
  gap: var(--space-4);
  align-items: center;
  padding: var(--space-4);
  border-bottom: var(--hairline) solid var(--line);
  background: var(--surface);
}

.who {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
}

.name {
  font-size: var(--text-md);
  font-weight: 500;
}

.detail {
  font-size: var(--text-sm);
  color: var(--ink-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.data {
  font-family: var(--data);
  font-size: var(--text-sm);
  color: var(--ink-2);
}

.primary {
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--accent);
}

.row-note {
  margin: 0;
  padding: var(--space-4);
  font-size: var(--text-sm);
  color: var(--ink-3);
}

.foot {
  margin: 0;
  padding: var(--space-3) var(--space-4);
  background: var(--surface-2);
  font-size: var(--text-sm);
  color: var(--ink-2);
}

.actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-3);
  padding: var(--space-5) var(--space-7);
  border-top: var(--hairline) solid var(--line);
  background: var(--surface-2);
}
</style>
