import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Inventory } from '@/domain/generated/types';
import { openScript, regenerateScript } from '@/tauri/commands';
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

  it('shows the script state and offers Open script and Regenerate script', async () => {
    const wrapper = mountDetail();
    expect(wrapper.find('.script-line').text()).toBe('Script up to date');
    expect(wrapper.find('.script-line').classes()).toContain('good');
    const buttons = wrapper.findAll('.actions button').map((b) => b.text());
    expect(buttons).toEqual(['Open script', 'Regenerate script']);

    await wrapper.findAll('.actions button')[0]!.trigger('click');
    await flushPromises();
    expect(openScript).toHaveBeenCalledWith('layout-desk');
    await wrapper.findAll('.actions button')[1]!.trigger('click');
    await flushPromises();
    expect(regenerateScript).toHaveBeenCalledWith('layout-desk');
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
