<script setup lang="ts">
import { computed, ref } from 'vue';
import type { Step, StepSide } from '@/domain/generated/types';
import {
  canMove,
  checkWaitSeconds,
  moveStep,
  removeStep,
  stepSentence,
  stepsOn,
  updateStep,
  WAIT_SECONDS_MAX,
  WAIT_SECONDS_MIN,
} from '@/domain/steps';
import Button from '@/ui/Button.vue';

const props = defineProps<{
  steps: Step[];
}>();

const emit = defineEmits<{
  change: [steps: Step[]];
  /** Add a wait step on this side; the caller gives it its id. */
  add: [side: StepSide];
}>();

// Which row is expanded is the template's business alone.
const expandedId = ref<string | null>(null);

// The timeline: the fixed rows the script always runs, with the steps around the apply.
const before = computed(() => stepsOn(props.steps, 'before'));
const after = computed(() => stepsOn(props.steps, 'after'));

function toggle(id: string) {
  expandedId.value = expandedId.value === id ? null : id;
}

function setSeconds(id: string, raw: string) {
  emit('change', updateStep(props.steps, id, { seconds: Number(raw) }));
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
    <li class="fixed">
      <span class="fixed-text">Check monitors</span>
    </li>

    <li
      v-for="step in before"
      :key="step.id"
      class="step"
      :class="{ expanded: expandedId === step.id }"
    >
      <button
        type="button"
        class="sentence"
        :aria-expanded="expandedId === step.id ? 'true' : 'false'"
        @click="toggle(step.id)"
      >
        {{ stepSentence(step) }}
      </button>
      <div v-if="expandedId === step.id" class="controls">
        <label class="field">
          <span>Wait</span>
          <input
            type="number"
            class="seconds"
            :min="WAIT_SECONDS_MIN"
            :max="WAIT_SECONDS_MAX"
            :value="step.seconds"
            :aria-invalid="checkWaitSeconds(step.seconds) ? 'true' : undefined"
            @input="setSeconds(step.id, ($event.target as HTMLInputElement).value)"
          >
          <span>seconds</span>
        </label>
        <span v-if="checkWaitSeconds(step.seconds)" class="rule" role="alert">{{ checkWaitSeconds(step.seconds) }}</span>
        <div class="row-actions">
          <Button class="move-up" :disabled="!canMove(steps, step.id, 'up')" @click="move(step.id, 'up')">
            Move up
          </Button>
          <Button class="move-down" :disabled="!canMove(steps, step.id, 'down')" @click="move(step.id, 'down')">
            Move down
          </Button>
          <Button class="remove" @click="remove(step.id)">
            Remove
          </Button>
        </div>
      </div>
    </li>
    <li class="add-row">
      <Button class="add add-before" @click="emit('add', 'before')">
        Add step before
      </Button>
    </li>

    <li class="fixed apply">
      <span class="fixed-text">Apply arrangement</span>
    </li>

    <li
      v-for="step in after"
      :key="step.id"
      class="step"
      :class="{ expanded: expandedId === step.id }"
    >
      <button
        type="button"
        class="sentence"
        :aria-expanded="expandedId === step.id ? 'true' : 'false'"
        @click="toggle(step.id)"
      >
        {{ stepSentence(step) }}
      </button>
      <div v-if="expandedId === step.id" class="controls">
        <label class="field">
          <span>Wait</span>
          <input
            type="number"
            class="seconds"
            :min="WAIT_SECONDS_MIN"
            :max="WAIT_SECONDS_MAX"
            :value="step.seconds"
            :aria-invalid="checkWaitSeconds(step.seconds) ? 'true' : undefined"
            @input="setSeconds(step.id, ($event.target as HTMLInputElement).value)"
          >
          <span>seconds</span>
        </label>
        <span v-if="checkWaitSeconds(step.seconds)" class="rule" role="alert">{{ checkWaitSeconds(step.seconds) }}</span>
        <div class="row-actions">
          <Button class="move-up" :disabled="!canMove(steps, step.id, 'up')" @click="move(step.id, 'up')">
            Move up
          </Button>
          <Button class="move-down" :disabled="!canMove(steps, step.id, 'down')" @click="move(step.id, 'down')">
            Move down
          </Button>
          <Button class="remove" @click="remove(step.id)">
            Remove
          </Button>
        </div>
      </div>
    </li>
    <li class="add-row">
      <Button class="add add-after" @click="emit('add', 'after')">
        Add step after
      </Button>
    </li>

    <li class="fixed">
      <span class="fixed-text">Verify</span>
    </li>
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

.step {
  border: var(--hairline) solid var(--line);
  border-radius: var(--r);
  background: var(--surface);
}

.step.expanded {
  border-color: var(--accent-line);
}

.sentence {
  display: block;
  width: 100%;
  min-height: var(--row-h);
  padding: var(--space-3) var(--space-4);
  border: 0;
  background: transparent;
  font-size: var(--text-md);
  text-align: left;
  cursor: pointer;
}

.sentence:focus-visible {
  outline-offset: calc(-1 * var(--mark-w));
}

.controls {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: 0 var(--space-4) var(--space-4);
}

.field {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-md);
}

.seconds {
  width: var(--col-on-off);
  height: var(--row-h);
  padding: 0 var(--space-2);
  border: var(--hairline) solid var(--line-strong);
  border-radius: var(--r);
  background: var(--surface);
  font-family: var(--data);
  font-size: var(--text-md);
}

.rule {
  font-size: var(--text-sm);
  color: var(--crit);
}

.row-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
</style>
