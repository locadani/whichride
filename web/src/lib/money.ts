import type { Lang } from '../i18n/format';

const formats = new Map<Lang, Intl.NumberFormat>();

export function formatMoney(cents: number, lang: Lang): string {
  let format = formats.get(lang);
  if (!format) {
    format = new Intl.NumberFormat(lang, { style: 'currency', currency: 'EUR' });
    formats.set(lang, format);
  }
  return format.format(cents / 100);
}
