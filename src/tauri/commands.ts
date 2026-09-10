// The only file that talks to Tauri: one typed wrapper per Rust command, plus the
// window queries the renderer needs. Argument and result types come from the generated
// bindings. Tests mock this module.
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { CaptureOutcome, Config, Inventory, Layout, ScriptStatus, WindowSize } from '@/domain/generated/types';

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

/** Saves the latest probe as a layout; with `replaceId`, re-captures that layout. */
export function captureLayout(name: string, replaceId: string | null): Promise<CaptureOutcome> {
  return invoke<CaptureOutcome>('capture_layout', { name, replaceId });
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
