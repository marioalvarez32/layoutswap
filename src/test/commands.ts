import { vi } from 'vitest';
import type * as commands from '@/tauri/commands';
import type { Config } from '@/domain/generated/types';
import { inputSourcesFixture, inventoryFixture, layoutFixture } from './fixtures';

/** The config Rust returns on a fresh machine. */
export function defaultConfig(): Config {
  return {
    schemaVersion: 3,
    window: { width: 1280, height: 860 },
    aliases: {},
    capabilities: {},
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
    readCapabilities: vi.fn<typeof commands.readCapabilities>().mockResolvedValue({}),
    readMissingCapabilities: vi.fn<typeof commands.readMissingCapabilities>().mockResolvedValue(null),
    captureLayout: vi.fn<typeof commands.captureLayout>().mockResolvedValue({ outcome: 'saved', layout: layoutFixture() }),
    saveLayout: vi.fn<typeof commands.saveLayout>().mockImplementation(async (_id, edits) => ({ ...layoutFixture(), ...edits })),
    scriptStates: vi.fn<typeof commands.scriptStates>().mockResolvedValue([]),
    inputSources: vi.fn<typeof commands.inputSources>().mockResolvedValue(inputSourcesFixture()),
    regenerateScript: vi.fn<typeof commands.regenerateScript>().mockResolvedValue(layoutFixture()),
    openScript: vi.fn<typeof commands.openScript>().mockResolvedValue(undefined),
    switchLayout: vi.fn<typeof commands.switchLayout>().mockResolvedValue({ outcome: 'applied', durationMs: 11_400 }),
    cancelSwitch: vi.fn<typeof commands.cancelSwitch>().mockResolvedValue(undefined),
    onSwitchEvent: vi.fn<typeof commands.onSwitchEvent>().mockResolvedValue(() => {}),
    openLog: vi.fn<typeof commands.openLog>().mockResolvedValue(undefined),
    openDisplaySettings: vi.fn<typeof commands.openDisplaySettings>().mockResolvedValue(undefined),
    saveDiagnostics: vi.fn<typeof commands.saveDiagnostics>().mockResolvedValue('C:/Users/x/Desktop/layoutswap-diagnostics-desk-20260910-183012.zip'),
    exportConfig: vi.fn<typeof commands.exportConfig>().mockResolvedValue('C:/Users/x/Desktop/layoutswap-config-2026-09-10.json'),
    importConfig: vi.fn<typeof commands.importConfig>().mockResolvedValue({ ...defaultConfig(), layouts: [layoutFixture()] }),
  };
}
