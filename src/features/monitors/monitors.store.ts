import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { Capabilities } from '@/domain/generated/types';
import { errorMessage } from '@/domain/errors';
import { loadConfig, readCapabilities } from '@/tauri/commands';

/**
 * What each monitor declared it can do (CONTEXT.md: Capabilities), keyed by device
 * path, and the one read that fills it, Re-check. Nothing reads capabilities on its
 * own: the DDC-CI requests stall the desktop while they run. The Monitors screen
 * reads from here; the layouts store asks for a reload after an import.
 */
export const useMonitorsStore = defineStore('monitors', () => {
  const capabilities = ref<Record<string, Capabilities>>({});
  /** A read is running; the rows of the monitors being read show it. */
  const reading = ref(false);
  const readError = ref<string | null>(null);

  /** Reads the stored entries from the config. */
  async function load() {
    try {
      capabilities.value = (await loadConfig()).capabilities;
      readError.value = null;
    } catch (cause) {
      readError.value = errorMessage(cause);
    }
  }

  /** Re-check: reads every Active monitor and replaces its entry; a second press waits. */
  async function reCheck() {
    if (reading.value) {
      return;
    }
    reading.value = true;
    try {
      capabilities.value = await readCapabilities();
      readError.value = null;
    } catch (cause) {
      readError.value = errorMessage(cause);
    } finally {
      reading.value = false;
    }
  }

  return { capabilities, reading, readError, load, reCheck };
});
