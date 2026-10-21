import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { DEFAULT_SITE } from '../astro.config.mjs';

const DIST = new URL('../dist/', import.meta.url);
const read = (path: string) => readFileSync(new URL(path, DIST), 'utf8');
const SITE = `${process.env.WHICHRIDE_SITE_URL || DEFAULT_SITE}/`;
const LANGS = ['en', 'it', 'de', 'tr'];
/** The page's text: no scripts, styles, tags or attributes (island props hold raw messages on purpose). */
const visible = (html: string) =>
  html.replace(/<script[\s\S]*?<\/script>/g, '').replace(/<style[\s\S]*?<\/style>/g, '').replace(/<[^>]*>/g, ' ');

describe.each(LANGS)('/%s/', (lang) => {
  const html = read(`${lang}/index.html`);

  test('declares its language, canonical URL and alternates', () => {
    expect(html).toContain(`<html lang="${lang}"`);
    expect(html).toContain(`<link rel="canonical" href="${SITE}${lang}/"`);
    for (const l of LANGS) expect(html).toContain(`<link rel="alternate" hreflang="${l}" href="${SITE}${l}/"`);
    expect(html).toContain(`<link rel="alternate" hreflang="x-default" href="${SITE}en/"`);
    expect(html).toMatch(/<meta name="description" content="[^"]{50,}"/);
  });

  test('has a CSP that allows the WebAssembly and only this origin', () => {
    const csp = html.match(/<meta http-equiv="content-security-policy" content="([^"]+)"/i)?.[1] ?? '';
    expect(csp).toContain("'wasm-unsafe-eval'");
    expect(csp).toContain("default-src 'self'");
    expect(csp).not.toContain('unsafe-inline');
    // Scripts, styles and preloads come from this origin (canonical/hreflang links are absolute by design).
    expect(html).not.toMatch(/<script[^>]+src="https?:/);
    expect(html).not.toMatch(/<link[^>]+rel="(stylesheet|modulepreload|preload)"[^>]+href="https?:/);
  });

  test('renders the first visit with no raw placeholders', () => {
    const text = visible(html);
    expect(text).not.toMatch(/%\d*\$?[sd]|\{\d+\}/);
    expect(text).toMatch(/\S/);
    expect(html).toContain('<noscript>');
    expect(html).toMatch(/<h1>/);
    expect(html).toContain('astro-island');
  });
});

test('the 404 page exists and is not indexed', () => {
  const html = read('404.html');
  expect(html).toContain('<meta name="robots" content="noindex"');
  for (const l of LANGS) expect(html).toContain(`href="/${l}/"`);
});

test('robots.txt and the sitemap list the four pages', () => {
  expect(read('robots.txt')).toContain(`Sitemap: ${SITE}sitemap.xml`);
  const sitemap = read('sitemap.xml');
  for (const l of LANGS) expect(sitemap).toContain(`<loc>${SITE}${l}/</loc>`);
});

test('the WebAssembly, scripts and pages are precompressed', () => {
  const assets = readdirSync(new URL('_astro/', DIST));
  const wasm = assets.find((f) => f.endsWith('.wasm'));
  expect(wasm).toBeDefined();
  expect(existsSync(new URL(`_astro/${wasm}.br`, DIST))).toBe(true);
  expect(existsSync(new URL(`_astro/${wasm}.gz`, DIST))).toBe(true);
  expect(existsSync(new URL('en/index.html.br', DIST))).toBe(true);
});
