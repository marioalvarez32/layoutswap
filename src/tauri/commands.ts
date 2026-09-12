// The only file that talks to Tauri: one typed wrapper per Rust command, plus the
// window queries the renderer needs. Argument and result types come from the generated
// bindings. Tests mock this module.
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Capabilities, CaptureOutcome, Config, InputSource, Inventory, Layout, LayoutEdits, ScriptStatus, SwitchEvent, SwitchResult, WindowSize } from '@/domain/generated/types';

export function loadConfig(): Promise<Config> {
  return invoke<Config>('load_config');
}

export function saveWindowSize(size: WindowSize): Promise<void> {
  return invoke<void>('save_window_size', { size });
}

export function isWindowMaximized(): Promise<boolean> {
  return getCurrentWindow().isMaximized();
}

/** Runs the probe script and returns what Windows shows right now. */
export function probe(): Promise<Inventory> {
  return invoke<Inventory>('probe');
}

/** Sets or clears a monitor's alias and returns the alias map. */
export function setAlias(devicePath: string, alias: string): Promise<Record<string, string>> {
  return invoke<Record<string, string>>('set_alias', { devicePath, alias });
}

/** Re-check: reads every Active monitor's capabilities and returns the stored map. */
export function readCapabilities(): Promise<Record<string, Capabilities>> {
  return invoke<Record<string, Capabilities>>('read_capabilities');
}

/**
 * First sight: reads capabilities only when the latest probe shows an Active monitor
 * without an entry. Null when there was nothing to read.
 */
export function readMissingCapabilities(): Promise<Record<string, Capabilities> | null> {
  return invoke<Record<string, Capabilities> | null>('read_missing_capabilities');
}

/** Saves the latest probe as a layout; with `replaceId`, re-captures that layout. */
export function captureLayout(name: string, replaceId: string | null): Promise<CaptureOutcome> {
  return invoke<CaptureOutcome>('capture_layout', { name, replaceId });
}

/**
 * Saves the editor's steps, timings and fallback onto a layout and regenerates its
 * script. Returns the stored layout. Refused when an edit breaks its bounds.
 */
export function saveLayout(layoutId: string, edits: LayoutEdits): Promise<Layout> {
  return invoke<Layout>('save_layout', { layoutId, edits });
}

/** The fixed input source table the step editor offers. */
export function inputSources(): Promise<InputSource[]> {
  return invoke<InputSource[]>('input_sources');
}

/** Every layout's generated-script state, read from the config and the disk. */
export function scriptStates(): Promise<ScriptStatus[]> {
  return invoke<ScriptStatus[]>('script_states');
}

/** Rewrites a layout's switch script and returns the layout with its new script record. */
export function regenerateScript(layoutId: string): Promise<Layout> {
  return invoke<Layout>('regenerate_script', { layoutId });
}

/** Opens a layout's switch script with whatever Windows associates with .ps1 files. */
export function openScript(layoutId: string): Promise<void> {
  return invoke<void>('open_script', { layoutId });
}

/**
 * Runs a layout's switch script and resolves once it has exited. Progress arrives
 * through `onSwitchEvent` meanwhile; subscribe before calling this.
 */
export function switchLayout(layoutId: string): Promise<SwitchResult> {
  return invoke<SwitchResult>('switch_layout', { layoutId });
}

/** Kills the running switch. Refused once the Apply arrangement step has reported. */
export function cancelSwitch(): Promise<void> {
  return invoke<void>('cancel_switch');
}

/** Opens a layout's switch log with the system default. */
export function openLog(layoutId: string): Promise<void> {
  return invoke<void>('open_log', { layoutId });
}

/** Opens Windows Settings > Display. */
export function openDisplaySettings(): Promise<void> {
  return invoke<void>('open_display_settings');
}

/**
 * Asks where to save a layout's diagnostics zip and writes it. Resolves with the
 * path, or null when the user cancelled the dialog.
 */
export function saveDiagnostics(layoutId: string): Promise<string | null> {
  return invoke<string | null>('save_diagnostics', { layoutId });
}

/** Asks where to save the config and copies it there. Null when the user cancelled. */
export function exportConfig(): Promise<string | null> {
  return invoke<string | null>('export_config');
}

/**
 * Asks which file to import and replaces the config with it, regenerating every
 * script. Resolves with the new config, or null when the user cancelled.
 */
export function importConfig(): Promise<Config | null> {
  return invoke<Config | null>('import_config');
}

/** The event name `commands::SWITCH_EVENT` emits on. */
const SWITCH_EVENT = 'switch-event';

/** Subscribes to the events a running switch emits; resolves with the unsubscribe. */
export function onSwitchEvent(handler: (event: SwitchEvent) => void): Promise<UnlistenFn> {
  return listen<SwitchEvent>(SWITCH_EVENT, (event) => handler(event.payload));
}
