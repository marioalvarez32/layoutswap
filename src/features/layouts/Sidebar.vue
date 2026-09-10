<script setup lang="ts">
import { formatOnOffCount, type LayoutListItem } from '@/domain/layouts';
import Button from '@/ui/Button.vue';

defineProps<{
  layouts: LayoutListItem[];
  selectedId: string | null;
  /** When the probe last ran, already formatted, or null when it has not run yet. */
  lastProbe: string | null;
}>();

defineEmits<{
  save: [];
  select: [id: string];
}>();

// The inventory screens are pinned so the navigation shape is final; their screens
// come in later slices, so the items are visible but disabled.
const pinned = ['Monitors', 'Audio', 'Remote Desktop', 'Settings'] as const;
</script>

<template>
  <nav class="sidebar" aria-label="Layouts">
    <div class="top">
      <Button variant="primary" class="save" @click="$emit('save')">
        Save current layout
      </Button>

      <p v-if="layouts.length === 0" class="empty">
        No layouts yet
      </p>

      <ul v-else class="rows">
        <li v-for="layout in layouts" :key="layout.id">
          <button
            type="button"
            class="row"
            :class="{ selected: layout.id === selectedId }"
            :aria-current="layout.id === selectedId ? 'true' : undefined"
            @click="$emit('select', layout.id)"
          >
            <span class="name">{{ layout.name }}</span>
            <span class="count">{{ formatOnOffCount(layout) }}</span>
          </button>
        </li>
      </ul>
    </div>

    <div class="bottom">
      <ul class="pinned">
        <li v-for="item in pinned" :key="item">
          <button type="button" class="pinned-item" disabled>
            {{ item }}
          </button>
        </li>
      </ul>
      <p class="probe">
        Last probe<br>{{ lastProbe ?? 'not run yet' }}
      </p>
    </div>
  </nav>
</template>

<style scoped>
.sidebar {
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: var(--space-4);
  width: var(--sidebar-w);
  height: 100%;
  flex: none;
  padding: var(--space-3) var(--space-2);
  background: var(--surface-2);
  border-right: var(--hairline) solid var(--line);
}

.top {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  min-height: 0;
}

.save {
  width: 100%;
}

.empty {
  margin: 0;
  padding: var(--space-3) var(--space-2);
  font-size: var(--text-sm);
  color: var(--ink-3);
}

ul {
  margin: 0;
  padding: 0;
  list-style: none;
}

.rows {
  display: flex;
  flex-direction: column;
  gap: var(--hairline);
  overflow: auto;
  min-height: 0;
}

.row {
  display: flex;
  flex-direction: column;
  gap: var(--space-0);
  width: 100%;
  min-height: var(--control-h);
  padding: var(--space-2) var(--space-3);
  border: 0;
  border-radius: var(--r);
  background: transparent;
  text-align: left;
  cursor: pointer;
}

.row:hover {
  background: var(--surface-3);
}

.row:focus-visible {
  outline-offset: calc(-1 * var(--mark-w));
}

.row.selected {
  background: var(--accent-soft);
  box-shadow: inset var(--mark-w) 0 0 var(--accent);
  color: var(--accent);
}

.name {
  font-size: var(--text-md);
  font-weight: 500;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.row.selected .name {
  font-weight: 600;
}

.count {
  font-family: var(--data);
  font-size: var(--text-xs);
  color: var(--ink-3);
}

.row.selected .count {
  color: var(--accent);
}

.bottom {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  flex: none;
  padding-top: var(--space-3);
  border-top: var(--hairline) solid var(--line);
}

.pinned {
  display: flex;
  flex-direction: column;
  gap: var(--hairline);
}

.pinned-item {
  display: flex;
  align-items: center;
  width: 100%;
  height: var(--row-h);
  padding: 0 var(--space-3);
  border: 0;
  border-radius: var(--r);
  background: transparent;
  font-size: var(--text-md);
  font-weight: 500;
  color: var(--ink-2);
  text-align: left;
  cursor: pointer;
}

.pinned-item:hover:not(:disabled) {
  background: var(--surface-3);
}

.pinned-item:disabled {
  color: var(--ink-3);
  cursor: default;
  opacity: 0.6;
}

.probe {
  margin: 0;
  padding: 0 var(--space-3) var(--space-0);
  font-family: var(--data);
  font-size: var(--text-xs);
  line-height: var(--leading);
  color: var(--ink-3);
}
</style>
