import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { layoutFixture } from '@/test/fixtures';
import ArrangementSchematic from './ArrangementSchematic.vue';

describe('ArrangementSchematic', () => {
  const monitors = layoutFixture().summary.monitors;
  const emptyText = 'Nothing on';

  it('draws one rectangle per on monitor, each labelled so identical panels read apart', () => {
    const wrapper = mount(ArrangementSchematic, { props: { monitors, aliases: {}, emptyText } });
    expect(wrapper.findAll('g.monitor')).toHaveLength(4);
    const labels = wrapper.findAll('text.label').map((t) => t.text()).sort();
    expect(labels).toEqual([
      'Built-in display',
      'KG241Y X1',
      'MSI MP165 E6 · USB-C DisplayPort 1',
      'MSI MP165 E6 · USB-C DisplayPort 2',
    ]);
  });

  it('marks the primary', () => {
    const wrapper = mount(ArrangementSchematic, { props: { monitors, aliases: {}, emptyText } });
    const primary = wrapper.findAll('g.monitor.primary');
    expect(primary).toHaveLength(1);
    expect(primary[0]!.find('text.label').text()).toBe('KG241Y X1');
    expect(primary[0]!.find('text.detail').text()).toBe('primary');
  });

  it('labels with the alias when one is set', () => {
    const wrapper = mount(ArrangementSchematic, { props: { monitors, aliases: { 'path-acer': 'Side' }, emptyText } });
    expect(wrapper.findAll('text.label').map((t) => t.text())).toContain('Side');
  });

  it('is a picture, not a control', () => {
    const wrapper = mount(ArrangementSchematic, { props: { monitors, aliases: {}, emptyText } });
    expect(wrapper.find('svg').attributes('role')).toBe('img');
    expect(wrapper.findAll('button, a, [tabindex]')).toHaveLength(0);
  });

  it('shows the given text when nothing is on', () => {
    const allOff = monitors.map((m) => ({ ...m, on: false, position: null, size: null, primary: false }));
    const wrapper = mount(ArrangementSchematic, { props: { monitors: allOff, aliases: {}, emptyText } });
    expect(wrapper.findAll('g.monitor')).toHaveLength(0);
    expect(wrapper.text()).toContain('Nothing on');
  });
});
