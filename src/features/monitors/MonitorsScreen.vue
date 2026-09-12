<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, ref } from 'vue';
import { monitorRows } from '@/domain/monitorRows';
import { useLayoutsStore } from '@/features/layouts/layouts.store';
import Button from '@/ui/Button.vue';
import MonitorRow from './MonitorRow.vue';
import { useMonitorsStore } from './monitors.store';

const layoutsStore = useLayoutsStore();
const monitorsStore = useMonitorsStore();
const { inventory, layouts, aliases, inputSources, probing, probeError, aliasError, lastProbe } = storeToRefs(layoutsStore);
const { capabilities, reading, readError } = storeToRefs(monitorsStore);

// Which rows show their capabilities, and which alias is being edited.
const expanded = ref(new Set<string>());
const editing = ref<string | null>(null);

const connectedCount = computed(() => inventory.value?.monitors.filter((m) => m.state !== 'Absent').length ?? 0);
const subtitle = computed(() => `${connectedCount.value === 1 ? 'One monitor' : `${connectedCount.value} monitors`} connected. Aliases are yours; every other column is read from the hardware.`);
const error = computed(() => probeError.value ?? readError.value ?? aliasError.value);

const rows = computed(() => monitorRows({
  inventory: inventory.value,
  layouts: layouts.value,
  aliases: aliases.value,
  inputSources: inputSources.value,
  capabilities: capabilities.value,
  reading: reading.value,
  expanded: expanded.value,
}));

function toggle(devicePath: string) {
  const next = new Set(expanded.value);
  if (!next.delete(devicePath)) {
    next.add(devicePath);
  }
  expanded.value = next;
}

// Enter saves and the field's blur follows it; the guard makes the second call a
// no-op, and an alias left as it was is not saved at all.
async function saveAlias(devicePath: string, alias: string) {
  if (editing.value !== devicePath) {
    return;
  }
  editing.value = null;
  if (alias.trim() === (rows.value.find((r) => r.devicePath === devicePath)?.alias ?? '')) {
    return;
  }
  await layoutsStore.renameMonitor(devicePath, alias);
}
</script>

<template>
  <article class="monitors">
    <header class="header">
      <div class="title">
        <h2>Monitors</h2>
        <p class="subtitle">
          {{ subtitle }}
        </p>
      </div>
      <div class="actions">
        <span class="last-probe data">Last probe {{ lastProbe ?? 'not run yet' }}</span>
        <Button class="refresh" :disabled="probing" @click="layoutsStore.probe">
          Refresh
        </Button>
        <Button class="re-check" :disabled="reading || probing" @click="monitorsStore.reCheck">
          <span v-if="reading" class="spinner" aria-hidden="true" />
          {{ reading ? 'Re-checking' : 'Re-check' }}
        </Button>
      </div>
    </header>
    <p v-if="error" class="alert" role="alert">
      {{ error }}
    </p>
    <p v-if="!inventory" class="empty">
      {{ probing ? 'Reading the monitors' : 'The monitors have not been read yet. Press Refresh.' }}
    </p>
    <ul v-else class="rows">
      <MonitorRow
        v-for="row in rows"
        :key="row.devicePath"
        :row="row"
        :editing="editing === row.devicePath"
        @toggle="toggle(row.devicePath)"
        @edit="editing = row.devicePath"
        @save="saveAlias(row.devicePath, $event)"
        @cancel="editing = null"
      />
    </ul>
  </article>
</template>

<style scoped>
.monitors {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
  overflow: auto;
}

.header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-6) var(--space-7) var(--space-4);
  border-bottom: var(--hairline) solid var(--line);
}

.title h2 {
  margin: 0;
  font-family: var(--display);
  font-size: var(--text-2xl);
  font-weight: 600;
  letter-spacing: var(--tracking-tight);
}

.subtitle {
  margin: var(--space-1) 0 0;
  font-size: var(--text-md);
  color: var(--ink-2);
}

.actions {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex: none;
}

.last-probe {
  font-size: var(--text-xs);
  color: var(--ink-3);
}

.data {
  font-family: var(--data);
}

.spinner {
  display: inline-block;
  width: 10px;
  height: 10px;
  margin-right: var(--space-2);
  border: 2px solid var(--accent-line);
  border-top-color: var(--accent);
  border-radius: var(--r-round);
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.alert {
  margin: 0;
  padding: var(--space-3) var(--space-7);
  border-bottom: var(--hairline) solid var(--crit);
  background: var(--crit-soft);
  color: var(--crit);
  font-size: var(--text-md);
}

.empty {
  margin: 0;
  padding: var(--space-6) var(--space-7);
  color: var(--ink-2);
}

.rows {
  margin: 0;
  padding: 0;
  list-style: none;
}
</style>
