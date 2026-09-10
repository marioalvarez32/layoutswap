/**
 * One row of the layouts list in the sidebar: what a user needs to pick a layout at a
 * glance. Derived from a layout's summary at capture; the capture ticket adds the
 * function that derives it.
 */
export interface LayoutListItem {
  id: string;
  name: string;
  /** Monitors on in this layout. */
  onCount: number;
  /** Connected monitors this layout turns off. */
  offCount: number;
}

/** "3 on · 1 off", the data-font count under a layout's name. */
export function formatOnOffCount(item: Pick<LayoutListItem, 'onCount' | 'offCount'>): string {
  return `${item.onCount} on · ${item.offCount} off`;
}
