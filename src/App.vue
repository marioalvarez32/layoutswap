<script setup lang="ts">
import { storeToRefs } from 'pinia';
import { computed, onMounted, ref } from 'vue';
import LayoutDetail from '@/features/layouts/LayoutDetail.vue';
import LayoutsEmptyState from '@/features/layouts/LayoutsEmptyState.vue';
import SaveChangesDialog from '@/features/layouts/SaveChangesDialog.vue';
import SaveLayoutPage from '@/features/layouts/SaveLayoutPage.vue';
import Sidebar from '@/features/layouts/Sidebar.vue';
import SwitchProgress from '@/features/layouts/SwitchProgress.vue';
import { useLayoutsStore } from '@/features/layouts/layouts.store';
import { useUnsavedGuard } from '@/features/layouts/useUnsavedGuard';
import { useWindowSize } from '@/features/settings/useWindowSize';
import { formatProbeTime } from '@/domain/time';

const layoutsStore = useLayoutsStore();
const {
  listItems, selectedId, selected, isEmpty, aliases, inventory, loadError, switchRun, resultActionBusy,
  transferBusy, transferNote, transferError,
} = storeToRefs(layoutsStore);

const { error: windowSizeError } = useWindowSize();

// The content area shows the Save current layout page, the switch progress of the
// selected layout, the selected layout, or the first-run empty state.
const savePageOpen = ref(false);
const switchOnScreen = computed(() => switchRun.value !== null && switchRun.value.layoutId === selectedId.value);
const lastProbe = computed(() => (inventory.value ? formatProbeTime(inventory.value.probedAt) : null));
const banner = computed(() => loadError.value ?? windowSizeError.value);

// Leaving a layout with unsaved steps asks first; the move waits on the answer.
const unsaved = useUnsavedGuard();

function openSave() {
  unsaved.guard(() => {
    savePageOpen.value = true;
  });
}

function closeSave() {
  savePageOpen.value = false;
}

function onSelect(id: string) {
  if (id === selectedId.value && !savePageOpen.value) {
    return;
  }
  unsaved.guard(() => {
    layoutsStore.select(id);
    savePageOpen.value = false;
  });
}

async function onImport() {
  await layoutsStore.importConfig();
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
      :transfer-busy="transferBusy"
      :transfer-note="transferNote"
      :transfer-error="transferError"
      @save="openSave"
      @select="onSelect"
      @export="layoutsStore.exportConfig"
      @import="onImport"
    />
    <SaveChangesDialog
      :open="unsaved.asking.value"
      :layout-name="unsaved.layoutName.value"
      :saving="unsaved.saving.value"
      :error="unsaved.error.value"
      @save="unsaved.save"
      @discard="unsaved.discard"
      @keep="unsaved.keep"
    />
    <main class="content">
      <p v-if="banner" class="alert" role="alert">
        {{ banner }}
      </p>
      <SaveLayoutPage v-if="savePageOpen" @cancel="closeSave" @saved="closeSave" />
      <SwitchProgress
        v-else-if="switchOnScreen && switchRun"
        :run="switchRun"
        :busy="resultActionBusy"
        @cancel="layoutsStore.cancelSwitch"
        @back="layoutsStore.dismissSwitch"
        @open-log="layoutsStore.resultAction('openLog')"
        @save-diagnostics="layoutsStore.resultAction('saveDiagnostics')"
        @open-display-settings="layoutsStore.resultAction('openDisplaySettings')"
      />
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
