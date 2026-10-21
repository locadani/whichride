import { readFileSync } from 'node:fs';
import { LANGS, accented, format, orList } from '../src/i18n/format';
import { RES_DIR, SHARED_KEYS, loadShared, parseStrings, toPositional, unescapeAndroid } from '../src/i18n/android';
import { WEB } from '../src/i18n/web';

const placeholders = (text: string) => [...text.matchAll(/\{\d+\}/g)].map((m) => m[0]).sort();
const forms = (message: unknown) => (typeof message === 'string' ? { other: message } : (message as Record<string, string>));

test('Android escapes are undone', () => {
  expect(unescapeAndroid(String.raw`We\'ll`)).toBe("We'll");
  expect(unescapeAndroid(String.raw`say \"hi\" \@home \?x`)).toBe('say "hi" @home ?x');
  expect(unescapeAndroid(String.raw`a\nb\tc\\d`)).toBe('a\nb\tc\\d');
  expect(unescapeAndroid(String.raw`è caro`)).toBe('è caro');
  expect(unescapeAndroid('"Don\'t quote me"')).toBe("Don't quote me");
});

test('placeholders become positional', () => {
  expect(toPositional('%1$d cars, from %2$s (%3$s)')).toBe('{1} cars, from {2} ({3})');
  expect(toPositional('Add %d cars')).toBe('Add {1} cars');
  expect(toPositional('%s or %s, 100%%')).toBe('{1} or {2}, 100%');
});

test('strings and plurals are parsed', () => {
  const xml = `<resources><string name="a">It\\'s %1$s</string>
    <plurals name="p"><item quantity="one">%d car</item><item quantity="other">%d cars</item></plurals></resources>`;
  const parsed = parseStrings(xml);
  expect(parsed.get('a')).toBe("It's {1}");
  expect(parsed.get('p')).toEqual({ one: '{1} car', other: '{1} cars' });
});

test('format fills placeholders and picks plurals by the first number', () => {
  expect(format('{2} then {1}', 'en', 'a', 'b')).toBe('b then a');
  const plural = { one: '{1} car', other: '{1} cars' };
  expect(format(plural, 'en', 1)).toBe('1 car');
  expect(format(plural, 'en', 3)).toBe('3 cars');
  expect(format({ other: '{1} araç' }, 'tr', 1)).toBe('1 araç');
  expect(() => format(undefined, 'en')).toThrow();
});

test('provider lists read like the app', () => {
  const en = loadShared('en');
  expect(orList([], en, 'en')).toBe('');
  expect(orList(['MILES'], en, 'en')).toBe('MILES');
  expect(orList(['MILES', 'Bolt Drive', 'SIXT share'], en, 'en')).toBe('MILES, Bolt Drive or SIXT share');
});

test('accented splits at the last occurrence', () => {
  expect(accented('Which cars are near you?', 'near you?')).toEqual(['Which cars are ', 'near you?', '']);
  expect(accented('No accent', 'zzz')).toEqual(['No accent', '', '']);
});

test.each(LANGS)('every shared key exists in %s with English placeholders', (lang) => {
  const en = loadShared('en');
  const messages = loadShared(lang);
  for (const key of SHARED_KEYS) {
    expect(messages[key], `${lang}:${key}`).toBeDefined();
    const english = placeholders(forms(en[key]).other);
    const translated = forms(messages[key]);
    expect(placeholders(translated.other), `${lang}:${key} other`).toEqual(english);
    for (const text of Object.values(translated)) {
      for (const p of placeholders(text)) expect(english, `${lang}:${key}`).toContain(p);
      expect(text, `${lang}:${key}`).not.toMatch(/%\d*\$?[sd]|\\['"@?n]/);
    }
  }
});

test.each(LANGS)('web-only strings in %s have every key, none empty', (lang) => {
  expect(Object.keys(WEB[lang]).sort()).toEqual(Object.keys(WEB.en).sort());
  for (const [key, text] of Object.entries(WEB[lang])) expect(text.trim(), `${lang}:${key}`).not.toBe('');
  expect(WEB[lang].heading).toContain(WEB[lang].heading_accent);
});

test('the km and minute limits match the app', () => {
  const units = readFileSync('../android/app/src/main/kotlin/io/github/locadani/whichride/format/Units.kt', 'utf8');
  expect(units).toContain('MAX_METRES = 2_000_000u');
  expect(units).toContain('MIN_METRES = 100u');
  expect(units).toContain('MAX_MINUTES = 1_440u');
  const en = loadShared('en');
  expect(en.manual_km_invalid).toBe('Between 0.1 and 2000 km');
  expect(en.manual_minutes_invalid).toBe('Between 1 and 1440 minutes');
  expect(RES_DIR).toMatch(/android\/app\/src\/main\/res$/);
});
