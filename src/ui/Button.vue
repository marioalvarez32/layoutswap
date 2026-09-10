<script setup lang="ts">
withDefaults(defineProps<{
  variant?: 'primary' | 'secondary';
  type?: 'button' | 'submit';
}>(), {
  variant: 'secondary',
  type: 'button',
});
</script>

<template>
  <button
    :type="type"
    class="button"
    :class="variant"
  >
    <slot />
  </button>
</template>

<style scoped>
.button {
  height: var(--control-h);
  padding: 0 var(--space-5);
  border: var(--hairline) solid var(--line-strong);
  border-radius: var(--r);
  background: var(--surface);
  color: var(--ink);
  font-family: var(--body);
  font-size: var(--text-md);
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
}

.button:hover:not(:disabled) {
  border-color: var(--ink-3);
}

.button.primary {
  padding: 0 var(--space-6);
  border-color: var(--accent);
  background: var(--accent);
  color: var(--accent-ink);
}

.button.primary:hover:not(:disabled) {
  border-color: var(--accent-hover);
  background: var(--accent-hover);
}

.button:disabled {
  cursor: default;
  opacity: 0.55;
}
</style>
