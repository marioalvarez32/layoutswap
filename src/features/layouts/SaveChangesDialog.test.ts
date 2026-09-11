import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import SaveChangesDialog from './SaveChangesDialog.vue';

describe('SaveChangesDialog', () => {
  it('names the layout and offers Save, Discard and Keep editing', async () => {
    const wrapper = mount(SaveChangesDialog, { props: { open: true, layoutName: 'Desk' } });
    expect(wrapper.find('h3').text()).toBe('Save changes to Desk?');
    expect(wrapper.find('dialog').attributes('open')).toBeDefined();
    await wrapper.find('form').trigger('submit');
    expect(wrapper.emitted('save')).toHaveLength(1);
    await wrapper.find('button.discard').trigger('click');
    expect(wrapper.emitted('discard')).toHaveLength(1);
    await wrapper.find('button.keep').trigger('click');
    expect(wrapper.emitted('keep')).toHaveLength(1);
  });

  it('stays closed until asked and closes again', async () => {
    const wrapper = mount(SaveChangesDialog, { props: { open: false, layoutName: 'Desk' } });
    expect(wrapper.find('dialog').attributes('open')).toBeUndefined();
    await wrapper.setProps({ open: true });
    expect(wrapper.find('dialog').attributes('open')).toBeDefined();
    await wrapper.setProps({ open: false });
    expect(wrapper.find('dialog').attributes('open')).toBeUndefined();
  });

  it('shows a refused save and holds the buttons while saving', () => {
    const wrapper = mount(SaveChangesDialog, { props: { open: true, layoutName: 'Desk', saving: true, error: 'Keep a wait step between 1 and 600 seconds.' } });
    expect(wrapper.find('.band.crit').text()).toContain('between 1 and 600');
    expect(wrapper.find('button.save').attributes('disabled')).toBeDefined();
    expect(wrapper.find('button.keep').attributes('disabled')).toBeDefined();
  });
});
