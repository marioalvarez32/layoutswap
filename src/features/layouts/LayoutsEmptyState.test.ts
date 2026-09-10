import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import LayoutsEmptyState from './LayoutsEmptyState.vue';

describe('LayoutsEmptyState', () => {
  const wrapper = mount(LayoutsEmptyState);

  it('says what a layout is and how to create one', () => {
    expect(wrapper.find('h2').text()).toBe('No layouts yet');
    expect(wrapper.text()).toContain('A layout is the arrangement of monitors Windows is showing right now');
    expect(wrapper.text()).toContain('Arrange your monitors in Windows Settings > Display first.');
  });

  it('offers Save current layout as the one action', () => {
    const buttons = wrapper.findAll('button');
    expect(buttons).toHaveLength(1);
    expect(buttons[0]?.text()).toBe('Save current layout');
  });

  it('emits save when the button is pressed', async () => {
    await wrapper.find('button').trigger('click');
    expect(wrapper.emitted('save')).toHaveLength(1);
  });
});
