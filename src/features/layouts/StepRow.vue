<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import type { InputSource, Step, WaitRule } from '@/domain/generated/types';
import {
  checkInputCode,
  checkWaitSeconds,
  formatInputCode,
  parseInputCode,
  stepSentence,
  waitRuleHint,
  withKind,
  WAIT_SECONDS_MAX,
  WAIT_SECONDS_MIN,
  type LabelOf,
} from '@/domain/steps';
import Button from '@/ui/Button.vue';

/** One monitor the send step can target, as the select lists it. */
export interface StepMonitor {
  devicePath: string;
  label: string;
  /** The input source it shows now, when the probe knows. */
  currentInput: number | null;
}

const props = defineProps<{
  step: Step;
  expanded: boolean;
  monitors: StepMonitor[];
  inputSources: InputSource[];
  labelOf: LabelOf;
  /** What the editor says under the step, or null. */
  note: string | null;
  dropWaitSeconds: number;
  availableWaitSeconds: number;
  canMoveUp: boolean;
  canMoveDown: boolean;
}>();

const emit = defineEmits<{
  toggle: [];
  change: [step: Step];
  move: [direction: 'up' | 'down'];
  remove: [];
}>();

const OTHER = 'other';

// The step under each kind, so the template and the handlers read typed fields.
const waitStep = computed(() => (props.step.kind === 'wait' ? props.step : null));
const sendStep = computed(() => (props.step.kind === 'sendInput' ? props.step : null));

const sentence = computed(() => stepSentence(props.step, props.labelOf, props.inputSources));
const secondsRule = computed(() => (waitStep.value ? checkWaitSeconds(waitStep.value.seconds) : null));
const inputKnown = computed(() => {
  const send = sendStep.value;
  return send !== null && props.inputSources.some((s) => s.code === send.inputSource);
});
const currentInputOf = computed(() => {
  const send = sendStep.value;
  return send ? props.monitors.find((m) => m.devicePath === send.devicePath)?.currentInput ?? null : null;
});
const monitorKnown = computed(() => {
  const send = sendStep.value;
  return send !== null && props.monitors.some((m) => m.devicePath === send.devicePath);
});
const waitHint = computed(() => (sendStep.value
  ? waitRuleHint(sendStep.value.wait, props.dropWaitSeconds, props.availableWaitSeconds)
  : ''));

// The "Other code" field keeps what the user typed until it parses to a code; the
// step keeps its last valid code meanwhile, so nothing half-typed is ever saved.
const otherCode = ref('');
const otherRule = ref<string | null>(null);
const otherMode = ref(false);
watch(() => props.step, (step) => {
  if (step.kind === 'sendInput' && !inputKnown.value) {
    otherCode.value = formatInputCode(step.inputSource);
    otherMode.value = true;
  }
}, { immediate: true });

const defaults = computed(() => ({
  devicePath: props.monitors[0]?.devicePath ?? '',
  inputSource: props.inputSources[0]?.code ?? 0x11,
}));

function setKind(kind: string) {
  emit('change', withKind(props.step, kind === 'sendInput' ? 'sendInput' : 'wait', defaults.value));
}

function setSeconds(raw: string) {
  if (props.step.kind === 'wait') {
    emit('change', { ...props.step, seconds: Number(raw) });
  }
}

function setMonitor(devicePath: string) {
  if (props.step.kind === 'sendInput') {
    emit('change', { ...props.step, devicePath });
  }
}

function setInput(raw: string) {
  if (props.step.kind !== 'sendInput') {
    return;
  }
  if (raw === OTHER) {
    otherCode.value = inputKnown.value ? '' : formatInputCode(props.step.inputSource);
    otherRule.value = null;
    otherMode.value = true;
    return;
  }
  otherMode.value = false;
  emit('change', { ...props.step, inputSource: Number(raw) });
}

function setOtherCode(raw: string) {
  otherCode.value = raw;
  const code = parseInputCode(raw);
  otherRule.value = checkInputCode(code);
  if (props.step.kind === 'sendInput' && otherRule.value === null) {
    emit('change', { ...props.step, inputSource: code });
  }
}

function setWait(wait: string) {
  if (props.step.kind === 'sendInput') {
    emit('change', { ...props.step, wait: wait as WaitRule });
  }
}

const showOther = computed(() => otherMode.value || !inputKnown.value);
const inputSelectValue = computed(() => (sendStep.value && !showOther.value ? String(sendStep.value.inputSource) : OTHER));
</script>

<template>
  <li class="step" :class="{ expanded }">
    <button
      type="button"
      class="sentence"
      :aria-expanded="expanded ? 'true' : 'false'"
      @click="emit('toggle')"
    >
      {{ sentence }}
    </button>
    <p v-if="note && !expanded" class="note">
      {{ note }}
    </p>
    <div v-if="expanded" class="controls">
      <label class="field">
        <span>Step</span>
        <select class="kind" :value="step.kind" @change="setKind(($event.target as HTMLSelectElement).value)">
          <option value="wait">Wait a number of seconds</option>
          <option value="sendInput">Send an input source to a monitor</option>
        </select>
      </label>

      <template v-if="waitStep">
        <label class="field">
          <span>Wait</span>
          <input
            type="number"
            class="seconds"
            :min="WAIT_SECONDS_MIN"
            :max="WAIT_SECONDS_MAX"
            :value="waitStep.seconds"
            :aria-invalid="secondsRule ? 'true' : undefined"
            @input="setSeconds(($event.target as HTMLInputElement).value)"
          >
          <span>seconds</span>
        </label>
        <span v-if="secondsRule" class="rule" role="alert">{{ secondsRule }}</span>
      </template>

      <template v-else-if="sendStep">
        <div class="send-fields">
          <label class="field">
            <span>Send</span>
            <select class="input" :value="inputSelectValue" @change="setInput(($event.target as HTMLSelectElement).value)">
              <option
                v-for="source in inputSources"
                :key="source.code"
                :value="String(source.code)"
              >
                {{ source.name }}{{ source.code === currentInputOf ? ' (now)' : '' }}
              </option>
              <option :value="OTHER">
                Other code
              </option>
            </select>
          </label>
          <label v-if="showOther" class="field">
            <span>Code</span>
            <input
              type="text"
              class="other-code"
              :value="otherCode"
              placeholder="0x1E"
              :aria-invalid="otherRule ? 'true' : undefined"
              @input="setOtherCode(($event.target as HTMLInputElement).value)"
            >
          </label>
          <label class="field">
            <span>to</span>
            <select class="monitor" :value="sendStep.devicePath" @change="setMonitor(($event.target as HTMLSelectElement).value)">
              <option
                v-for="m in monitors"
                :key="m.devicePath"
                :value="m.devicePath"
              >
                {{ m.label }}
              </option>
              <option v-if="!monitorKnown" :value="sendStep.devicePath">
                {{ labelOf(sendStep.devicePath) }}
              </option>
            </select>
          </label>
          <label class="field">
            <span>then</span>
            <select class="wait" :value="sendStep.wait" @change="setWait(($event.target as HTMLSelectElement).value)">
              <option value="none">
                no wait
              </option>
              <option value="drop">
                wait until it drops
              </option>
              <option value="available">
                wait until it is Available
              </option>
            </select>
            <span v-if="waitHint" class="faint">{{ waitHint }}</span>
          </label>
        </div>
        <span v-if="otherRule" class="rule" role="alert">{{ otherRule }}</span>
        <p v-if="note" class="note">
          {{ note }}
        </p>
      </template>

      <div class="row-actions">
        <Button class="move-up" :disabled="!canMoveUp" @click="emit('move', 'up')">
          Move up
        </Button>
        <Button class="move-down" :disabled="!canMoveDown" @click="emit('move', 'down')">
          Move down
        </Button>
        <Button class="remove" @click="emit('remove')">
          Remove
        </Button>
      </div>
    </div>
  </li>
</template>

<style scoped>
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

.note {
  margin: 0;
  padding: 0 var(--space-4) var(--space-3);
  font-size: var(--text-sm);
  color: var(--warn);
}

.controls {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: 0 var(--space-4) var(--space-4);
}

.send-fields {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3) var(--space-4);
}

.field {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-md);
}

.seconds,
.other-code,
select {
  height: var(--row-h);
  padding: 0 var(--space-2);
  border: var(--hairline) solid var(--line-strong);
  border-radius: var(--r);
  background: var(--surface);
  font-size: var(--text-md);
}

.seconds,
.other-code {
  width: var(--col-primary);
  font-family: var(--data);
}

.rule {
  font-size: var(--text-sm);
  color: var(--crit);
}

.faint {
  font-family: var(--data);
  font-size: var(--text-xs);
  color: var(--ink-3);
}

.row-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
</style>
