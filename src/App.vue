<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, onMounted, ref } from 'vue';
import LayoutDetail from '@/features/layouts/LayoutDetail.vue';
import LayoutsEmptyState from '@/features/layouts/LayoutsEmptyState.vue';
import SaveLayoutPage from '@/features/layouts/SaveLayoutPage.vue';
import Sidebar from '@/features/layouts/Sidebar.vue';
import { useLayoutsStore } from '@/features/layouts/layouts.store';
import { useWindowSize } from '@/features/settings/useWindowSize';
import { formatProbeTime } from '@/domain/time';

const layoutsStore = useLayoutsStore();
const { listItems, selectedId, selected, isEmpty, aliases, inventory, loadError } = storeToRefs(layoutsStore);

const { error: windowSizeError } = useWindowSize();

// The content area shows the Save current layout page, the selected layout, or the
// first-run empty state.
const savePageOpen = ref(false);
const lastProbe = computed(() => (inventory.value ? formatProbeTime(inventory.value.probedAt) : null));
const banner = computed(() => loadError.value ?? windowSizeError.value);

function openSave() {
  savePageOpen.value = true;
}

function closeSave() {
  savePageOpen.value = false;
}

function onSelect(id: string) {
  layoutsStore.select(id);
  savePageOpen.value = false;
}

onMounted(async () => {
  await layoutsStore.load();
  await layoutsStore.probe();
});
</script>

<template>
  <div class="shell">
    <Sidebar
      :layouts="listItems"
      :selected-id="savePageOpen ? null : selectedId"
      :last-probe="lastProbe"
      @save="openSave"
      @select="onSelect"
    />
    <main class="content">
      <p v-if="banner" class="alert" role="alert">
        {{ banner }}
      </p>
      <SaveLayoutPage v-if="savePageOpen" @cancel="closeSave" @saved="closeSave" />
      <LayoutDetail
        v-else-if="selected"
        :layout="selected"
        :aliases="aliases"
        :inventory="inventory"
      />
      <div v-else-if="isEmpty" class="centre">
        <LayoutsEmptyState @save="openSave" />
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
