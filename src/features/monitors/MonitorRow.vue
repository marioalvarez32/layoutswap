<script setup lang="ts">
import { nextTick, ref, watch } from 'vue';
import type { MonitorRowView } from '@/domain/monitorRows';
import Chip from '@/ui/Chip.vue';

const props = defineProps<{
  row: MonitorRowView;
  editing: boolean;
}>();

const emit = defineEmits<{
  toggle: [];
  edit: [];
  save: [alias: string];
  cancel: [];
}>();

const draft = ref(props.row.alias);
const field = ref<HTMLInputElement | null>(null);

watch(() => props.editing, async (editing) => {
  if (editing) {
    draft.value = props.row.alias;
    await nextTick();
    field.value?.focus();
    field.value?.select();
  }
});
</script>

<template>
  <li class="monitor-row" :class="{ expanded: row.expanded }">
    <div class="head">
      <Chip :tone="row.tone" :title="row.stateNote">
        {{ row.stateText }}
      </Chip>
      <input
        v-if="editing"
        ref="field"
        v-model="draft"
        type="text"
        class="alias-field"
        aria-label="Alias"
        @keydown.enter.prevent="emit('save', draft)"
        @keydown.escape.prevent="emit('cancel')"
        @blur="emit('save', draft)"
      >
      <button
        v-else
        type="button"
        class="alias"
        :title="`Edit the alias of ${row.reportedName}`"
        @click="emit('edit')"
      >
        <span class="alias-text">{{ row.alias }}</span>
        <span class="alias-hint">edit</span>
      </button>
      <span class="reported">{{ row.reportedName }}</span>
      <span class="spacer" />
      <span class="note">{{ row.note }}</span>
    </div>

    <dl class="grid data">
      <div><dt>GPU</dt><dd>{{ row.gpu }}</dd></div>
      <div><dt>Connector</dt><dd>{{ row.connector }}</dd></div>
      <div><dt>Position</dt><dd>{{ row.position }}</dd></div>
      <div><dt>Size</dt><dd>{{ row.size }}</dd></div>
      <div>
        <dt>Current input source</dt>
        <dd>
          <Chip class="input" :tone="row.inputKnown ? 'accent' : 'mute'">
            {{ row.input }}
          </Chip>
        </dd>
      </div>
    </dl>

    <button
      v-if="row.canExpand"
      type="button"
      class="expand"
      @click="emit('toggle')"
    >
      <span class="caret" aria-hidden="true">{{ row.expanded ? '▼' : '▶' }}</span>
      {{ row.expanded ? 'Hide capabilities' : 'Show capabilities' }}
    </button>
    <p v-else-if="row.muted" class="muted" :class="{ reading: row.reading }">
      <span v-if="row.reading" class="spinner" aria-hidden="true" />
      {{ row.muted }}
    </p>

    <section v-if="row.canExpand && row.expanded" class="capabilities">
      <dl class="facts">
        <dt>Accepts</dt>
        <dd class="accepts">
          <Chip v-for="input in row.accepts" :key="input.name" :tone="input.current ? 'accent' : 'mute'">
            {{ input.name }}
          </Chip>
        </dd>
        <dt>Wake</dt>
        <dd class="wake data" :class="row.wake.tone">
          {{ row.wake.text }}
        </dd>
        <dt>Modes</dt>
        <dd class="modes data">
          <span v-for="line in row.modes" :key="line">{{ line }}</span>
        </dd>
      </dl>
      <p class="read-at data">
        {{ row.readAt }}
      </p>
    </section>
  </li>
</template>

<style scoped>
.monitor-row {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-4) var(--space-7);
  border-bottom: var(--hairline) solid var(--line);
  background: var(--surface);
}

.head {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
}

.alias {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) var(--space-2);
  border: var(--hairline) dashed var(--line-strong);
  border-radius: var(--r-sm);
  background: transparent;
  font-family: var(--body);
  font-size: var(--text-lg);
  font-weight: 600;
  color: var(--ink);
  cursor: text;
}

.alias:hover,
.alias:focus-visible {
  border-color: var(--accent);
}

.alias-hint {
  font-size: var(--text-2xs);
  font-weight: 400;
  color: var(--ink-3);
}

.alias-field {
  width: 150px;
  height: var(--control-h);
  padding: 0 var(--space-2);
  border: 1.5px solid var(--accent);
  border-radius: var(--r-sm);
  background: var(--surface);
  font-family: var(--body);
  font-size: var(--text-lg);
  font-weight: 600;
  color: var(--ink);
}

.reported {
  font-size: var(--text-md);
  color: var(--ink-2);
}

.spacer {
  flex: 1;
}

.note {
  font-size: var(--text-sm);
  color: var(--ink-2);
}

.grid {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: var(--space-4);
  margin: 0;
  font-size: var(--text-sm);
  color: var(--ink-2);
}

.grid div {
  display: flex;
  flex-direction: column;
  gap: var(--space-0);
  min-width: 0;
}

.grid dt {
  font-size: var(--text-2xs);
  letter-spacing: var(--tracking-label);
  text-transform: uppercase;
  color: var(--ink-3);
}

.grid dd {
  margin: 0;
  overflow-wrap: anywhere;
}

.data {
  font-family: var(--data);
}

.expand {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  width: fit-content;
  padding: 0;
  border: 0;
  background: transparent;
  font-family: var(--body);
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--accent);
  cursor: pointer;
}

.expand:hover {
  text-decoration: underline;
}

.caret {
  font-size: var(--text-2xs);
}

.muted {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: 0;
  font-size: var(--text-sm);
  color: var(--ink-3);
}

.muted.reading {
  color: var(--accent);
}

.spinner {
  width: 9px;
  height: 9px;
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

.capabilities {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-4);
  border: var(--hairline) solid var(--line);
  border-radius: var(--r);
  background: var(--surface-2);
}

.facts {
  display: grid;
  grid-template-columns: 96px minmax(0, 1fr);
  gap: var(--space-3) var(--space-4);
  align-items: start;
  margin: 0;
}

.facts dt {
  padding-top: var(--space-1);
  font-family: var(--data);
  font-size: var(--text-2xs);
  letter-spacing: var(--tracking-label);
  text-transform: uppercase;
  color: var(--ink-3);
}

.facts dd {
  margin: 0;
}

.accepts {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.wake {
  font-size: var(--text-sm);
}

.wake.good {
  color: var(--good);
}

.wake.warn {
  color: var(--warn);
}

.wake.mute {
  color: var(--ink-3);
}

.modes {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  font-size: var(--text-sm);
  color: var(--ink-2);
}

.read-at {
  margin: 0;
  padding-top: var(--space-3);
  border-top: var(--hairline) solid var(--line);
  font-size: var(--text-xs);
  color: var(--ink-3);
}
</style>
