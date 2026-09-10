import { onScopeDispose, ref } from 'vue';
import { errorMessage } from '@/domain/errors';
import { windowSizeToPersist } from '@/domain/window-size';
import { isWindowMaximized, loadConfig, saveWindowSize } from '@/tauri/commands';
import type { WindowSize } from '@/domain/generated/types';

const DEFAULT_DEBOUNCE_MS = 250;

/**
 * Remembers the window size across launches (DESIGN.md, Q7). Reads the persisted size
 * from the config on start, then writes the observed size after each resize settles,
 * following the rule in `domain/window-size.ts`. The webview fills the window, so the
 * document's inner size is the window's logical inner size. A maximised window is not
 * a size the user chose, so it is never written.
 */
export function useWindowSize(options: { debounceMs?: number } = {}) {
  const debounceMs = options.debounceMs ?? DEFAULT_DEBOUNCE_MS;

  const persisted = ref<WindowSize | null>(null);
  const error = ref<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function restore() {
    try {
      const config = await loadConfig();
      persisted.value = config.window;
    } catch (cause) {
      error.value = errorMessage(cause);
    }
  }

  async function persist(observed: WindowSize) {
    const next = windowSizeToPersist(persisted.value, observed);
    if (!next) {
      return;
    }
    try {
      if (await isWindowMaximized()) {
        return;
      }
      await saveWindowSize(next);
      persisted.value = next;
      error.value = null;
    } catch (cause) {
      error.value = errorMessage(cause);
    }
  }

  function onResize() {
    clearTimeout(timer);
    timer = setTimeout(() => {
      void persist({ width: window.innerWidth, height: window.innerHeight });
    }, debounceMs);
  }

  window.addEventListener('resize', onResize);
  void restore();

  onScopeDispose(() => {
    clearTimeout(timer);
    window.removeEventListener('resize', onResize);
  });

  return { persisted, error };
}
