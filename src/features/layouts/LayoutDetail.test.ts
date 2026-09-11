import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Inventory } from '@/domain/generated/types';
import { newSwitchRun } from '@/domain/switch';
import { openLog, openScript, regenerateScript, switchLayout } from '@/tauri/commands';
import { inventoryFixture, layoutFixture } from '@/test/fixtures';
import { useLayoutsStore } from './layouts.store';
import LayoutDetail from './LayoutDetail.vue';

vi.mock('@/tauri/commands', async () => (await import('@/test/commands')).commandsMock());

function mountDetail(inventory: Inventory | null = inventoryFixture(), aliases: Record<string, string> = {}) {
  return mount(LayoutDetail, { props: { layout: layoutFixture(), aliases, inventory } });
}

describe('LayoutDetail', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    const store = useLayoutsStore();
    store.layouts = [layoutFixture()];
    store.selectedId = 'layout-desk';
    store.scriptStatuses = [{ layoutId: 'layout-desk', state: 'current', path: 'C:/x/switch.ps1' }];
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('shows the script state and offers Open script, Open log, Regenerate script and Switch', async () => {
    const wrapper = mountDetail();
    expect(wrapper.find('.script-line').text()).toBe('Script up to date');
    expect(wrapper.find('.script-line').classes()).toContain('good');
    const buttons = wrapper.findAll('.actions button').map((b) => b.text());
    expect(buttons).toEqual(['Open script', 'Open log', 'Regenerate script', 'Switch to Desk']);

    await wrapper.findAll('.actions button')[0]!.trigger('click');
    await flushPromises();
    expect(openScript).toHaveBeenCalledWith('layout-desk');
    await wrapper.findAll('.actions button')[1]!.trigger('click');
    await flushPromises();
    expect(openLog).toHaveBeenCalledWith('layout-desk');
    await wrapper.findAll('.actions button')[2]!.trigger('click');
    await flushPromises();
    expect(regenerateScript).toHaveBeenCalledWith('layout-desk');
  });

  it('starts the switch from the primary action', async () => {
    const wrapper = mountDetail();
    const switchButton = wrapper.findAll('.actions button')[3]!;
    expect(switchButton.classes()).toContain('primary');
    expect(switchButton.attributes('disabled')).toBeUndefined();
    await switchButton.trigger('click');
    await flushPromises();
    expect(switchLayout).toHaveBeenCalledWith('layout-desk');
  });

  it('disables Switch while another layout is switching and says which', () => {
    const store = useLayoutsStore();
    store.switchRun = newSwitchRun({ id: 'layout-film', name: 'Film' }, ['Check monitors'], 2, Date.now());
    const wrapper = mountDetail();
    const switchButton = wrapper.findAll('.actions button')[3]!;
    expect(switchButton.attributes('disabled')).toBeDefined();
    expect(wrapper.find('.blocked').text()).toBe('Wait for the switch to Film to finish.');
  });

  it('shows a refused switch in the band', async () => {
    vi.mocked(switchLayout).mockRejectedValueOnce({ message: 'Wait for the switch to Film to finish, then try again.', logPath: null });
    const wrapper = mountDetail();
    await wrapper.findAll('.actions button')[3]!.trigger('click');
    await flushPromises();
    expect(wrapper.find('.band.crit').text()).toContain('Wait for the switch to Film');
  });

  it('shows the name, when it was captured, and the read-only note', () => {
    const wrapper = mountDetail();
    expect(wrapper.find('h2').text()).toBe('Desk');
    expect(wrapper.text()).toContain('Captured 9 Sep at');
    expect(wrapper.text()).toContain('Edit in Windows Settings > Display, then save again.');
  });

  it('draws the schematic beside the summary table', () => {
    const wrapper = mountDetail();
    expect(wrapper.find('.arrangement svg.schematic').exists()).toBe(true);
  });

  it('lists the on monitors with their spec and marks the primary', () => {
    const wrapper = mountDetail();
    const rows = wrapper.findAll('.spec-row');
    expect(rows).toHaveLength(4);
    const acer = rows.find((r) => r.text().includes('KG241Y X1'))!;
    expect(acer.text()).toContain('Primary');
    expect(acer.find('.data').text()).toBe('0,0 · 1920×1080 · 60 Hz · landscape · 100%');
    const builtIn = rows.find((r) => r.text().includes('Built-in display'))!;
    expect(builtIn.find('.data').text()).toBe('1920,0 · 2560×1600 · 165 Hz · landscape · 150%');
  });

  it('lists the off monitors as chips with the one-line explanation', () => {
    const wrapper = mountDetail();
    const off = wrapper.find('.off-row');
    expect(off.findAll('.chip').map((c) => c.text())).toEqual(['VG34VQEL1A']);
    expect(off.text()).toContain('An off monitor stays connected but has no position.');
  });

  it('names the connector on an off chip when another monitor shares the name', () => {
    const layout = layoutFixture();
    const secondMsi = layout.summary.monitors.find((m) => m.devicePath === 'path-msi-2')!;
    Object.assign(secondMsi, { on: false, position: null, size: null, refreshHz: null, rotation: null, scalePercent: null });
    const wrapper = mount(LayoutDetail, { props: { layout, aliases: {}, inventory: inventoryFixture() } });
    const chips = wrapper.find('.off-row').findAll('.chip').map((c) => c.text());
    expect(chips).toContain('MSI MP165 E6 · USB-C DisplayPort 2');
    expect(chips).toContain('VG34VQEL1A');
    expect(wrapper.findAll('.spec-name').map((n) => n.text())).toContain('MSI MP165 E6 · USB-C DisplayPort 1');
  });

  it('tells two identical on panels apart in the summary table', () => {
    const wrapper = mountDetail();
    const names = wrapper.findAll('.spec-name').map((n) => n.text());
    expect(names).toContain('MSI MP165 E6 · USB-C DisplayPort 1');
    expect(names).toContain('MSI MP165 E6 · USB-C DisplayPort 2');
    expect(names).toContain('KG241Y X1');
  });

  it('shows each monitor with its live state chip', () => {
    const inventory = inventoryFixture();
    const ultrawide = inventory.monitors.find((m) => m.devicePath === 'path-ultrawide')!;
    ultrawide.state = 'Absent';
    const wrapper = mountDetail(inventory);
    const rows = wrapper.findAll('.row');
    expect(rows).toHaveLength(5);
    const states = Object.fromEntries(rows.map((r) => [r.find('.name').text() + '|' + r.find('.detail').text(), r.find('.chip').text()]));
    expect(states['KG241Y X1|HDMI']).toBe('Active');
    expect(states['VG34VQEL1A|DisplayPort']).toBe('Absent');
    const ultrawideRow = rows.find((r) => r.text().includes('VG34VQEL1A'))!;
    expect(ultrawideRow.find('.hint').text()).toContain('is Absent');
    expect(ultrawideRow.find('.on-off').text()).toBe('Off');
  });

  it('shows the current input source where DDC-CI answered', () => {
    const wrapper = mountDetail();
    const rows = wrapper.findAll('.row');
    const acer = rows.find((r) => r.text().includes('KG241Y X1'))!;
    expect(acer.find('.input').text()).toBe('HDMI 1');
    expect(acer.find('.input').attributes('title')).toBe('Input source code 0x11');
    const builtIn = rows.find((r) => r.text().includes('Built-in display'))!;
    expect(builtIn.find('.input').exists()).toBe(false);
    const ultrawide = rows.find((r) => r.text().includes('VG34VQEL1A'))!;
    expect(ultrawide.find('.input').exists()).toBe(false);
  });

  it('shows aliases where they exist', () => {
    const wrapper = mountDetail(inventoryFixture(), { 'path-acer': 'Side' });
    expect(wrapper.findAll('.spec-name').map((n) => n.text())).toContain('Side');
    expect(wrapper.findAll('.name').map((n) => n.text())).toContain('Side');
  });

  it('shows no state chips before the first probe', () => {
    const wrapper = mountDetail(null);
    expect(wrapper.findAll('.row .chip')).toHaveLength(0);
  });
});
