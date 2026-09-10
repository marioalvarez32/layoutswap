import type { Layout } from '@/domain/generated/types';

/**
 * One row of the layouts list in the sidebar: what a user needs to pick a layout at a
 * glance.
 */
export interface LayoutListItem {
  id: string;
  name: string;
  /** Monitors on in this layout. */
  onCount: number;
  /** Connected monitors this layout turns off. */
  offCount: number;
}

export function toListItem(layout: Layout): LayoutListItem {
  const onCount = layout.summary.monitors.filter((m) => m.on).length;
  return {
    id: layout.id,
    name: layout.name,
    onCount,
    offCount: layout.summary.monitors.length - onCount,
  };
}

/** "3 on · 1 off", the data-font count under a layout's name. */
export function formatOnOffCount(item: Pick<LayoutListItem, 'onCount' | 'offCount'>): string {
  return `${item.onCount} on · ${item.offCount} off`;
}

/** Replaces the layout with the same id, or appends it. Returns a new array. */
export function upsertLayout(layouts: readonly Layout[], layout: Layout): Layout[] {
  const index = layouts.findIndex((l) => l.id === layout.id);
  if (index === -1) {
    return [...layouts, layout];
  }
  return layouts.map((l, i) => (i === index ? layout : l));
}
