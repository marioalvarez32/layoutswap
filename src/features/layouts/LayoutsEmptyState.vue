<script setup lang="ts">
import Button from '@/ui/Button.vue';

defineEmits<{
  save: [];
}>();
</script>

<template>
  <section class="empty-state">
    <!-- Three monitors as drawn on the canvas: one on and primary, one on, one off. -->
    <svg
      class="glyph"
      viewBox="0 0 206 60"
      width="206"
      height="60"
      aria-hidden="true"
    >
      <rect
        class="on primary"
        x="0.5"
        y="0.5"
        width="96"
        height="58"
        rx="3"
      />
      <rect
        class="on"
        x="103.5"
        y="0.5"
        width="58"
        height="58"
        rx="3"
      />
      <rect
        class="off"
        x="166.5"
        y="0.5"
        width="39"
        height="58"
        rx="3"
      />
    </svg>
    <h2>No layouts yet</h2>
    <p class="lead">
      A layout is the arrangement of monitors Windows is showing right now, saved with
      the steps a switch needs: input sources, audio endpoints and Remote Desktop monitor IDs.
    </p>
    <Button variant="primary" @click="$emit('save')">
      Save current layout
    </Button>
    <p class="hint">
      Arrange your monitors in Windows Settings &gt; Display first.
    </p>
  </section>
</template>

<style scoped>
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: var(--space-5);
  max-width: var(--prose-w);
}

.glyph {
  display: block;
}

.glyph rect {
  stroke-width: var(--hairline);
}

.glyph .on {
  fill: var(--surface-2);
  stroke: var(--line-strong);
}

.glyph .on.primary {
  fill: var(--accent-soft);
  stroke: var(--accent-line);
}

.glyph .off {
  fill: none;
  stroke: var(--line-strong);
  stroke-dasharray: 3 3;
}

h2 {
  margin: 0;
  font-family: var(--display);
  font-size: var(--text-2xl);
  font-weight: 600;
  letter-spacing: var(--tracking-tight);
}

.lead {
  margin: 0;
  font-size: var(--text-base);
  line-height: var(--leading-prose);
  color: var(--ink-2);
  text-wrap: pretty;
}

.hint {
  margin: 0;
  font-size: var(--text-md);
  color: var(--ink-3);
}
</style>
