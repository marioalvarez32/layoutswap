<script setup lang="ts">
import { computed } from 'vue';
import type { Inventory, Layout, MonitorState } from '@/domain/generated/types';
import { chipLabels, formatSpec, inLayoutHint, liveState, monitorDisplay, monitorStateNote } from '@/domain/monitors';
import { describeCaptureTime } from '@/domain/time';
import Chip from '@/ui/Chip.vue';

const props = defineProps<{
  layout: Layout;
  aliases: Record<string, string>;
  /** The latest probe, for each monitor's live state; null before the first probe. */
  inventory: Inventory | null;
}>();

type Tone = 'good' | 'warn' | 'crit' | 'mute';
const TONES: Record<MonitorState, Tone> = { Active: 'good', Available: 'warn', Absent: 'crit' };

const monitors = computed(() =>
  props.layout.summary.monitors.map((m) => {
    const display = monitorDisplay(props.aliases, m);
    const state = liveState(props.inventory, m.devicePath);
    const tone: Tone = state ? TONES[state] : 'mute';
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
  <article class="detail">
    <header class="header">
      <h2>{{ layout.name }}</h2>
      <p class="captured">
        {{ describeCaptureTime(layout.capturedAt) }}
      </p>
    </header>

    <div class="body">
      <section class="section">
        <div class="section-head">
          <h3>Arrangement</h3>
          <span class="muted">Read-only. Edit in Windows Settings &gt; Display, then save again.</span>
        </div>
        <div class="panel">
          <ul class="spec-list">
            <li v-for="m in onMonitors" :key="m.devicePath" class="spec-row">
              <span class="spec-who">
                <span class="spec-name">{{ m.shortName }}</span>
                <span v-if="m.primary" class="primary">Primary</span>
              </span>
              <span class="data">{{ m.spec }}</span>
            </li>
          </ul>
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
        <h3>Monitors in this layout</h3>
        <ul class="list">
          <li v-for="m in monitors" :key="m.devicePath" class="row">
            <div class="who">
              <span class="name">{{ m.label }}</span>
              <span class="detail">{{ m.detail }}</span>
              <Chip v-if="m.state" :tone="m.tone" :title="m.note">
                {{ m.state }}
              </Chip>
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
.detail {
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

.spec-list {
  display: flex;
  flex-direction: column;
}

.spec-row {
  display: grid;
  grid-template-columns: var(--col-alias) 1fr;
  gap: var(--space-4);
  padding: var(--space-2) 0;
  border-bottom: var(--hairline) solid var(--line);
}

.spec-who {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.spec-name {
  font-size: var(--text-md);
  font-weight: 600;
}

.primary {
  font-size: var(--text-xs);
  font-weight: 500;
  color: var(--accent);
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
