// The app's km/min rules (format/Units.kt), with integer maths.
export const MIN_METRES = 100;
export const MAX_METRES = 2_000_000;
export const MAX_MINUTES = 1_440;

const KILOMETRES = /^(\d{0,4})(?:[.,](\d{1,3}))?$/;

/** "7.4", "7,4", "12" → metres, or null when not a distance between 0.1 and 2000 km. */
export function parseKilometres(text: string): number | null {
  const match = KILOMETRES.exec(text.trim());
  if (!match) return null;
  const [, whole, fraction = ''] = match;
  if (whole === '' && fraction === '') return null;
  const metres = Number(whole || '0') * 1_000 + Number(fraction.padEnd(3, '0') || '0');
  return metres >= MIN_METRES && metres <= MAX_METRES ? metres : null;
}

/** Whole minutes → seconds, or null when not between 1 and 1440. */
export function parseMinutes(text: string): number | null {
  const trimmed = text.trim();
  if (!/^\d{1,4}$/.test(trimmed)) return null;
  const minutes = Number(trimmed);
  return minutes >= 1 && minutes <= MAX_MINUTES ? minutes * 60 : null;
}
