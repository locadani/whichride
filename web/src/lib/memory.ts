// The trip and ticked cars, in this browser only, for 15 minutes from the last change (cars move).
export const KEY = 'whichride.trip';
export const TTL_MS = 15 * 60 * 1000;
const MAX_FIELD = 16;
const MAX_OFFERS = 200;

export interface Remembered { km: string; minutes: string; offers: string[] }

export function browserStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export function remember(storage: Storage | null, value: Remembered, now: number): void {
  try {
    storage?.setItem(KEY, JSON.stringify({ ...value, savedAt: now }));
  } catch {
    // private mode, quota, blocked: the page works without memory
  }
}

function forget(storage: Storage) {
  try {
    storage.removeItem(KEY);
  } catch {
    // nothing to do
  }
}

const isShortString = (v: unknown): v is string => typeof v === 'string' && v.length <= MAX_FIELD;

/** What was remembered, or null (and the entry removed) when missing, expired or malformed. */
export function recall(storage: Storage | null, now: number): Remembered | null {
  if (!storage) return null;
  let raw: string | null;
  try {
    raw = storage.getItem(KEY);
  } catch {
    return null;
  }
  if (raw === null) return null;
  try {
    const v = JSON.parse(raw) as Record<string, unknown>;
    const fresh = typeof v.savedAt === 'number' && v.savedAt <= now && now - v.savedAt <= TTL_MS;
    const offers = v.offers;
    if (
      fresh &&
      isShortString(v.km) &&
      isShortString(v.minutes) &&
      Array.isArray(offers) &&
      offers.length <= MAX_OFFERS &&
      offers.every((o) => typeof o === 'string' && o.length <= 100)
    ) {
      // Each car once: duplicates would be counted twice and break keyed lists.
      return { km: v.km, minutes: v.minutes, offers: [...new Set(offers as string[])] };
    }
  } catch {
    // malformed: forgotten below
  }
  forget(storage);
  return null;
}
