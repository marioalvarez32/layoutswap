<script setup lang="ts">
withDefaults(defineProps<{
  /** accent for On, mute for Off, good/warn/crit for Active/Available/Absent. */
  tone?: 'accent' | 'mute' | 'good' | 'warn' | 'crit';
}>(), {
  tone: 'mute',
});
</script>

<template>
  <span class="chip" :class="tone">
    <slot />
  </span>
</template>

<style scoped>
.chip {
  display: inline-flex;
  align-items: center;
  padding: var(--space-0) var(--space-2);
  border-radius: var(--r-sm);
  font-family: var(--data);
  font-size: var(--text-xs);
  font-weight: 500;
  line-height: var(--leading);
  white-space: nowrap;
}

.accent {
  background: var(--accent-soft);
  color: var(--accent);
}

.mute {
  background: var(--surface-3);
  color: var(--ink-2);
}

.good {
  background: var(--good-soft);
  color: var(--good);
}

.warn {
  background: var(--warn-soft);
  color: var(--warn);
}

.crit {
  background: var(--crit-soft);
  color: var(--crit);
}
</style>
