import { vi } from 'vitest';
import type * as commands from '@/tauri/commands';
import type { Config } from '@/domain/generated/types';
import { inventoryFixture, layoutFixture } from './fixtures';

/** The config Rust returns on a fresh machine. */
export function defaultConfig(): Config {
  return {
    schemaVersion: 1,
    window: { width: 1280, height: 860 },
    aliases: {},
    layouts: [],
  };
}

/**
 * A full mock of `@/tauri/commands` with resolved defaults. Use it as the factory of
 * `vi.mock('@/tauri/commands', async () => (await import('@/test/commands')).commandsMock())`
 * (the factory is hoisted, so it must import this file itself), then override per test
 * with `vi.mocked(loadConfig).mockResolvedValue(...)`.
 */
export function commandsMock(): typeof commands {
  return {
    loadConfig: vi.fn<typeof commands.loadConfig>().mockResolvedValue(defaultConfig()),
    saveWindowSize: vi.fn<typeof commands.saveWindowSize>().mockResolvedValue(undefined),
    isWindowMaximized: vi.fn<typeof commands.isWindowMaximized>().mockResolvedValue(false),
    probe: vi.fn<typeof commands.probe>().mockResolvedValue(inventoryFixture()),
    captureLayout: vi.fn<typeof commands.captureLayout>().mockResolvedValue({ outcome: 'saved', layout: layoutFixture() }),
  };
}
