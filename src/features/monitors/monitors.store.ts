import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { Capabilities } from '@/domain/generated/types';
import { errorMessage } from '@/domain/errors';
import { loadConfig, readCapabilities, readMissingCapabilities } from '@/tauri/commands';

/**
 * What each monitor declared it can do (CONTEXT.md: Capabilities), keyed by device
 * path, and the read that fills it: on first sight after a probe, or on Re-check.
 * The Monitors screen reads from here; the layouts store calls for the first-sight
 * read after every probe and for a reload after an import.
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

  /**
   * First sight: asks the app to read every Active monitor without an entry. Nothing
   * happens when every monitor is known, or while a read already runs.
   */
  async function readMissing() {
    await read(readMissingCapabilities);
  }

  /** Re-check: reads every Active monitor again and replaces its entry. */
  async function reCheck() {
    await read(readCapabilities);
  }

  async function read(run: () => Promise<Record<string, Capabilities> | null>) {
    if (reading.value) {
      return;
    }
    reading.value = true;
    try {
      const stored = await run();
      if (stored !== null) {
        capabilities.value = stored;
      }
      readError.value = null;
    } catch (cause) {
      readError.value = errorMessage(cause);
    } finally {
      reading.value = false;
    }
  }

  return { capabilities, reading, readError, load, readMissing, reCheck };
});
