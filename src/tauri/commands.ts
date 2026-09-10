// The only file that talks to Tauri: one typed wrapper per Rust command, plus the
// window queries the renderer needs. Argument and result types come from the generated
// bindings. Tests mock this module.
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import type { Config, WindowSize } from '@/domain/generated/types';

export function loadConfig(): Promise<Config> {
  return invoke<Config>('load_config');
}

export function saveWindowSize(size: WindowSize): Promise<void> {
  return invoke<void>('save_window_size', { size });
}

export function isWindowMaximized(): Promise<boolean> {
  return getCurrentWindow().isMaximized();
}
