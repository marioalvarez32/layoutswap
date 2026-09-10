import { describe, expect, it } from 'vitest';
import { layoutFixture } from '@/test/fixtures';
import { SCHEMATIC_BOX, SCHEMATIC_PADDING, schematicRects } from './schematic';

function byPath(rects: ReturnType<typeof schematicRects>, devicePath: string) {
  const rect = rects.find((r) => r.devicePath === devicePath);
  if (!rect) {
    throw new Error(`no rect for ${devicePath}`);
  }
  return rect;
}

describe('schematicRects', () => {
  const summary = layoutFixture().summary.monitors;

  it('draws one rectangle per on monitor, none for off ones', () => {
    const rects = schematicRects(summary);
    expect(rects.map((r) => r.devicePath).sort()).toEqual(['path-acer', 'path-builtin', 'path-msi-1', 'path-msi-2']);
  });

  it('does not draw an on monitor whose position is unknown', () => {
    const noPosition = summary.map((m) => (m.devicePath === 'path-acer' ? { ...m, position: null } : m));
    expect(schematicRects(noPosition).map((r) => r.devicePath)).not.toContain('path-acer');
  });

  it('keeps every rectangle inside the box, with the padding', () => {
    for (const r of schematicRects(summary)) {
      expect(r.x).toBeGreaterThanOrEqual(SCHEMATIC_PADDING);
      expect(r.y).toBeGreaterThanOrEqual(SCHEMATIC_PADDING);
      expect(r.x + r.width).toBeLessThanOrEqual(SCHEMATIC_BOX.width - SCHEMATIC_PADDING + 0.01);
      expect(r.y + r.height).toBeLessThanOrEqual(SCHEMATIC_BOX.height - SCHEMATIC_PADDING + 0.01);
    }
  });

  it('places the monitor above the primary above it, handling negative coordinates', () => {
    const rects = schematicRects(summary);
    const above = byPath(rects, 'path-msi-2');
    const beside = byPath(rects, 'path-msi-1');
    const primary = byPath(rects, 'path-acer');
    expect(above.y).toBeLessThan(primary.y);
    expect(above.y + above.height).toBeCloseTo(primary.y, 1);
    expect(above.x).toBeCloseTo(primary.x, 1);
    expect(beside.x + beside.width).toBeCloseTo(primary.x, 1);
    expect(primary.primary).toBe(true);
    expect(above.primary).toBe(false);
  });

  it('preserves aspect ratio and uses one scale for every monitor', () => {
    const rects = schematicRects(summary);
    const acer = byPath(rects, 'path-acer');
    const builtIn = byPath(rects, 'path-builtin');
    expect(acer.width / acer.height).toBeCloseTo(1920 / 1080, 2);
    expect(builtIn.width / builtIn.height).toBeCloseTo(2560 / 1600, 2);
    expect(builtIn.width / acer.width).toBeCloseTo(2560 / 1920, 2);
  });

  it('labels with the alias fallback and tells identical panels apart', () => {
    const rects = schematicRects(summary, { 'path-acer': 'Side' });
    expect(byPath(rects, 'path-acer').label).toBe('Side');
    expect(byPath(rects, 'path-msi-1').label).toBe('MSI MP165 E6 · USB-C DisplayPort 1');
    expect(byPath(rects, 'path-msi-2').label).toBe('MSI MP165 E6 · USB-C DisplayPort 2');
    expect(byPath(rects, 'path-builtin').label).toBe('Built-in display');
  });

  it('states the size, with the primary marked, when the rectangle is wide enough', () => {
    const only = summary.filter((m) => m.devicePath === 'path-acer' || m.devicePath === 'path-builtin');
    const rects = schematicRects(only);
    expect(byPath(rects, 'path-acer').detail).toBe('primary · 1920×1080');
    expect(byPath(rects, 'path-builtin').detail).toBe('2560×1600');
  });

  it('shortens the detail to what fits instead of cutting it', () => {
    const rects = schematicRects(summary);
    // Five monitors share the box, so the primary is too narrow for its size line.
    expect(byPath(rects, 'path-acer').detail).toBe('primary');
    const tiny = schematicRects(summary, {}, { width: 90, height: 60 });
    expect(tiny.every((r) => r.detail === '' || r.detail === 'primary')).toBe(true);
  });

  it('centres a single monitor and fills the box on its constraining side', () => {
    const only = summary.filter((m) => m.devicePath === 'path-acer');
    const [rect] = schematicRects(only);
    expect(rect).toBeDefined();
    // The inner box (276 by 148) is wider than 16:9, so height is the limit.
    expect(rect!.height).toBeCloseTo(SCHEMATIC_BOX.height - 2 * SCHEMATIC_PADDING, 1);
    expect(rect!.y).toBeCloseTo(SCHEMATIC_PADDING, 1);
    const centreX = rect!.x + rect!.width / 2;
    expect(centreX).toBeCloseTo(SCHEMATIC_BOX.width / 2, 1);
    expect(rect!.compact).toBe(false);
  });

  it('fills the width for a wide arrangement', () => {
    const wide = [
      { devicePath: 'a', reportedName: 'A', connector: 'HDMI', on: true, position: { x: 0, y: 0 }, size: { width: 1920, height: 1080 }, primary: true },
      { devicePath: 'b', reportedName: 'B', connector: 'HDMI', on: true, position: { x: 1920, y: 0 }, size: { width: 1920, height: 1080 }, primary: false },
      { devicePath: 'c', reportedName: 'C', connector: 'HDMI', on: true, position: { x: 3840, y: 0 }, size: { width: 1920, height: 1080 }, primary: false },
    ];
    const rects = schematicRects(wide);
    const left = byPath(rects, 'a');
    const right = byPath(rects, 'c');
    expect(left.x).toBeCloseTo(SCHEMATIC_PADDING, 1);
    expect(right.x + right.width).toBeCloseTo(SCHEMATIC_BOX.width - SCHEMATIC_PADDING, 1);
  });

  it('returns nothing to draw when every monitor is off', () => {
    const allOff = summary.map((m) => ({ ...m, on: false, position: null, size: null, primary: false }));
    expect(schematicRects(allOff)).toEqual([]);
    expect(schematicRects([])).toEqual([]);
  });

  it('marks rectangles too small for the detail line as compact', () => {
    const rects = schematicRects(summary, {}, { width: 120, height: 70 });
    expect(rects.every((r) => r.compact)).toBe(true);
    expect(schematicRects(summary).every((r) => !r.compact)).toBe(true);
  });
});
