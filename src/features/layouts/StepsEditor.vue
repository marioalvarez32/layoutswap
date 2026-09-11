<script setup lang="ts">
import { computed, ref } from 'vue';
import type { InputSource, Layout, Step, StepSide } from '@/domain/generated/types';
import { canMove, moveStep, removeStep, replaceStep, sendStepNote, stepsOn, type LabelOf } from '@/domain/steps';
import Button from '@/ui/Button.vue';
import StepRow, { type StepMonitor } from './StepRow.vue';

const props = defineProps<{
  steps: Step[];
  layout: Pick<Layout, 'summary'>;
  monitors: StepMonitor[];
  inputSources: InputSource[];
  labelOf: LabelOf;
  dropWaitSeconds: number;
  availableWaitSeconds: number;
}>();

const emit = defineEmits<{
  change: [steps: Step[]];
  /** Add a step on this side; the caller gives it its id. */
  add: [side: StepSide];
}>();

// Which row is expanded is the template's business alone.
const expandedId = ref<string | null>(null);

// The timeline: the fixed rows the script always runs, with the steps around the apply
// and an Add button at the end of each side.
type Row = { kind: 'fixed'; text: string; apply?: boolean } | { kind: 'step'; step: Step } | { kind: 'add'; side: StepSide };
const rows = computed<Row[]>(() => [
  { kind: 'fixed', text: 'Check monitors' },
  ...stepsOn(props.steps, 'before').map((step): Row => ({ kind: 'step', step })),
  { kind: 'add', side: 'before' },
  { kind: 'fixed', text: 'Apply arrangement', apply: true },
  ...stepsOn(props.steps, 'after').map((step): Row => ({ kind: 'step', step })),
  { kind: 'add', side: 'after' },
  { kind: 'fixed', text: 'Verify' },
]);

function toggle(id: string) {
  expandedId.value = expandedId.value === id ? null : id;
}

function change(step: Step) {
  emit('change', replaceStep(props.steps, step.id, step));
}

function move(id: string, direction: 'up' | 'down') {
  emit('change', moveStep(props.steps, id, direction));
}

function remove(id: string) {
  emit('change', removeStep(props.steps, id));
  if (expandedId.value === id) {
    expandedId.value = null;
  }
}

function expand(id: string) {
  expandedId.value = id;
}

defineExpose({ expand });
</script>

<template>
  <ol class="timeline">
    <template v-for="(row, i) in rows" :key="row.kind === 'step' ? row.step.id : `${row.kind}-${i}`">
      <li v-if="row.kind === 'fixed'" class="fixed" :class="{ apply: row.apply }">
        <span class="fixed-text">{{ row.text }}</span>
      </li>
      <li v-else-if="row.kind === 'add'" class="add-row">
        <Button class="add" :class="`add-${row.side}`" @click="emit('add', row.side)">
          Add step {{ row.side }}
        </Button>
      </li>
      <StepRow
        v-else
        :step="row.step"
        :expanded="expandedId === row.step.id"
        :monitors="monitors"
        :input-sources="inputSources"
        :label-of="labelOf"
        :note="sendStepNote(row.step, layout, labelOf)"
        :drop-wait-seconds="dropWaitSeconds"
        :available-wait-seconds="availableWaitSeconds"
        :can-move-up="canMove(steps, row.step.id, 'up')"
        :can-move-down="canMove(steps, row.step.id, 'down')"
        @toggle="toggle(row.step.id)"
        @change="change"
        @move="move(row.step.id, $event)"
        @remove="remove(row.step.id)"
      />
    </template>
  </ol>
</template>

<style scoped>
.timeline {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}

.fixed {
  display: flex;
  align-items: center;
  min-height: var(--row-h);
  padding: 0 var(--space-4);
  border: var(--hairline) solid var(--line);
  border-radius: var(--r);
  background: var(--surface-2);
}

.fixed-text {
  font-size: var(--text-md);
  font-weight: 500;
  color: var(--ink-2);
}

.fixed.apply .fixed-text {
  color: var(--accent);
}

.add-row {
  display: flex;
  padding: 0 var(--space-4);
}
</style>
