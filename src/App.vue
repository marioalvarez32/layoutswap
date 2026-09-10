<script setup lang="ts">
import { storeToRefs } from 'pinia';
import LayoutsEmptyState from '@/features/layouts/LayoutsEmptyState.vue';
import Sidebar from '@/features/layouts/Sidebar.vue';
import { useLayoutsStore } from '@/features/layouts/layouts.store';
import { useWindowSize } from '@/features/settings/useWindowSize';

const layoutsStore = useLayoutsStore();
const { layouts, selectedId, isEmpty } = storeToRefs(layoutsStore);

const { error: configError } = useWindowSize();
</script>

<template>
  <div class="shell">
    <Sidebar
      :layouts="layouts"
      :selected-id="selectedId"
      :last-probe="null"
      @select="layoutsStore.select"
    />
    <main class="content">
      <p v-if="configError" class="alert" role="alert">
        {{ configError }}
      </p>
      <div v-if="isEmpty" class="centre">
        <LayoutsEmptyState />
      </div>
    </main>
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  height: 100%;
  background: var(--surface);
}

.content {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
}

.alert {
  margin: 0;
  padding: var(--space-4) var(--space-7);
  border-bottom: var(--hairline) solid var(--crit);
  background: var(--crit-soft);
  color: var(--crit);
  font-size: var(--text-md);
}

.centre {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  padding: var(--space-8);
  overflow: auto;
}
</style>
