import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { Step } from '@/domain/generated/types';
import StepsEditor from './StepsEditor.vue';

function wait(id: string, side: Step['side'], seconds: number): Step {
  return { id, side, kind: 'wait', seconds };
}

const STEPS: Step[] = [wait('a', 'before', 3), wait('b', 'before', 5), wait('c', 'after', 1)];

function lastChange(wrapper: ReturnType<typeof mount>): Step[] {
  const events = wrapper.emitted('change') as Step[][][];
  return events[events.length - 1]![0]!;
}

describe('StepsEditor', () => {
  it('shows the timeline: fixed rows with the steps as sentences around the apply', () => {
    const wrapper = mount(StepsEditor, { props: { steps: STEPS } });
    const rows = wrapper.findAll('.timeline > li').map((li) => li.text().trim());
    expect(rows).toEqual([
      'Check monitors',
      'Wait 3 seconds',
      'Wait 5 seconds',
      'Add step before',
      'Apply arrangement',
      'Wait 1 second',
      'Add step after',
      'Verify',
    ]);
    expect(wrapper.findAll('.controls')).toHaveLength(0);
  });

  it('keeps the fixed rows and the two Add buttons with no steps', () => {
    const wrapper = mount(StepsEditor, { props: { steps: [] } });
    expect(wrapper.findAll('.timeline > li').map((li) => li.text().trim())).toEqual([
      'Check monitors', 'Add step before', 'Apply arrangement', 'Add step after', 'Verify',
    ]);
  });

  it('asks for a step on the side of the button', async () => {
    const wrapper = mount(StepsEditor, { props: { steps: STEPS } });
    await wrapper.find('.add-after').trigger('click');
    await wrapper.find('.add-before').trigger('click');
    expect(wrapper.emitted('add')).toEqual([['after'], ['before']]);
  });

  it('expands a row on click to its controls and edits the seconds', async () => {
    const wrapper = mount(StepsEditor, { props: { steps: STEPS } });
    await wrapper.findAll('.sentence')[0]!.trigger('click');
    expect(wrapper.findAll('.sentence')[0]!.attributes('aria-expanded')).toBe('true');
    const seconds = wrapper.find('input.seconds');
    expect((seconds.element as HTMLInputElement).value).toBe('3');
    await seconds.setValue('9');
    expect(lastChange(wrapper)[0]).toEqual(wait('a', 'before', 9));
    await wrapper.findAll('.sentence')[0]!.trigger('click');
    expect(wrapper.find('.controls').exists()).toBe(false);
  });

  it('expands a row the caller names, as after an add', async () => {
    const wrapper = mount(StepsEditor, { props: { steps: STEPS } });
    (wrapper.vm as unknown as { expand: (id: string) => void }).expand('c');
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll('.step')[2]!.classes()).toContain('expanded');
  });

  it('shows the rule when the seconds leave their bounds', async () => {
    const wrapper = mount(StepsEditor, { props: { steps: [wait('a', 'before', 0)] } });
    await wrapper.find('.sentence').trigger('click');
    expect(wrapper.find('.rule').text()).toContain('between 1 and 600');
    expect(wrapper.find('input.seconds').attributes('aria-invalid')).toBe('true');
  });

  it('moves a step within its side and disables the move at the edge', async () => {
    const wrapper = mount(StepsEditor, { props: { steps: STEPS } });
    await wrapper.findAll('.sentence')[1]!.trigger('click');
    expect(wrapper.find('.move-up').attributes('disabled')).toBeUndefined();
    expect(wrapper.find('.move-down').attributes('disabled')).toBeDefined();
    await wrapper.find('.move-up').trigger('click');
    expect(lastChange(wrapper).map((s) => s.id)).toEqual(['b', 'a', 'c']);
  });

  it('removes a step and closes its controls', async () => {
    const wrapper = mount(StepsEditor, { props: { steps: STEPS } });
    await wrapper.findAll('.sentence')[2]!.trigger('click');
    await wrapper.find('.remove').trigger('click');
    const steps = lastChange(wrapper);
    expect(steps.map((s) => s.id)).toEqual(['a', 'b']);
    await wrapper.setProps({ steps });
    expect(wrapper.find('.controls').exists()).toBe(false);
  });
});
