import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { errorMessage } from '@/domain/errors';
import type { CaptureOutcome, Inventory, Layout, ScriptStatus, SwitchResult } from '@/domain/generated/types';
import { toListItem, upsertLayout } from '@/domain/layouts';
import { appendLog, applyProgress, newSwitchRun, type SwitchRun } from '@/domain/switch';
import {
  cancelSwitch as cancelSwitchScript,
  captureLayout,
  loadConfig,
  onSwitchEvent,
  probe as runProbe,
  regenerateScript as regenerate,
  scriptStates as loadScriptStates,
  switchLayout,
} from '@/tauri/commands';

/**
 * The layouts a user has saved, their aliases, the layout the sidebar has selected,
 * the latest probe, each layout's script state, and the switch in progress:
 * everything more than one feature reads.
 */
export const useLayoutsStore = defineStore('layouts', () => {
  const layouts = ref<Layout[]>([]);
  const aliases = ref<Record<string, string>>({});
  const selectedId = ref<string | null>(null);
  const inventory = ref<Inventory | null>(null);
  const scriptStatuses = ref<ScriptStatus[]>([]);
  const probing = ref(false);
  const loadError = ref<string | null>(null);
  const probeError = ref<string | null>(null);
  /** The switch on screen: running, or finished until Back dismisses it. */
  const switchRun = ref<SwitchRun | null>(null);

  const isEmpty = computed(() => layouts.value.length === 0);
  const listItems = computed(() => layouts.value.map(toListItem));
  const selected = computed(() => layouts.value.find((l) => l.id === selectedId.value) ?? null);
  /** The layout being switched to while a script runs, so other Switch actions stand down. */
  const switching = computed(() => {
    const run = switchRun.value;
    return run && run.result === null ? { layoutId: run.layoutId, layoutName: run.layoutName } : null;
  });

  /** Reads the config and the script states. The first layout is selected when nothing is yet. */
  async function load() {
    try {
      const config = await loadConfig();
      layouts.value = config.layouts;
      aliases.value = config.aliases;
      loadError.value = null;
      if (selected.value === null) {
        selectedId.value = config.layouts[0]?.id ?? null;
      }
      await refreshScriptStates();
    } catch (cause) {
      loadError.value = errorMessage(cause);
    }
  }

  async function refreshScriptStates() {
    scriptStatuses.value = await loadScriptStates();
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
      await refreshScriptStates();
    }
    return outcome;
  }

  /** Rewrites a layout's script and clears its stale state. Failures throw. */
  async function regenerateScript(id: string) {
    const layout = await regenerate(id);
    layouts.value = upsertLayout(layouts.value, layout);
    await refreshScriptStates();
  }

  /**
   * Switches to a layout and resolves when its script has exited. The run appears on
   * the started event, follows every progress and log event, and keeps the result
   * until Back. A refusal before the script starts throws, with no run shown; a
   * failure after it ends the run as failed, so the screen always reaches Back.
   */
  async function switchTo(id: string) {
    const unlisten = await onSwitchEvent((event) => {
      switch (event.kind) {
        case 'started': {
          const name = layouts.value.find((l) => l.id === event.layoutId)?.name ?? '';
          switchRun.value = newSwitchRun({ id: event.layoutId, name }, event.steps, event.applyStep, Date.now());
          break;
        }
        case 'progress':
          if (switchRun.value) {
            switchRun.value.steps = applyProgress(switchRun.value.steps, event.line);
          }
          break;
        case 'log':
          if (switchRun.value) {
            switchRun.value.log = appendLog(switchRun.value.log, event.text);
          }
          break;
      }
    });
    try {
      finishRun(await switchLayout(id));
    } catch (cause) {
      if (switchRun.value === null || switchRun.value.result !== null) {
        throw cause;
      }
      finishRun({ outcome: 'failed', step: null, stepName: '', nextAction: errorMessage(cause), reason: '', exitCode: -1, logPath: '' });
    } finally {
      unlisten();
    }
  }

  function finishRun(result: SwitchResult) {
    if (switchRun.value) {
      switchRun.value.result = result;
      switchRun.value.finishedAt = Date.now();
      switchRun.value.cancelling = false;
    }
  }

  /** Asks the app to kill the running switch. A refusal lands on the run. */
  async function cancelSwitch() {
    const run = switchRun.value;
    if (!run || run.result !== null || run.cancelling) {
      return;
    }
    run.cancelling = true;
    run.error = null;
    try {
      await cancelSwitchScript();
    } catch (cause) {
      run.error = errorMessage(cause);
      run.cancelling = false;
    }
  }

  /** Back to the layout: the finished run leaves the content area. */
  function dismissSwitch() {
    if (switchRun.value?.result !== null) {
      switchRun.value = null;
    }
  }

  function select(id: string) {
    selectedId.value = id;
    if (switchRun.value && switchRun.value.result !== null && switchRun.value.layoutId !== id) {
      switchRun.value = null;
    }
  }

  return {
    layouts,
    aliases,
    selectedId,
    inventory,
    scriptStatuses,
    probing,
    loadError,
    probeError,
    switchRun,
    isEmpty,
    listItems,
    selected,
    switching,
    load,
    probe,
    capture,
    regenerateScript,
    switchTo,
    cancelSwitch,
    dismissSwitch,
    select,
  };
});
