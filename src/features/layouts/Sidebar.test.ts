import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import Sidebar from './Sidebar.vue';

const layouts = [
  { id: 'a', name: 'Desk', onCount: 3, offCount: 1 },
  { id: 'b', name: 'Console', onCount: 2, offCount: 2 },
];

describe('Sidebar', () => {
  describe('with no layouts', () => {
    const wrapper = mount(Sidebar, { props: { layouts: [], selectedId: null, lastProbe: null } });

    it('puts Save current layout first', () => {
      const first = wrapper.find('button');
      expect(first.text()).toBe('Save current layout');
      expect(first.attributes('disabled')).toBeUndefined();
    });

    it('says there are no layouts yet', () => {
      expect(wrapper.text()).toContain('No layouts yet');
      expect(wrapper.findAll('.row')).toHaveLength(0);
    });

    it('pins Monitors live, Audio and Remote Desktop disabled, then Export config and Import config enabled', async () => {
      const pinned = wrapper.findAll('.pinned-item');
      expect(pinned.map((b) => b.text())).toEqual(['Monitors', 'Audio', 'Remote Desktop', 'Export config', 'Import config']);
      expect(wrapper.find('.monitors').attributes('disabled')).toBeUndefined();
      await wrapper.find('.monitors').trigger('click');
      expect(wrapper.emitted('open')).toEqual([['monitors']]);
      for (const item of pinned.slice(1, 3)) {
        expect(item.attributes('disabled')).toBeDefined();
      }
      expect(wrapper.find('.export').attributes('disabled')).toBeUndefined();
      expect(wrapper.find('.import').attributes('disabled')).toBeUndefined();
      await wrapper.find('.export').trigger('click');
      await wrapper.find('.import').trigger('click');
      expect(wrapper.emitted('export')).toHaveLength(1);
      expect(wrapper.emitted('import')).toHaveLength(1);
    });

    it('shows what the last transfer did, or why it was refused, under the items', () => {
      const noted = mount(Sidebar, { props: { layouts: [], selectedId: null, lastProbe: null, transferNote: 'Imported 2 layouts' } });
      expect(noted.find('.transfer-line').text()).toBe('Imported 2 layouts');
      const refused = mount(Sidebar, { props: { layouts: [], selectedId: null, lastProbe: null, transferNote: 'Imported 2 layouts', transferError: 'Pick another file to import: x was refused, it is not a layoutswap config.', transferBusy: true } });
      expect(refused.find('.transfer-line').text()).toContain('Pick another file');
      expect(refused.find('.transfer-line').classes()).toContain('crit');
      expect(refused.find('.export').attributes('disabled')).toBeDefined();
    });

    it('says the probe has not run yet', () => {
      expect(wrapper.text()).toContain('Last probe');
      expect(wrapper.text()).toContain('not run yet');
    });

    it('emits save when Save current layout is pressed', async () => {
      await wrapper.find('button').trigger('click');
      expect(wrapper.emitted('save')).toHaveLength(1);
    });
  });

  describe('with layouts', () => {
    const wrapper = mount(Sidebar, { props: { layouts, selectedId: 'b', lastProbe: '14:32 today' } });

    it('renders one row per layout with its on and off counts', () => {
      const rows = wrapper.findAll('.row');
      expect(rows.map((r) => r.find('.name').text())).toEqual(['Desk', 'Console']);
      expect(rows.map((r) => r.find('.count').text())).toEqual(['3 on · 1 off', '2 on · 2 off']);
    });

    it('marks the selected layout', () => {
      const rows = wrapper.findAll('.row');
      expect(rows[0]?.attributes('aria-current')).toBeUndefined();
      expect(rows[1]?.attributes('aria-current')).toBe('true');
    });

    it('emits select with the layout id when a row is pressed', async () => {
      await wrapper.findAll('.row')[0]?.trigger('click');
      expect(wrapper.emitted('select')).toEqual([['a']]);
    });

    it('does not show the empty message', () => {
      expect(wrapper.text()).not.toContain('No layouts yet');
    });
  });
});
