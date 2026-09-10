export const LAYOUT_NAME_MAX_CHARS = 40;

export interface NamedLayout {
  id: string;
  name: string;
}

export interface LayoutNameCheck {
  /** The name as it would be saved. */
  name: string;
  /** False when a rule is broken; `message` says which. */
  ok: boolean;
  message: string | null;
  /** The layout that already has this name, compared without case. */
  conflict: NamedLayout | null;
}

/**
 * The layout name rules, the same ones Rust enforces: trimmed, not empty, at most 40
 * characters, unique regardless of case. A conflict is not a violation: saving offers
 * to replace the other layout. `ignoreId` excludes the layout being re-captured.
 */
export function checkLayoutName(
  raw: string,
  existing: readonly NamedLayout[],
  ignoreId: string | null = null,
): LayoutNameCheck {
  const name = raw.trim();
  if (name.length === 0) {
    return { name, ok: false, message: 'Give the layout a name.', conflict: null };
  }
  if ([...name].length > LAYOUT_NAME_MAX_CHARS) {
    return {
      name,
      ok: false,
      message: `Keep the name to ${LAYOUT_NAME_MAX_CHARS} characters or fewer.`,
      conflict: null,
    };
  }
  const lower = name.toLowerCase();
  const other = existing.find((l) => l.id !== ignoreId && l.name.trim().toLowerCase() === lower);
  const conflict = other ? { id: other.id, name: other.name } : null;
  return { name, ok: true, message: null, conflict };
}
