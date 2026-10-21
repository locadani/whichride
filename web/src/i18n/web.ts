// Wording only the website has (heading, meta, footer…); everything shared is in strings.xml.
import en from './en.json';
import it from './it.json';
import de from './de.json';
import tr from './tr.json';
import type { Lang } from './format';

export type WebStrings = typeof en;
export const WEB: Record<Lang, WebStrings> = { en, it, de, tr };
