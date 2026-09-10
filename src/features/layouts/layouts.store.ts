import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { LayoutListItem } from '@/domain/layouts';

/**
 * The layouts a user has saved and which one the sidebar has selected. The capture
 * ticket fills `layouts` from the config; in this slice the list starts empty.
 */
export const useLayoutsStore = defineStore('layouts', () => {
  const layouts = ref<LayoutListItem[]>([]);
  const selectedId = ref<string | null>(null);

  const isEmpty = computed(() => layouts.value.length === 0);

  function select(id: string) {
    selectedId.value = id;
  }

  return { layouts, selectedId, isEmpty, select };
});
