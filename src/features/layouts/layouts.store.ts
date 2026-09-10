import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { errorMessage } from '@/domain/errors';
import type { CaptureOutcome, Inventory, Layout } from '@/domain/generated/types';
import { toListItem, upsertLayout } from '@/domain/layouts';
import { captureLayout, loadConfig, probe as runProbe } from '@/tauri/commands';

/**
 * The layouts a user has saved, their aliases, the layout the sidebar has selected,
 * and the latest probe: everything more than one feature reads.
 */
export const useLayoutsStore = defineStore('layouts', () => {
  const layouts = ref<Layout[]>([]);
  const aliases = ref<Record<string, string>>({});
  const selectedId = ref<string | null>(null);
  const inventory = ref<Inventory | null>(null);
  const probing = ref(false);
  const loadError = ref<string | null>(null);
  const probeError = ref<string | null>(null);

  const isEmpty = computed(() => layouts.value.length === 0);
  const listItems = computed(() => layouts.value.map(toListItem));
  const selected = computed(() => layouts.value.find((l) => l.id === selectedId.value) ?? null);

  /** Reads the config. The first layout is selected when nothing is yet. */
  async function load() {
    try {
      const config = await loadConfig();
      layouts.value = config.layouts;
      aliases.value = config.aliases;
      loadError.value = null;
      if (selected.value === null) {
        selectedId.value = config.layouts[0]?.id ?? null;
      }
    } catch (cause) {
      loadError.value = errorMessage(cause);
    }
  }

  /** Runs the probe and keeps its result as the latest picture of the monitors. */
  async function probe() {
    probing.value = true;
    try {
      inventory.value = await runProbe();
      probeError.value = null;
    } catch (cause) {
      probeError.value = errorMessage(cause);
    } finally {
      probing.value = false;
    }
  }

  /**
   * Captures the latest probe under `name`. A saved layout replaces or joins the list
   * and becomes the selected one; a name conflict is returned for the caller to
   * confirm. Failures throw so the caller can show them where they happened.
   */
  async function capture(name: string, replaceId: string | null): Promise<CaptureOutcome> {
    const outcome = await captureLayout(name, replaceId);
    if (outcome.outcome === 'saved') {
      layouts.value = upsertLayout(layouts.value, outcome.layout);
      selectedId.value = outcome.layout.id;
    }
    return outcome;
  }

  function select(id: string) {
    selectedId.value = id;
  }

  return {
    layouts,
    aliases,
    selectedId,
    inventory,
    probing,
    loadError,
    probeError,
    isEmpty,
    listItems,
    selected,
    load,
    probe,
    capture,
    select,
  };
});
