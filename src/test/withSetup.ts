import { createApp, defineComponent, h } from 'vue';

/**
 * Runs a composable inside a throwaway component so lifecycle hooks and scope disposal
 * behave as they do in the app. Call `unmount()` to trigger cleanup.
 */
export function withSetup<T>(composable: () => T): { result: T; unmount: () => void } {
  let result!: T;
  const Host = defineComponent({
    setup() {
      result = composable();
      return () => h('div');
    },
  });
  const app = createApp(Host);
  app.mount(document.createElement('div'));
  return { result, unmount: () => app.unmount() };
}
