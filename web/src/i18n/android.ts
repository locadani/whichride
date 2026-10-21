// Build time only: reads the wording the site shares with the app from the app's strings.xml.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { XMLParser } from 'fast-xml-parser';
import type { Lang, Message, Messages, Plural } from './format';

/** Relative to web/ (the working directory of astro, vitest and npm scripts). */
export const RES_DIR = resolve(process.cwd(), '../android/app/src/main/res');

export const SHARED_KEYS = [
  'intro_title', 'intro_title_accent', 'intro_body', 'list_or', 'add_cars', 'add_more_cars',
  'cheaper_not_added', 'nothing_cheaper', 'which_cars_body', 'add_n_cars', 'done',
  'cheapest_of_options', 'package_meta', 'manual_km', 'manual_minutes', 'manual_km_invalid',
  'manual_minutes_invalid', 'loading_prices', 'built_in_prices',
] as const;

/** Android resource escapes: surrounding quotes, \' \" \@ \? \n \t \\ and \uXXXX. */
export function unescapeAndroid(raw: string): string {
  let text = raw.trim();
  if (text.length >= 2 && text.startsWith('"') && text.endsWith('"') && !text.endsWith('\\"')) text = text.slice(1, -1);
  return text.replace(/\\(u[0-9a-fA-F]{4}|.)/g, (_, c: string) => {
    if (c.length === 5) return String.fromCharCode(parseInt(c.slice(1), 16));
    if (c === 'n') return '\n';
    if (c === 't') return '\t';
    return c;
  });
}

/** `%1$s`, `%2$d` → `{1}`, `{2}`; bare `%s`/`%d` numbered in order; `%%` → `%`. */
export function toPositional(text: string): string {
  let next = 0;
  return text.replace(/%%|%(?:(\d+)\$)?[sd]/g, (whole, n?: string) => {
    if (whole === '%%') return '%';
    next += 1;
    return `{${n ?? next}}`;
  });
}

const parser = new XMLParser({
  ignoreAttributes: false,
  attributeNamePrefix: '',
  parseTagValue: false,
  trimValues: false,
  isArray: (name) => ['string', 'plurals', 'item'].includes(name),
});

type Node = { name: string; '#text'?: string; quantity?: string; item?: Node[] };

export function parseStrings(xml: string): Map<string, Message> {
  const resources = (parser.parse(xml) as { resources?: { string?: Node[]; plurals?: Node[] } }).resources ?? {};
  const out = new Map<string, Message>();
  const text = (node: Node) => toPositional(unescapeAndroid(String(node['#text'] ?? '')));
  for (const node of resources.string ?? []) out.set(node.name, text(node));
  for (const node of resources.plurals ?? []) {
    const forms: Record<string, string> = {};
    for (const item of node.item ?? []) if (item.quantity) forms[item.quantity] = text(item);
    if (forms.other !== undefined) out.set(node.name, forms as Plural);
  }
  return out;
}

const cache = new Map<Lang, Messages>();

/** The allowlisted keys for one language. A missing key is a build error. */
export function loadShared(lang: Lang, resDir: string = RES_DIR): Messages {
  const hit = cache.get(lang);
  if (hit && resDir === RES_DIR) return hit;
  const folder = lang === 'en' ? 'values' : `values-${lang}`;
  const all = parseStrings(readFileSync(`${resDir}/${folder}/strings.xml`, 'utf8'));
  const messages: Messages = {};
  for (const key of SHARED_KEYS) {
    const message = all.get(key);
    if (message === undefined) throw new Error(`${folder}/strings.xml has no "${key}"`);
    messages[key] = message;
  }
  if (resDir === RES_DIR) cache.set(lang, messages);
  return messages;
}
