<script setup lang="ts">
import { computed, useId } from 'vue';
import { SCHEMATIC_BOX, type SchematicMonitor, schematicRects } from '@/domain/schematic';

const props = defineProps<{
  /** Every monitor of the arrangement; only on ones with a position are drawn. */
  monitors: readonly SchematicMonitor[];
  aliases: Record<string, string>;
  /** What to say when there is nothing to draw. */
  emptyText: string;
}>();

// A picture, not a control: no hover, no drag, no selection.
const clipPrefix = useId();
const box = SCHEMATIC_BOX;
const rects = computed(() =>
  schematicRects(props.monitors, props.aliases).map((rect, index) => ({ ...rect, clipId: `${clipPrefix}-${index}` })),
);
</script>

<template>
  <svg
    class="schematic"
    :viewBox="`0 0 ${box.width} ${box.height}`"
    :width="box.width"
    :height="box.height"
    role="img"
    aria-label="Arrangement of the monitors that are on"
  >
    <defs>
      <clipPath v-for="r in rects" :id="r.clipId" :key="r.clipId">
        <rect
          :x="r.x"
          :y="r.y"
          :width="r.width"
          :height="r.height"
        />
      </clipPath>
    </defs>
    <g
      v-for="r in rects"
      :key="r.devicePath"
      class="monitor"
      :class="{ primary: r.primary }"
    >
      <rect
        :x="r.x"
        :y="r.y"
        :width="r.width"
        :height="r.height"
      />
      <text
        class="label"
        :x="r.x + 6"
        :y="r.compact ? r.y + r.height / 2 + 4 : r.y + r.height / 2 - 2"
        :clip-path="`url(#${r.clipId})`"
      >{{ r.label }}</text>
      <text
        v-if="!r.compact && r.detail"
        class="detail"
        :x="r.x + 6"
        :y="r.y + r.height / 2 + 11"
        :clip-path="`url(#${r.clipId})`"
      >{{ r.detail }}</text>
    </g>
    <text
      v-if="rects.length === 0"
      class="empty"
      :x="box.width / 2"
      :y="box.height / 2"
      text-anchor="middle"
    >{{ emptyText }}</text>
  </svg>
</template>

<style scoped>
.schematic {
  display: block;
  max-width: 100%;
  height: auto;
  flex: none;
  border: var(--hairline) solid var(--line);
  border-radius: var(--r-sm);
  background: var(--surface);
  user-select: none;
}

.monitor rect {
  rx: var(--r-sm);
  fill: var(--surface-3);
  stroke: var(--line-strong);
  stroke-width: var(--hairline);
}

.monitor.primary rect {
  fill: var(--accent-soft);
  stroke: var(--accent);
}

.label {
  font-family: var(--body);
  font-size: var(--text-xs);
  font-weight: 600;
  fill: var(--ink-2);
}

.detail {
  font-family: var(--data);
  font-size: var(--text-2xs);
  fill: var(--ink-2);
}

.monitor.primary .label,
.monitor.primary .detail {
  fill: var(--accent);
}

.empty {
  font-family: var(--body);
  font-size: var(--text-sm);
  fill: var(--ink-3);
}
</style>
