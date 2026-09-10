import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { captureLayout, probe } from '@/tauri/commands';
import { inventoryFixture, layoutFixture } from '@/test/fixtures';
import { useLayoutsStore } from './layouts.store';
import SaveLayoutPage from './SaveLayoutPage.vue';

vi.mock('@/tauri/commands', async () => (await import('@/test/commands')).commandsMock());

async function mountPage() {
  const wrapper = mount(SaveLayoutPage);
  await flushPromises();
  return wrapper;
}

describe('SaveLayoutPage', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.mocked(probe).mockResolvedValue(inventoryFixture());
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('probes when opened and lists every connected monitor with its on or off state', async () => {
    const wrapper = await mountPage();
    expect(probe).toHaveBeenCalledTimes(1);
    const rows = wrapper.findAll('.row');
    expect(rows).toHaveLength(5);
    expect(rows.map((r) => r.find('.chip').text())).toEqual(['On', 'On', 'On', 'On', 'Off']);
    expect(wrapper.text()).toContain('read from Windows');
    expect(wrapper.text()).toContain('Arrangement is edited in Windows Settings > Display before saving.');
  });

  it('shows size, position and the primary marker for on monitors only', async () => {
    const wrapper = await mountPage();
    const rows = wrapper.findAll('.row');
    const acer = rows.find((r) => r.text().includes('KG241Y X1'))!;
    expect(acer.text()).toContain('1920×1080 · 60 Hz');
    expect(acer.text()).toContain('0,0');
    expect(acer.text()).toContain('Primary');
    const ultrawide = rows.find((r) => r.text().includes('VG34VQEL1A'))!;
    expect(ultrawide.findAll('.data').map((d) => d.text())).toEqual(['', '']);
  });

  it('tells identical panels apart by connector, and shows the alias when one is set', async () => {
    useLayoutsStore().aliases = { 'path-msi-1': 'Portrait' };
    const wrapper = await mountPage();
    const details = wrapper.findAll('.row').map((r) => `${r.find('.name').text()} / ${r.find('.detail').text()}`);
    expect(details).toContain('MSI MP165 E6 / USB-C DisplayPort 2');
    expect(details).toContain('Portrait / MSI MP165 E6');
  });

  it('disables Save until the name passes the rules and shows the rule inline', async () => {
    const wrapper = await mountPage();
    const submit = wrapper.find('button[type="submit"]');
    expect(submit.attributes('disabled')).toBeDefined();
    await wrapper.find('input').setValue('x'.repeat(41));
    expect(wrapper.text()).toContain('Keep the name to 40 characters or fewer.');
    expect(submit.attributes('disabled')).toBeDefined();
    await wrapper.find('input').setValue('Film');
    expect(submit.attributes('disabled')).toBeUndefined();
  });

  it('emits saved with the stored layout', async () => {
    const wrapper = await mountPage();
    await wrapper.find('input').setValue('Film');
    await wrapper.find('form').trigger('submit');
    await flushPromises();
    expect(captureLayout).toHaveBeenCalledWith('Film', null);
    expect(wrapper.emitted('saved')?.[0]?.[0]).toMatchObject({ name: 'Desk' });
  });

  it('offers to replace a layout that already has the name', async () => {
    useLayoutsStore().layouts = [layoutFixture()];
    const wrapper = await mountPage();
    await wrapper.find('input').setValue('desk');
    await wrapper.find('form').trigger('submit');
    await flushPromises();
    expect(captureLayout).not.toHaveBeenCalled();
    const band = wrapper.find('[role="alertdialog"]');
    expect(band.text()).toContain('A layout called Desk already exists.');
    const replace = band.findAll('button').find((b) => b.text().startsWith('Replace'))!;
    await replace.trigger('click');
    await flushPromises();
    expect(captureLayout).toHaveBeenCalledWith('desk', 'layout-desk');
  });

  it('emits cancel', async () => {
    const wrapper = await mountPage();
    const cancel = wrapper.findAll('button').find((b) => b.text() === 'Cancel')!;
    await cancel.trigger('click');
    expect(wrapper.emitted('cancel')).toHaveLength(1);
  });
});
