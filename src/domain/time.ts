function timeOfDay(date: Date): string {
  return `${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`;
}

function sameDay(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

function dayAndMonth(date: Date): string {
  return `${date.getDate()} ${MONTHS[date.getMonth()]}`;
}

/** "14:32 today" or "14:32, 8 Sep": the sidebar's last-probe line. */
export function formatProbeTime(iso: string, now: Date = new Date()): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return 'unknown';
  }
  return sameDay(date, now) ? `${timeOfDay(date)} today` : `${timeOfDay(date)}, ${dayAndMonth(date)}`;
}

/** "read from Windows just now" for the first minute, then "read from Windows at 14:32". */
export function describeProbeAge(iso: string, now: Date = new Date()): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return 'read from Windows';
  }
  const ageMs = now.getTime() - date.getTime();
  if (ageMs < 60_000) {
    return 'read from Windows just now';
  }
  return `read from Windows at ${timeOfDay(date)}`;
}

/** "Captured 9 Sep at 14:32". */
export function describeCaptureTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return 'Captured';
  }
  return `Captured ${dayAndMonth(date)} at ${timeOfDay(date)}`;
}
