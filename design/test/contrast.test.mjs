import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { checkContrast, contrastRatio, floor2, relativeLuminance } from '../lib/contrast.mjs';
import { loadModel, TokenError } from '../lib/model.mjs';

const here = (p) => fileURLToPath(new URL(p, import.meta.url));

test('luminance of white and black', () => {
  assert.equal(relativeLuminance('#FFFFFF'), 1);
  assert.equal(relativeLuminance('#000000'), 0);
});

test('black on white is 21:1, in either order', () => {
  assert.equal(contrastRatio('#000000', '#FFFFFF'), 21);
  assert.equal(contrastRatio('#FFFFFF', '#000000'), 21);
});

test('#777777 on white fails 4.5 although it rounds to 4.5', () => {
  const ratio = contrastRatio('#777777', '#FFFFFF');
  assert.ok(ratio > 4.47 && ratio < 4.49, String(ratio));
  const model = { themes: { light: { fg: '#777777', bg: '#FFFFFF' }, dark: { fg: '#000000', bg: '#FFFFFF' } }, brand: {} };
  const rules = { minimum: { text: 4.5, 'non-text': 3 }, pairs: [{ fg: 'fg', bg: 'bg', kind: 'text' }], brand: [] };
  const [light, dark] = checkContrast(model, rules);
  assert.equal(light.ok, false);
  assert.equal(dark.ok, true);
  assert.equal(floor2(light.ratio), '4.47');
});

test('a rule naming a missing role or an unknown kind stops the check', () => {
  const model = { themes: { light: { a: '#000000' }, dark: { a: '#000000' } }, brand: { mark: '#000000' } };
  const typo = { minimum: { text: 4.5, 'non-text': 3 }, pairs: [{ fg: 'a', bg: 'backgroud', kind: 'text' }], brand: [] };
  assert.throws(() => checkContrast(model, typo), (e) => e instanceof TokenError && /backgroud/.test(e.message));
  const kind = { minimum: { text: 4.5, 'non-text': 3 }, pairs: [{ fg: 'a', bg: 'a', kind: 'large-text' }], brand: [] };
  assert.throws(() => checkContrast(model, kind), /large-text/);
  const brand = { minimum: { text: 4.5, 'non-text': 3 }, pairs: [], brand: [{ fg: 'mark', bg: 'nope', kind: 'non-text' }] };
  assert.throws(() => checkContrast(model, brand), /nope/);
});

test('every pair of the real tokens passes', async () => {
  const model = await loadModel(here('../tokens.json'));
  const rules = JSON.parse(await readFile(here('../contrast.json'), 'utf8'));
  const results = checkContrast(model, rules);
  assert.equal(results.length, rules.pairs.length * 2 + rules.brand.length);
  assert.deepEqual(results.filter((r) => !r.ok), []);
});

test('a role named like a built-in property is still reported by name', () => {
  const model = { themes: { light: { a: '#000000' }, dark: { a: '#000000' } }, brand: {} };
  const rules = { minimum: { text: 4.5, 'non-text': 3 }, pairs: [{ fg: 'constructor', bg: 'a', kind: 'text' }], brand: [] };
  assert.throws(() => checkContrast(model, rules), (e) => e instanceof TokenError && /constructor/.test(e.message));
});

test('a pair can take its foreground from the other theme', () => {
  const model = { themes: { light: { accent: '#000000', 'on-surface': '#000000' }, dark: { accent: '#FFFFFF', 'on-surface': '#FFFFFF' } }, brand: {} };
  const rules = { minimum: { text: 4.5, 'non-text': 3 }, pairs: [{ fg: 'accent', fgTheme: 'other', bg: 'on-surface', kind: 'text' }], brand: [] };
  const [light, dark] = checkContrast(model, rules);
  assert.equal(light.ratio, 21);
  assert.equal(dark.ratio, 21);
  assert.equal(light.fg, 'accent (dark)');
  const bad = { ...rules, pairs: [{ ...rules.pairs[0], fgTheme: 'same' }] };
  assert.throws(() => checkContrast(model, bad), /fgTheme/);
});

test('the pairs Material draws from inverse and container roles are checked', async () => {
  const rules = JSON.parse(await readFile(here('../contrast.json'), 'utf8'));
  const has = (fg, bg, extra = {}) => rules.pairs.some((p) => p.fg === fg && p.bg === bg && Object.entries(extra).every(([k, v]) => p[k] === v));
  assert.ok(has('background', 'on-surface'), 'snackbar text: inverseOnSurface on inverseSurface');
  assert.ok(has('accent', 'on-surface', { fgTheme: 'other' }), 'snackbar action: inversePrimary on inverseSurface');
  assert.ok(has('outline', 'surface-highest'), 'unchecked switch border on its track');
});
