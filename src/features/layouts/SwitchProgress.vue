<script setup lang="ts">
import { computed } from 'vue';
import { canCancel, failureBand, stepTone, switchHeadline, type SwitchRun } from '@/domain/switch';
import Button from '@/ui/Button.vue';
import Chip from '@/ui/Chip.vue';
import { useElapsed } from './useElapsed';

const props = defineProps<{
  run: SwitchRun;
}>();

const emit = defineEmits<{
  cancel: [];
  back: [];
}>();

const running = computed(() => props.run.result === null);
const { elapsedMs } = useElapsed(() => props.run.startedAt, running);

const headline = computed(() => switchHeadline(props.run, elapsedMs.value));
const failure = computed(() => (props.run.result ? failureBand(props.run.result) : null));
const cancelAllowed = computed(() => canCancel(props.run));
const steps = computed(() => props.run.steps.map((s) => ({ ...s, tone: stepTone(s.status) })));
const logText = computed(() => props.run.log.join('\n'));
</script>

<template>
  <section class="switch-progress">
    <header class="header">
      <h2>{{ headline.title }}</h2>
      <span class="aside" :class="headline.tone">{{ headline.aside }}</span>
    </header>

    <div class="body">
      <p v-if="failure" class="band crit" role="alert">
        <strong class="action">{{ failure.action }}</strong>
        <span class="detail">{{ failure.detail }}</span>
      </p>
      <p v-if="run.error" class="band warn" role="alert">
        {{ run.error }}
      </p>

      <ol class="steps">
        <li
          v-for="s in steps"
          :key="s.step"
          class="step"
          :class="s.status"
        >
          <Chip :tone="s.tone" class="status">
            {{ s.status }}
          </Chip>
          <span class="step-text">{{ s.text }}</span>
        </li>
      </ol>

      <div class="log-section">
        <span class="log-label">Log</span>
        <pre class="log">{{ logText || 'Waiting for the script to print its first line' }}</pre>
      </div>

      <div class="actions">
        <template v-if="running">
          <Button class="cancel" :disabled="!cancelAllowed" @click="emit('cancel')">
            {{ run.cancelling ? 'Cancelling' : 'Cancel' }}
          </Button>
          <span v-if="!cancelAllowed && !run.cancelling" class="faint">
            Cancel is off while the arrangement is being applied.
          </span>
        </template>
        <Button
          v-else
          variant="primary"
          class="back"
          @click="emit('back')"
        >
          Back to {{ run.layoutName }}
        </Button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.switch-progress {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-6);
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

.aside {
  flex: none;
  font-family: var(--data);
  font-size: var(--text-md);
  font-variant-numeric: tabular-nums;
  color: var(--ink-2);
}

.aside.good {
  color: var(--good);
}

.aside.crit {
  color: var(--crit);
}

.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
  flex: 1;
  min-height: 0;
  padding: var(--space-6) var(--space-7);
  overflow: auto;
}

.band {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  margin: 0;
  padding: var(--space-4);
  border: var(--hairline) solid;
  border-radius: var(--r);
  font-size: var(--text-md);
}

.band.crit {
  border-color: var(--crit);
  border-left-width: var(--mark-w);
  background: var(--crit-soft);
  color: var(--ink);
}

.band.crit .action {
  font-family: var(--display);
  font-weight: 600;
}

.band.crit .detail {
  font-size: var(--text-sm);
  color: var(--ink-2);
}

.band.warn {
  border-color: var(--warn);
  background: var(--warn-soft);
  color: var(--warn);
}

.steps {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}

.step {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-4);
  border: var(--hairline) solid var(--line);
  border-radius: var(--r);
  background: var(--surface);
}

.step.running {
  border-color: var(--accent-line);
  background: var(--accent-soft);
}

.step.waiting .step-text {
  color: var(--ink-3);
}

.step.failed {
  border-color: var(--crit);
}

.status {
  min-width: var(--col-on-off);
  justify-content: center;
}

.step-text {
  flex: 1;
  font-size: var(--text-md);
  text-wrap: pretty;
}

.log-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.log-label {
  font-family: var(--data);
  font-size: var(--text-xs);
  letter-spacing: var(--tracking-label);
  text-transform: uppercase;
  color: var(--ink-3);
}

.log {
  margin: 0;
  padding: var(--space-4);
  border: var(--hairline) solid var(--line);
  border-radius: var(--r);
  background: var(--surface-2);
  font-family: var(--data);
  font-size: var(--text-sm);
  line-height: var(--leading-prose);
  color: var(--ink-2);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.actions {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}

.faint {
  font-size: var(--text-sm);
  color: var(--ink-3);
}
</style>
