import { mount } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { applyProgress, newSwitchRun, type SwitchRun } from '@/domain/switch';
import SwitchProgress from './SwitchProgress.vue';

const STEPS = ['Check monitors', 'Apply arrangement', 'Verify'];

function running(): SwitchRun {
  const run = newSwitchRun({ id: 'layout-desk', name: 'Desk' }, STEPS, 2, 10_000);
  run.steps = applyProgress(run.steps, { step: 1, of: 3, status: 'done', text: 'Check monitors' });
  run.log = ['=== Switch to Desk ===', '  Side: Available', '  Built-in display: Available'];
  return run;
}

function mountRun(run: SwitchRun) {
  return mount(SwitchProgress, { props: { run } });
}

describe('SwitchProgress', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(17_100);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('while running', () => {
    it('names the layout and shows the elapsed time in the data font', () => {
      const wrapper = mountRun(running());
      expect(wrapper.find('h2').text()).toBe('Switching to Desk');
      expect(wrapper.find('.aside').text()).toBe('7.1 s elapsed');
      vi.advanceTimersByTime(500);
      return wrapper.vm.$nextTick().then(() => {
        expect(wrapper.find('.aside').text()).toBe('7.6 s elapsed');
      });
    });

    it('lists every step with its chip', () => {
      const wrapper = mountRun(running());
      const rows = wrapper.findAll('.step');
      expect(rows.map((r) => r.find('.chip').text())).toEqual(['done', 'waiting', 'waiting']);
      expect(rows.map((r) => r.find('.step-text').text())).toEqual(STEPS);
      expect(rows[0]!.find('.chip').classes()).toContain('good');
    });

    it('shows the log tail', () => {
      const wrapper = mountRun(running());
      expect(wrapper.find('.log').text()).toBe('=== Switch to Desk ===\n  Side: Available\n  Built-in display: Available');
    });

    it('offers Cancel and emits it', async () => {
      const wrapper = mountRun(running());
      const cancel = wrapper.find('button.cancel');
      expect(cancel.text()).toBe('Cancel');
      expect(cancel.attributes('disabled')).toBeUndefined();
      await cancel.trigger('click');
      expect(wrapper.emitted('cancel')).toHaveLength(1);
      expect(wrapper.find('button.back').exists()).toBe(false);
    });

    it('disables Cancel once the apply step reports running', async () => {
      const run = running();
      run.steps = applyProgress(run.steps, { step: 2, of: 3, status: 'running', text: 'Apply arrangement' });
      const wrapper = mountRun(run);
      expect(wrapper.find('button.cancel').attributes('disabled')).toBeDefined();
      expect(wrapper.text()).toContain('Cancel is off while the arrangement is being applied.');
      expect(wrapper.findAll('.step')[1]!.classes()).toContain('running');
    });

    it('shows a refused cancel beside the button', () => {
      const run = running();
      run.error = 'Wait for the switch to Desk to finish: the arrangement is already being applied and cannot be stopped.';
      const wrapper = mountRun(run);
      expect(wrapper.find('.band.warn').text()).toContain('cannot be stopped');
    });
  });

  describe('when done', () => {
    it('says Applied in N s and offers Back to the layout', async () => {
      const run = running();
      run.steps = applyProgress(run.steps, { step: 2, of: 3, status: 'skipped', text: 'Apply arrangement: already applied' });
      run.steps = applyProgress(run.steps, { step: 3, of: 3, status: 'done', text: 'Verify' });
      run.result = { outcome: 'applied', durationMs: 11_400 };
      run.finishedAt = 21_400;
      const wrapper = mountRun(run);
      expect(wrapper.find('h2').text()).toBe('Switched to Desk');
      expect(wrapper.find('.aside').text()).toBe('Applied in 11.4 s');
      expect(wrapper.find('.aside').classes()).toContain('good');
      expect(wrapper.findAll('.step').map((r) => r.find('.chip').text())).toEqual(['done', 'skipped', 'done']);
      expect(wrapper.find('button.cancel').exists()).toBe(false);
      const back = wrapper.find('button.back');
      expect(back.text()).toBe('Back to Desk');
      await back.trigger('click');
      expect(wrapper.emitted('back')).toHaveLength(1);
    });

    it('puts the next action first when the switch failed', () => {
      const run = running();
      run.steps = applyProgress(run.steps, { step: 1, of: 3, status: 'failed', text: 'Check monitors: press the input button on Ultrawide, or plug it in, then switch again; 1 Absent' });
      run.result = { outcome: 'failed', step: 1, stepName: 'Check monitors', nextAction: 'press the input button on Ultrawide, or plug it in, then switch again', reason: '1 Absent', exitCode: 2, logPath: 'C:/x/switch.log' };
      run.finishedAt = 12_500;
      const wrapper = mountRun(run);
      expect(wrapper.find('h2').text()).toBe('Could not switch to Desk');
      expect(wrapper.find('.aside').text()).toBe('Stopped after 2.5 s');
      expect(wrapper.find('.band.crit .action').text()).toBe('Press the input button on Ultrawide, or plug it in, then switch again.');
      expect(wrapper.find('.band.crit .detail').text()).toContain('Check monitors failed: 1 Absent');
      expect(wrapper.findAll('.step')[0]!.find('.chip').text()).toBe('failed');
      expect(wrapper.find('button.back').text()).toBe('Back to Desk');
    });

    it('reads cancelled after a cancel', () => {
      const run = running();
      run.result = { outcome: 'cancelled' };
      run.finishedAt = 13_000;
      const wrapper = mountRun(run);
      expect(wrapper.find('h2').text()).toBe('Switch to Desk cancelled');
      expect(wrapper.find('.band.crit').exists()).toBe(false);
    });
  });
});
