import { describe, expect, it } from 'vitest';
import { windowSizeToPersist } from './window-size';

describe('windowSizeToPersist', () => {
  it('persists the first observed size when nothing was persisted yet', () => {
    expect(windowSizeToPersist(null, { width: 1280, height: 860 })).toEqual({ width: 1280, height: 860 });
  });

  it('persists a size that differs from the persisted one', () => {
    const persisted = { width: 1280, height: 860 };
    expect(windowSizeToPersist(persisted, { width: 1400, height: 860 })).toEqual({ width: 1400, height: 860 });
    expect(windowSizeToPersist(persisted, { width: 1280, height: 700 })).toEqual({ width: 1280, height: 700 });
  });

  it('writes nothing when the size is unchanged', () => {
    expect(windowSizeToPersist({ width: 1280, height: 860 }, { width: 1280, height: 860 })).toBeNull();
  });

  it('writes nothing while the window is minimised', () => {
    expect(windowSizeToPersist(null, { width: 0, height: 0 })).toBeNull();
    expect(windowSizeToPersist({ width: 1280, height: 860 }, { width: 0, height: 860 })).toBeNull();
  });

  it('rounds fractional logical pixels', () => {
    expect(windowSizeToPersist(null, { width: 1279.6, height: 860.4 })).toEqual({ width: 1280, height: 860 });
  });

  it('treats a rounded size equal to the persisted one as unchanged', () => {
    expect(windowSizeToPersist({ width: 1280, height: 860 }, { width: 1280.3, height: 859.7 })).toBeNull();
  });
});
