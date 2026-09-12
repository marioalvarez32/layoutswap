import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Capabilities } from '@/domain/generated/types';
import { probe, readCapabilities, setAlias } from '@/tauri/commands';
import { inputSourcesFixture, inventoryFixture, layoutFixture } from '@/test/fixtures';
import { useLayoutsStore } from '@/features/layouts/layouts.store';
import MonitorsScreen from './MonitorsScreen.vue';
import { useMonitorsStore } from './monitors.store';

vi.mock('@/tauri/commands', async () => (await import('@/test/commands')).commandsMock());

function entry(overrides: Partial<Capabilities> = {}): Capabilities {
  return {
    readAt: '2026-09-11T11:52:00-05:00',
    answered: true,
    inputCodes: [0x11, 0x12, 0x0f],
    powerModes: [1, 5],
    modes: [{ width: 1920, height: 1080, hz: 60 }, { width: 1920, height: 1080, hz: 120 }],
    raw: '(prot(monitor)vcp(60(11 12 0F) D6(01 05)))',
    ...overrides,
  };
}

function setUp() {
  setActivePinia(createPinia());
  const layoutsStore = useLayoutsStore();
  const inventory = inventoryFixture();
  const msi = inventory.monitors.find((m) => m.devicePath === 'path-msi-2')!;
  msi.ddcCi = 'answered';
  msi.powerMode = 4;
  msi.asleep = true;
  layoutsStore.inventory = inventory;
  layoutsStore.layouts = [layoutFixture()];
  layoutsStore.aliases = { 'path-acer': 'Side' };
  layoutsStore.inputSources = inputSourcesFixture();
  const monitorsStore = useMonitorsStore();
  monitorsStore.capabilities = {
    'path-acer': entry(),
    'path-builtin': entry({ answered: false, inputCodes: [], powerModes: [], raw: '' }),
  };
  return { layoutsStore, monitorsStore };
}

function rowOf(wrapper: ReturnType<typeof mount>, reportedName: string) {
  return wrapper.findAll('.monitor-row').find((r) => r.find('.reported').text() === reportedName)!;
}

describe('MonitorsScreen', () => {
  beforeEach(setUp);

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('lists every monitor with its alias, state, note and data columns', () => {
    const wrapper = mount(MonitorsScreen);
    expect(wrapper.find('.subtitle').text()).toBe('5 monitors connected. Aliases are yours; every other column is read from the hardware.');
    expect(wrapper.find('.last-probe').text()).toMatch(/^Last probe \d\d:\d\d/);
    expect(wrapper.find('.cost').text()).toContain('The desktop stutters for about 15 seconds while it runs');
    const acer = rowOf(wrapper, 'KG241Y X1');
    expect(acer.find('.alias-text').text()).toBe('Side');
    expect(acer.find('.chip').text()).toBe('Active');
    expect(acer.find('.note').text()).toBe('In Desk');
    expect(acer.findAll('.grid dd').map((d) => d.text())).toEqual(['NVIDIA GeForce RTX 5070 Laptop GPU', 'HDMI', '0,0', '1920×1080 · 60 Hz', 'HDMI 1']);
    const msi = wrapper.findAll('.monitor-row').find((r) => r.find('.chip').text() === 'Active, asleep')!;
    expect(msi.find('.alias-text').text()).toBe('MSI MP165 E6');
    const ultrawide = rowOf(wrapper, 'VG34VQEL1A');
    expect(ultrawide.find('.chip').text()).toBe('Available');
    expect(ultrawide.find('.note').text()).toBe('Plugged in, Windows is not drawing to it');
    expect(ultrawide.findAll('.grid dd').map((d) => d.text()).slice(2)).toEqual(['not Active', 'not Active', 'unknown']);
  });

  it('shows why a row cannot expand, and expands one that can to its capabilities', async () => {
    const wrapper = mount(MonitorsScreen);
    expect(rowOf(wrapper, 'VG34VQEL1A').find('.muted').text()).toBe('Switch it on in Windows first');
    expect(rowOf(wrapper, 'Built-in display').find('.muted').text()).toBe('Does not answer over DDC-CI');
    const msi = wrapper.findAll('.monitor-row').find((r) => r.find('.chip').text() === 'Active, asleep')!;
    expect(msi.find('.muted').text()).toBe('Not read yet');

    const acer = rowOf(wrapper, 'KG241Y X1');
    expect(acer.find('.muted').exists()).toBe(false);
    expect(acer.find('.expand').text()).toContain('Show capabilities');
    await acer.find('.expand').trigger('click');
    expect(acer.find('.expand').text()).toContain('Hide capabilities');
    expect(acer.findAll('.accepts .chip').map((c) => c.text())).toEqual(['DisplayPort 1', 'HDMI 1', 'HDMI 2']);
    expect(acer.findAll('.accepts .chip').map((c) => c.classes('accent'))).toEqual([false, true, false]);
    expect(acer.find('.wake').text()).toBe('Can be woken by the app');
    expect(acer.find('.wake').classes()).toContain('good');
    expect(acer.findAll('.modes span').map((s) => s.text())).toEqual(['1920 x 1080 at 60, 120 Hz']);
    expect(acer.find('.read-at').text()).toMatch(/^Read on 11 Sep 2026, \d\d:\d\d$/);
    await acer.find('.expand').trigger('click');
    expect(acer.find('.capabilities').exists()).toBe(false);
  });

  it('edits an alias inline and saves it on Enter, or cancels on Escape', async () => {
    const wrapper = mount(MonitorsScreen);
    vi.mocked(setAlias).mockResolvedValueOnce({ 'path-acer': 'Side', 'path-ultrawide': 'Big' });
    const ultrawide = rowOf(wrapper, 'VG34VQEL1A');
    await ultrawide.find('.alias').trigger('click');
    const field = ultrawide.find('input.alias-field');
    expect(field.exists()).toBe(true);
    await field.setValue('Big');
    await field.trigger('keydown', { key: 'Enter' });
    await field.trigger('blur');
    await flushPromises();
    expect(setAlias).toHaveBeenCalledWith('path-ultrawide', 'Big');
    expect(rowOf(wrapper, 'VG34VQEL1A').find('.alias-text').text()).toBe('Big');

    await rowOf(wrapper, 'KG241Y X1').find('.alias').trigger('click');
    await wrapper.find('input.alias-field').trigger('keydown', { key: 'Escape' });
    expect(wrapper.find('input.alias-field').exists()).toBe(false);
    expect(setAlias).toHaveBeenCalledTimes(1);

    // Leaving the field with the alias as it was saves nothing.
    await rowOf(wrapper, 'KG241Y X1').find('.alias').trigger('click');
    await wrapper.find('input.alias-field').trigger('blur');
    expect(setAlias).toHaveBeenCalledTimes(1);
  });

  it('refreshes with a probe and re-checks with a capabilities read, showing the rows being read', async () => {
    const wrapper = mount(MonitorsScreen);
    await wrapper.find('.refresh').trigger('click');
    expect(probe).toHaveBeenCalledTimes(1);

    let finish: (value: Record<string, Capabilities>) => void = () => {};
    vi.mocked(readCapabilities).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          finish = resolve;
        }),
    );
    await wrapper.find('.re-check').trigger('click');
    expect(readCapabilities).toHaveBeenCalledTimes(1);
    expect(wrapper.find('.re-check').text()).toBe('Re-checking');
    expect(wrapper.find('.re-check').attributes('disabled')).toBeDefined();
    expect(rowOf(wrapper, 'KG241Y X1').find('.muted').text()).toBe('Reading capabilities');
    expect(rowOf(wrapper, 'VG34VQEL1A').find('.muted').text()).toBe('Switch it on in Windows first');
    finish({ 'path-acer': entry({ powerModes: [5] }) });
    await flushPromises();
    expect(wrapper.find('.re-check').text()).toBe('Re-check');
    await rowOf(wrapper, 'KG241Y X1').find('.expand').trigger('click');
    expect(rowOf(wrapper, 'KG241Y X1').find('.wake').text()).toBe('Needs a button press to wake');
  });

  it('says the monitors have not been read before the first probe', () => {
    useLayoutsStore().inventory = null;
    const wrapper = mount(MonitorsScreen);
    expect(wrapper.find('.empty').text()).toContain('not been read yet');
    expect(wrapper.findAll('.monitor-row')).toHaveLength(0);
  });
});
