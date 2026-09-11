<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, ref } from 'vue';
import type { ApplyFailure, Inventory, Layout, MonitorState } from '@/domain/generated/types';
import { chipLabels, formatSpec, inLayoutHint, inputSourceTooltip, liveMonitor, liveState, monitorDisplay, monitorStateNote } from '@/domain/monitors';
import { stepLabeller, AVAILABLE_WAIT_SECONDS_MAX, DROP_WAIT_SECONDS_MAX } from '@/domain/steps';
import { describeCaptureTime } from '@/domain/time';
import Button from '@/ui/Button.vue';
import Chip from '@/ui/Chip.vue';
import ArrangementSchematic from './ArrangementSchematic.vue';
import StepsEditor from './StepsEditor.vue';
import { useLayoutsStore } from './layouts.store';
import { useLayoutEditor } from './useLayoutEditor';
import { useLayoutScript } from './useLayoutScript';
import { useSwitchLayout } from './useSwitchLayout';

const props = defineProps<{
  layout: Layout;
  aliases: Record<string, string>;
  /** The latest probe, for each monitor's live state; null before the first probe. */
  inventory: Inventory | null;
}>();

const script = useLayoutScript(() => props.layout.id);
const switchAction = useSwitchLayout(() => props.layout.id);
const editor = useLayoutEditor(() => props.layout.id);
const { inputSources } = storeToRefs(useLayoutsStore());

// The send step names monitors by the layout's label rule and marks their current input.
const labelOf = computed(() => stepLabeller(props.aliases, props.layout.summary.monitors));
const stepMonitors = computed(() => props.layout.summary.monitors.map((m) => ({
  devicePath: m.devicePath,
  label: labelOf.value(m.devicePath),
  currentInput: liveMonitor(props.inventory, m.devicePath)?.inputSource ?? null,
})));
const stepsEditor = ref<InstanceType<typeof StepsEditor> | null>(null);

// The fallback choice, in the words the design record uses.
const FALLBACK_CHOICES: { value: ApplyFailure; text: string }[] = [
  { value: 'stop', text: 'Stop and explain' },
  { value: 'extend', text: 'Fall back to Windows Extend when a monitor is not Available' },
];

function addStep(side: 'before' | 'after') {
  stepsEditor.value?.expand(editor.addStep(side));
}
const error = computed(() => script.error.value ?? switchAction.error.value ?? editor.error.value);

type Tone = 'good' | 'warn' | 'crit' | 'mute';
const TONES: Record<MonitorState, Tone> = { Active: 'good', Available: 'warn', Absent: 'crit' };

const monitors = computed(() =>
  props.layout.summary.monitors.map((m) => {
    const display = monitorDisplay(props.aliases, m);
    const state = liveState(props.inventory, m.devicePath);
    const tone: Tone = state ? TONES[state] : 'mute';
    const live = liveMonitor(props.inventory, m.devicePath);
    return {
      devicePath: m.devicePath,
      on: m.on,
      primary: m.primary,
      label: display.label,
      detail: display.detail,
      spec: formatSpec(m),
      state,
      tone,
      note: state ? monitorStateNote(state) : '',
      hint: state ? inLayoutHint(display.label, m.on, state) : '',
      input: live?.inputSourceName ?? null,
      inputTooltip: live?.inputSource === null || live === null ? '' : inputSourceTooltip(live.inputSource),
    };
  }),
);
// The summary table and the off chips carry the connector when another monitor in
// the layout shares the label, so two identical panels read apart.
const withShortNames = computed(() => {
  const labels = chipLabels(monitors.value);
  return monitors.value.map((m, i) => ({ ...m, shortName: labels[i] ?? m.label }));
});
const onMonitors = computed(() => withShortNames.value.filter((m) => m.on));
const offMonitors = computed(() => withShortNames.value.filter((m) => !m.on));
</script>

<template>
  <article class="layout-detail">
    <header class="header">
      <div class="title">
        <h2>{{ layout.name }}</h2>
        <p class="captured">
          {{ describeCaptureTime(layout.capturedAt) }}
        </p>
        <p v-if="script.indicator.value" class="script-line" :class="script.indicator.value.tone">
          <span class="dot" aria-hidden="true" />
          {{ script.indicator.value.text }}
        </p>
      </div>
      <div class="actions">
        <template v-if="editor.dirty.value">
          <Button class="discard" :disabled="editor.saving.value" @click="editor.discard">
            Discard
          </Button>
          <Button
            variant="primary"
            class="save"
            :disabled="editor.saving.value"
            @click="editor.save"
          >
            Save
          </Button>
        </template>
        <Button :disabled="script.busy.value" @click="script.open">
          Open script
        </Button>
        <Button :disabled="script.busy.value" @click="script.openSwitchLog">
          Open log
        </Button>
        <Button :disabled="script.busy.value" @click="script.regenerate">
          Regenerate script
        </Button>
        <span class="switch">
          <Button
            variant="primary"
            :disabled="switchAction.busy.value || switchAction.blockedBy.value !== null"
            @click="switchAction.start"
          >
            Switch to {{ layout.name }}
          </Button>
          <span v-if="switchAction.blockedBy.value" class="blocked">{{ switchAction.blockedBy.value }}</span>
        </span>
      </div>
    </header>

    <div class="body">
      <p v-if="error" class="band crit" role="alert">
        {{ error }}
      </p>

      <section class="section">
        <div class="section-head">
          <h3>Arrangement</h3>
          <span class="muted">Read-only. Edit in Windows Settings &gt; Display, then save again.</span>
        </div>
        <div class="panel">
          <div class="arrangement">
            <ArrangementSchematic
              :monitors="layout.summary.monitors"
              :aliases="aliases"
              empty-text="Nothing on in this layout"
            />
            <ul class="spec-list">
              <li v-for="m in onMonitors" :key="m.devicePath" class="spec-row">
                <span class="spec-who">
                  <span class="spec-name">{{ m.shortName }}</span>
                  <span v-if="m.primary" class="primary">Primary</span>
                </span>
                <span class="data">{{ m.spec }}</span>
              </li>
            </ul>
          </div>
          <div class="off-row">
            <span class="muted">Off in this layout:</span>
            <template v-if="offMonitors.length > 0">
              <Chip v-for="m in offMonitors" :key="m.devicePath" tone="mute">
                {{ m.shortName }}
              </Chip>
              <span class="faint">An off monitor stays connected but has no position.</span>
            </template>
            <span v-else class="faint">none, every connected monitor is on.</span>
          </div>
        </div>
      </section>

      <section class="section">
        <div class="section-head">
          <h3>Steps</h3>
          <span class="muted">Run in order around Apply arrangement. Save to regenerate the script.</span>
        </div>
        <StepsEditor
          ref="stepsEditor"
          :steps="editor.edits.value.steps"
          :layout="layout"
          :monitors="stepMonitors"
          :input-sources="inputSources"
          :label-of="labelOf"
          :drop-wait-seconds="editor.edits.value.dropWaitSeconds"
          :available-wait-seconds="editor.edits.value.availableWaitSeconds"
          @change="editor.setSteps"
          @add="addStep"
        />
        <fieldset class="fallback">
          <legend>If the switch fails</legend>
          <label v-for="choice in FALLBACK_CHOICES" :key="choice.value" class="fallback-choice">
            <input
              type="radio"
              name="on-apply-failure"
              :value="choice.value"
              :checked="editor.edits.value.onApplyFailure === choice.value"
              @change="editor.patch({ onApplyFailure: choice.value })"
            >
            <span>{{ choice.text }}</span>
          </label>
        </fieldset>
        <details class="advanced">
          <summary>Advanced: drop wait {{ editor.edits.value.dropWaitSeconds }} s · Available wait {{ editor.edits.value.availableWaitSeconds }} s</summary>
          <label class="field">
            <span>Drop wait</span>
            <input
              type="number"
              class="drop-wait"
              min="0"
              :max="DROP_WAIT_SECONDS_MAX"
              :value="editor.edits.value.dropWaitSeconds"
              :aria-invalid="editor.dropWaitRule.value ? 'true' : undefined"
              @input="editor.patch({ dropWaitSeconds: Number(($event.target as HTMLInputElement).value) })"
            >
            <span>seconds a send step waits for its monitor to drop</span>
          </label>
          <span v-if="editor.dropWaitRule.value" class="rule" role="alert">{{ editor.dropWaitRule.value }}</span>
          <label class="field">
            <span>Available wait</span>
            <input
              type="number"
              class="available-wait"
              min="1"
              :max="AVAILABLE_WAIT_SECONDS_MAX"
              :value="editor.edits.value.availableWaitSeconds"
              :aria-invalid="editor.availableWaitRule.value ? 'true' : undefined"
              @input="editor.patch({ availableWaitSeconds: Number(($event.target as HTMLInputElement).value) })"
            >
            <span>seconds a wait for a monitor to become Available lasts, Check monitors included</span>
          </label>
          <span v-if="editor.availableWaitRule.value" class="rule" role="alert">{{ editor.availableWaitRule.value }}</span>
        </details>
      </section>

      <section class="section">
        <h3>Monitors in this layout</h3>
        <ul class="list">
          <li v-for="m in monitors" :key="m.devicePath" class="row">
            <div class="who">
              <span class="name">{{ m.label }}</span>
              <span class="detail">{{ m.detail }}</span>
              <Chip v-if="m.state" :tone="m.tone" :title="m.note">
                {{ m.state }}
              </Chip>
              <span v-if="m.input" class="input data" :title="m.inputTooltip">{{ m.input }}</span>
            </div>
            <div class="hint">
              {{ m.hint }}
            </div>
            <div class="on-off" :class="{ on: m.on }">
              {{ m.on ? 'On' : 'Off' }}
            </div>
          </li>
        </ul>
      </section>
    </div>
  </article>
</template>

<style scoped>
.layout-detail {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.header {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: var(--space-6);
  padding: var(--space-6) var(--space-7) var(--space-5);
  border-bottom: var(--hairline) solid var(--line);
}

.title {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
}

.actions {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
  flex: none;
}

.switch {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: var(--space-1);
}

.blocked {
  font-size: var(--text-xs);
  color: var(--ink-3);
  white-space: nowrap;
}

.script-line {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: 0;
  font-size: var(--text-sm);
  color: var(--ink-2);
}

.dot {
  width: var(--dot);
  height: var(--dot);
  border-radius: var(--r-round);
  flex: none;
}

.script-line.good .dot {
  background: var(--good);
}

.script-line.warn .dot {
  background: var(--warn);
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

h2 {
  margin: 0;
  font-family: var(--display);
  font-size: var(--text-xl);
  font-weight: 600;
  letter-spacing: var(--tracking-tight);
}

.captured {
  margin: 0;
  font-size: var(--text-sm);
  color: var(--ink-2);
}

.body {
  display: flex;
  flex-direction: column;
  gap: var(--space-7);
  flex: 1;
  min-height: 0;
  padding: var(--space-6) var(--space-7);
  overflow: auto;
}

.section {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.section-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-4);
}

h3 {
  margin: 0;
  font-family: var(--display);
  font-size: var(--text-base);
  font-weight: 600;
}

.muted {
  font-size: var(--text-sm);
  color: var(--ink-2);
}

.faint {
  font-size: var(--text-sm);
  color: var(--ink-3);
}

.panel {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-5);
  border: var(--hairline) solid var(--line);
  border-radius: var(--r);
  background: var(--surface-2);
}

ul {
  margin: 0;
  padding: 0;
  list-style: none;
}

.arrangement {
  display: flex;
  align-items: flex-start;
  gap: var(--space-6);
}

.spec-list {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
}

.spec-row {
  display: grid;
  grid-template-columns: var(--col-alias) 1fr;
  gap: var(--space-4);
  padding: var(--space-2) 0;
  border-bottom: var(--hairline) solid var(--line);
}

.spec-who {
  min-width: 0;
  overflow-wrap: anywhere;
}

.spec-name {
  font-size: var(--text-md);
  font-weight: 600;
}

.primary {
  margin-left: var(--space-2);
  font-size: var(--text-xs);
  font-weight: 500;
  color: var(--accent);
  white-space: nowrap;
}

.data {
  font-family: var(--data);
  font-size: var(--text-sm);
  color: var(--ink-2);
}

.off-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  flex-wrap: wrap;
}

.fallback {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
  padding: var(--space-3) var(--space-4);
  border: var(--hairline) solid var(--line);
  border-radius: var(--r);
}

.fallback legend {
  padding: 0 var(--space-1);
  font-size: var(--text-sm);
  color: var(--ink-2);
}

.fallback-choice {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-md);
}

.advanced {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-3) var(--space-4);
  border: var(--hairline) solid var(--line);
  border-radius: var(--r);
  background: var(--surface-2);
}

.advanced summary {
  font-size: var(--text-sm);
  color: var(--ink-2);
  cursor: pointer;
}

.field {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-md);
}

.drop-wait,
.available-wait {
  width: var(--col-primary);
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

.list {
  border: var(--hairline) solid var(--line);
  border-radius: var(--r);
  overflow: hidden;
}

.row {
  display: grid;
  grid-template-columns: 1fr var(--col-hint) var(--col-on-off);
  gap: var(--space-4);
  align-items: center;
  padding: var(--space-4);
  border-bottom: var(--hairline) solid var(--line);
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

.input {
  white-space: nowrap;
}

.hint {
  font-size: var(--text-sm);
  color: var(--ink-2);
  text-wrap: pretty;
}

.on-off {
  justify-self: end;
  font-size: var(--text-sm);
  font-weight: 500;
  color: var(--ink-2);
}

.on-off.on {
  color: var(--accent);
}
</style>
