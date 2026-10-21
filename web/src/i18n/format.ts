// Message formatting shared by the build and the island; no Node APIs here.
export const LANGS = ['en', 'it', 'de', 'tr'] as const;
export type Lang = (typeof LANGS)[number];

/** Each language by its own name, the same on every page. */
export const LANGUAGE_NAMES: Record<Lang, string> = { en: 'English', it: 'Italiano', de: 'Deutsch', tr: 'Türkçe' };

export type Plural = { other: string } & Partial<Record<Intl.LDMLPluralRule, string>>;
export type Message = string | Plural;
export type Messages = Record<string, Message>;

/** Fills `{1}`, `{2}`… with `args`; a plural picks its form by the first argument. */
export function format(message: Message | undefined, lang: Lang, ...args: (string | number)[]): string {
  if (message === undefined) throw new Error('missing message');
  const text =
    typeof message === 'string' ? message : (message[new Intl.PluralRules(lang).select(Number(args[0]))] ?? message.other);
  return text.replace(/\{(\d+)\}/g, (whole, n: string) => {
    const value = args[Number(n) - 1];
    return value === undefined ? whole : String(value);
  });
}

/** "MILES, Bolt Drive or SIXT share", as the app's `orList`. */
export function orList(names: string[], messages: Messages, lang: Lang): string {
  if (names.length <= 1) return names[0] ?? '';
  return format(messages.list_or, lang, names.slice(0, -1).join(', '), names[names.length - 1]);
}

/** A title split around its accented part (the app's `accented`). */
export function accented(title: string, accent: string): [string, string, string] {
  const at = accent ? title.lastIndexOf(accent) : -1;
  if (at < 0) return [title, '', ''];
  return [title.slice(0, at), accent, title.slice(at + accent.length)];
}
