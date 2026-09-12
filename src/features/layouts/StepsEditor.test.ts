import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { InputSource, Step } from '@/domain/generated/types';
import { stepLabeller } from '@/domain/steps';
import { layoutFixture } from '@/test/fixtures';
import StepsEditor from './StepsEditor.vue';

const TABLE: InputSource[] = [
  { code: 0x0f, name: 'DisplayPort 1' },
  { code: 0x11, name: 'HDMI 1' },
];

function wait(id: string, side: Step['side'], seconds: number): Step {
  return { id, side, kind: 'wait', seconds };
}

function send(id: string, side: Step['side'], devicePath: string, inputSource: number, wait: 'none' | 'drop' | 'available' = 'none'): Step {
  return { id, side, kind: 'sendInput', devicePath, inputSource, wait };
}

const STEPS: Step[] = [wait('a', 'before', 3), wait('b', 'before', 5), wait('c', 'after', 1)];
const layout = layoutFixture();
const labelOf = stepLabeller({ 'path-ultrawide': 'Ultrawide' }, layout.summary.monitors);
const monitors = layout.summary.monitors.map((m) => ({
  devicePath: m.devicePath,
  label: labelOf(m.devicePath),
  on: m.on,
  currentInput: m.devicePath === 'path-acer' ? 0x11 : null,
  capabilities: m.devicePath === 'path-acer'
    ? { readAt: '2026-09-11T11:52:00-05:00', answered: true, inputCodes: [0x11, 0x12], powerModes: [1], modes: [], raw: '' }
    : m.devicePath === 'path-msi-1'
      ? { readAt: '2026-09-11T11:52:00-05:00', answered: true, inputCodes: [0x0f], powerModes: [5], modes: [], raw: '' }
      : null,
}));

function mountEditor(steps: Step[]) {
  return mount(StepsEditor, {
    props: { steps, layout, monitors, inputSources: TABLE, labelOf, dropWaitSeconds: 5, availableWaitSeconds: 120 },
  });
}

function lastChange(wrapper: ReturnType<typeof mountEditor>): Step[] {
  const events = wrapper.emitted('change') as Step[][][];
  return events[events.length - 1]![0]!;
}

describe('StepsEditor', () => {
  it('shows the timeline: fixed rows with the steps as sentences around the apply', () => {
    const wrapper = mountEditor(STEPS);
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
    const wrapper = mountEditor([]);
    expect(wrapper.findAll('.timeline > li').map((li) => li.text().trim())).toEqual([
      'Check monitors', 'Add step before', 'Apply arrangement', 'Add step after', 'Verify',
    ]);
  });

  it('asks for a step on the side of the button', async () => {
    const wrapper = mountEditor(STEPS);
    await wrapper.find('.add-after').trigger('click');
    await wrapper.find('.add-before').trigger('click');
    expect(wrapper.emitted('add')).toEqual([['after'], ['before']]);
  });

  it('expands a row on click to its controls and edits the seconds', async () => {
    const wrapper = mountEditor(STEPS);
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
    const wrapper = mountEditor(STEPS);
    (wrapper.vm as unknown as { expand: (id: string) => void }).expand('c');
    await wrapper.vm.$nextTick();
    expect(wrapper.findAll('.step')[2]!.classes()).toContain('expanded');
  });

  it('turns a wait step into a send step and back through the kind select', async () => {
    const wrapper = mountEditor(STEPS);
    await wrapper.findAll('.sentence')[0]!.trigger('click');
    await wrapper.find('select.kind').setValue('sendInput');
    expect(lastChange(wrapper)[0]).toEqual(send('a', 'before', 'path-msi-2', 0x0f));
    await wrapper.setProps({ steps: lastChange(wrapper) });
    expect(wrapper.findAll('.sentence')[0]!.text()).toBe('Send DisplayPort 1 to MSI MP165 E6 · USB-C DisplayPort 2');
    await wrapper.find('select.kind').setValue('wait');
    expect(lastChange(wrapper)[0]).toEqual(wait('a', 'before', 3));
  });

  it('edits a send step: input with the current one marked, monitor, wait rule with its timeout', async () => {
    const wrapper = mountEditor([send('s', 'before', 'path-acer', 0x0f)]);
    await wrapper.find('.sentence').trigger('click');
    // The Acer declares HDMI 1 and HDMI 2, so those lead under its name and the rest
    // of the table follows as not declared; Other code stays last, outside the groups.
    const groups = wrapper.findAll('select.input optgroup');
    expect(groups.map((g) => g.attributes('label'))).toEqual(['Accepted by KG241Y X1', 'Not declared by KG241Y X1']);
    expect(groups[0]!.findAll('option').map((o) => o.text())).toEqual(['HDMI 1 (now)', 'Input 0x12']);
    expect(groups[1]!.findAll('option').map((o) => o.text())).toEqual(['DisplayPort 1']);
    const inputOptions = wrapper.findAll('select.input option').map((o) => o.text());
    expect(inputOptions).toEqual(['HDMI 1 (now)', 'Input 0x12', 'DisplayPort 1', 'Other code']);
    await wrapper.find('select.input').setValue('17');
    expect(lastChange(wrapper)[0]).toMatchObject({ inputSource: 0x11 });
    expect(wrapper.findAll('select.monitor option').map((o) => o.text())).toContain('Ultrawide (off in this layout)');
    await wrapper.find('select.monitor').setValue('path-ultrawide');
    expect(lastChange(wrapper)[0]).toMatchObject({ devicePath: 'path-ultrawide' });
    // The MSI panel declares DisplayPort 1 only, so the groups follow the monitor.
    await wrapper.setProps({ steps: [send('s', 'before', 'path-msi-1', 0x11)] });
    const msiGroups = wrapper.findAll('select.input optgroup');
    expect(msiGroups.map((g) => g.attributes('label'))).toEqual(['Accepted by MSI MP165 E6 · USB-C DisplayPort 1', 'Not declared by MSI MP165 E6 · USB-C DisplayPort 1']);
    expect(msiGroups[0]!.findAll('option').map((o) => o.text())).toEqual(['DisplayPort 1']);
    expect(msiGroups[1]!.findAll('option').map((o) => o.text())).toEqual(['HDMI 1']);
    // The ultrawide has no capabilities, so its list is the plain table again.
    await wrapper.setProps({ steps: [send('s', 'before', 'path-ultrawide', 0x11)] });
    expect(wrapper.findAll('select.input optgroup')).toHaveLength(0);
    expect(wrapper.findAll('select.input option').map((o) => o.text())).toEqual(['DisplayPort 1', 'HDMI 1', 'Other code']);
    await wrapper.find('select.wait').setValue('drop');
    expect(lastChange(wrapper)[0]).toMatchObject({ wait: 'drop' });
    await wrapper.setProps({ steps: [send('s', 'before', 'path-ultrawide', 0x11, 'drop')] });
    expect(wrapper.find('.sentence').text()).toBe('Send HDMI 1 to Ultrawide, then wait until Ultrawide shows HDMI 1 or drops');
    expect(wrapper.find('select.wait ~ .faint').text()).toBe('up to 5 s');
  });

  it('takes another code as hex and refuses one a monitor cannot hold', async () => {
    const wrapper = mountEditor([send('s', 'before', 'path-acer', 0x1e)]);
    await wrapper.find('.sentence').trigger('click');
    expect((wrapper.find('select.input').element as HTMLSelectElement).value).toBe('other');
    const code = wrapper.find('input.other-code');
    expect((code.element as HTMLInputElement).value).toBe('0x1E');
    await code.setValue('0x1B');
    expect(lastChange(wrapper)[0]).toMatchObject({ inputSource: 0x1b });
    await code.setValue('0x100');
    expect(wrapper.find('.rule').text()).toContain('0x01 and 0xFF');
  });

  it('warns under a send step whose monitor is off after the apply, expanded or not', async () => {
    const wrapper = mountEditor([send('s', 'after', 'path-ultrawide', 0x11)]);
    expect(wrapper.find('.note').text()).toBe('Ultrawide is off after the apply, so this step will be skipped. Move it before the apply.');
    await wrapper.find('.sentence').trigger('click');
    expect(wrapper.find('.controls .note').text()).toContain('off after the apply');
  });

  it('shows an unknown monitor as such and keeps it selectable', async () => {
    const wrapper = mountEditor([send('s', 'before', 'gone', 0x11)]);
    expect(wrapper.find('.sentence').text()).toBe('Send HDMI 1 to unknown monitor');
    expect(wrapper.find('.note').text()).toContain('not in the layout any more');
    await wrapper.find('.sentence').trigger('click');
    expect((wrapper.find('select.monitor').element as HTMLSelectElement).value).toBe('gone');
  });

  it('moves a step within its side, across the apply at the edge, and not past the ends', async () => {
    const wrapper = mountEditor(STEPS);
    await wrapper.findAll('.sentence')[1]!.trigger('click');
    expect(wrapper.find('.move-up').attributes('disabled')).toBeUndefined();
    expect(wrapper.find('.move-down').attributes('disabled')).toBeUndefined();
    await wrapper.find('.move-up').trigger('click');
    expect(lastChange(wrapper).map((s) => s.id)).toEqual(['b', 'a', 'c']);
    await wrapper.find('.move-down').trigger('click');
    expect(lastChange(wrapper).map((s) => [s.id, s.side])).toEqual([['a', 'before'], ['b', 'after'], ['c', 'after']]);

    await wrapper.findAll('.sentence')[2]!.trigger('click');
    expect(wrapper.find('.move-down').attributes('disabled')).toBeDefined();
  });

  it('removes a step and closes its controls', async () => {
    const wrapper = mountEditor(STEPS);
    await wrapper.findAll('.sentence')[2]!.trigger('click');
    await wrapper.find('.remove').trigger('click');
    const steps = lastChange(wrapper);
    expect(steps.map((s) => s.id)).toEqual(['a', 'b']);
    await wrapper.setProps({ steps });
    expect(wrapper.find('.controls').exists()).toBe(false);
  });
});
