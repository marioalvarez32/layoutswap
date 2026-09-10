import type { AppErrorPayload } from '@/domain/generated/types';

/**
 * The message to show for a failure that came back across the command seam.
 * Rust sends an AppErrorPayload; anything else is turned into text as-is.
 */
export function errorMessage(error: unknown): string {
  if (isAppErrorPayload(error)) {
    return error.message;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return String(error);
}

function isAppErrorPayload(value: unknown): value is AppErrorPayload {
  return typeof value === 'object'
    && value !== null
    && 'message' in value
    && typeof (value as { message: unknown }).message === 'string';
}
