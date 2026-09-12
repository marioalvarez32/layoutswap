import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { errorMessage } from '@/domain/errors';
import type { CaptureOutcome, InputSource, Inventory, Layout, LayoutEdits, ScriptStatus, SwitchResult } from '@/domain/generated/types';
import { toListItem, upsertLayout } from '@/domain/layouts';
import { DEFAULT_EDITS, editsOf, isDirty } from '@/domain/steps';
import { appendLog, applyProgress, newSwitchRun, type SwitchRun } from '@/domain/switch';
import { useMonitorsStore } from '@/features/monitors/monitors.store';
import {
  cancelSwitch as cancelSwitchScript,
  captureLayout,
  exportConfig as exportConfigFile,
  importConfig as importConfigFile,
  inputSources as loadInputSources,
  loadConfig,
  onSwitchEvent,
  openDisplaySettings as openDisplaySettingsPage,
  openLog as openSwitchLog,
  probe as runProbe,
  regenerateScript as regenerate,
  saveDiagnostics as saveDiagnosticsZip,
  saveLayout as storeLayoutEdits,
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
  /** The fixed input source table, for the step editor. */
  const inputSources = ref<InputSource[]>([]);
  /** The layout editor's unsaved edits, for one layout at a time. */
  const draft = ref<{ layoutId: string; edits: LayoutEdits } | null>(null);
  /** An action on the result screen is in flight, such as the save dialog. */
  const resultActionBusy = ref(false);
  /** Export or import is in flight, dialog included. */
  const transferBusy = ref(false);
  /** What the last export or import did, for the sidebar footer. */
  const transferNote = ref<string | null>(null);
  const transferError = ref<string | null>(null);

  const isEmpty = computed(() => layouts.value.length === 0);
  const listItems = computed(() => layouts.value.map(toListItem));
  const selected = computed(() => layouts.value.find((l) => l.id === selectedId.value) ?? null);
  /** Whether the draft would change its layout: the shell asks before leaving it. */
  const dirty = computed(() => draft.value !== null && isDraftDirty(draft.value.layoutId));
  /** The name of the layout with unsaved changes, for the dialog. */
  const dirtyLayoutName = computed(() => layouts.value.find((l) => l.id === draft.value?.layoutId)?.name ?? '');

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
      draft.value = null;
      loadError.value = null;
      if (selected.value === null) {
        selectedId.value = config.layouts[0]?.id ?? null;
      }
      await refreshScriptStates();
      inputSources.value = await loadInputSources();
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
      // First sight: a monitor without capabilities gets them read now, in the
      // background; the probe itself never waits for it.
      void useMonitorsStore().readMissing();
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
      // A re-capture kept the saved steps; a draft on top of the old capture is stale.
      if (draft.value?.layoutId === outcome.layout.id) {
        draft.value = null;
      }
      await refreshScriptStates();
    }
    return outcome;
  }

  /** The edits the editor shows for a layout: its draft, or the layout as stored. */
  function editsFor(id: string): LayoutEdits {
    if (draft.value?.layoutId === id) {
      return draft.value.edits;
    }
    const layout = layouts.value.find((l) => l.id === id);
    return layout ? editsOf(layout) : DEFAULT_EDITS;
  }

  function isDraftDirty(id: string): boolean {
    const layout = layouts.value.find((l) => l.id === id);
    return draft.value?.layoutId === id && layout !== undefined && isDirty(draft.value.edits, layout);
  }

  /** Replaces the draft for a layout. A draft equal to the layout is dropped. */
  function edit(id: string, edits: LayoutEdits) {
    const layout = layouts.value.find((l) => l.id === id);
    draft.value = layout && !isDirty(edits, layout) ? null : { layoutId: id, edits };
  }

  /** Saves the draft onto its layout and regenerates the script. Failures throw; the draft stays. */
  async function saveDraft() {
    const current = draft.value;
    if (!current) {
      return;
    }
    const layout = await storeLayoutEdits(current.layoutId, current.edits);
    layouts.value = upsertLayout(layouts.value, layout);
    if (draft.value === current) {
      draft.value = null;
    }
    await refreshScriptStates();
  }

  function discardDraft() {
    draft.value = null;
  }

  /**
   * Re-captures a layout's arrangement from a fresh probe under its own name, keeping
   * its steps, timings and fallback. Failures throw. Refused while the layout has
   * unsaved edits, so they are not lost under the new capture.
   */
  async function recapture(id: string) {
    const layout = layouts.value.find((l) => l.id === id);
    if (!layout) {
      throw new Error('Pick the layout again from the sidebar: it is not in the list any more.');
    }
    if (isDraftDirty(id)) {
      throw new Error('Save or discard the layout\'s changes first, then re-capture.');
    }
    await probe();
    if (probeError.value) {
      throw new Error(probeError.value);
    }
    await capture(layout.name, id);
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
      finishRun({ outcome: 'failed', step: null, stepName: '', nextAction: errorMessage(cause), reason: '', exitCode: -1, logPath: '', explanation: { kind: 'none' } });
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

  /**
   * Runs one action of the finished result screen: Open log, Save diagnostics or
   * Open Settings > Display. Success may leave a notice; a refusal lands on the run.
   */
  async function resultAction(action: 'openLog' | 'saveDiagnostics' | 'openDisplaySettings') {
    const run = switchRun.value;
    if (!run || run.result === null || resultActionBusy.value) {
      return;
    }
    resultActionBusy.value = true;
    run.error = null;
    run.notice = null;
    try {
      switch (action) {
        case 'openLog':
          await openSwitchLog(run.layoutId);
          break;
        case 'openDisplaySettings':
          await openDisplaySettingsPage();
          break;
        case 'saveDiagnostics': {
          const path = await saveDiagnosticsZip(run.layoutId);
          run.notice = path === null ? null : `Diagnostics saved to ${path}`;
          break;
        }
      }
    } catch (cause) {
      run.error = errorMessage(cause);
    } finally {
      resultActionBusy.value = false;
    }
  }

  /** Export: the config as one file, wherever the user says. */
  async function exportConfig() {
    await transfer(async () => {
      const path = await exportConfigFile();
      transferNote.value = path === null ? null : `Exported to ${path}`;
    });
  }

  /**
   * Import: replaces the layouts and aliases with the chosen file's, selects the first
   * layout, and reloads the script states. Refused while a switch runs.
   */
  async function importConfig() {
    if (switching.value) {
      transferError.value = `Wait for the switch to ${switching.value.layoutName} to finish, then import again.`;
      return;
    }
    await transfer(async () => {
      const config = await importConfigFile();
      if (config === null) {
        return;
      }
      layouts.value = config.layouts;
      aliases.value = config.aliases;
      await useMonitorsStore().load();
      selectedId.value = config.layouts[0]?.id ?? null;
      switchRun.value = null;
      draft.value = null;
      await refreshScriptStates();
      transferNote.value = `Imported ${config.layouts.length} ${config.layouts.length === 1 ? 'layout' : 'layouts'}`;
    });
  }

  async function transfer(action: () => Promise<void>) {
    if (transferBusy.value) {
      return;
    }
    transferBusy.value = true;
    transferError.value = null;
    transferNote.value = null;
    try {
      await action();
    } catch (cause) {
      transferError.value = errorMessage(cause);
    } finally {
      transferBusy.value = false;
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
    inputSources,
    probing,
    loadError,
    probeError,
    switchRun,
    draft,
    dirty,
    dirtyLayoutName,
    resultActionBusy,
    transferBusy,
    transferNote,
    transferError,
    isEmpty,
    listItems,
    selected,
    switching,
    load,
    probe,
    capture,
    recapture,
    editsFor,
    isDraftDirty,
    edit,
    saveDraft,
    discardDraft,
    regenerateScript,
    switchTo,
    cancelSwitch,
    resultAction,
    exportConfig,
    importConfig,
    dismissSwitch,
    select,
  };
});
