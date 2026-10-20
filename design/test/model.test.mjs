import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadModel, resolveTokens, toModel, TokenError, MODES } from '../lib/model.mjs';

const TOKENS = fileURLToPath(new URL('../tokens.json', import.meta.url));

const color = (path, value) => ({ path, type: 'color', value });
const minimal = () => [
  color(['theme', 'light', 'background'], '#FFFFFF'),
  color(['theme', 'dark', 'background'], '#000000'),
  { path: ['font', 'text'], type: 'fontFamily', value: ['Sora', 'sans-serif'] },
  { path: ['space', 'md'], type: 'dimension', value: '16px' },
  {
    path: ['type', 'body-large'], type: 'typography',
    value: { fontFamily: ['Sora', 'sans-serif'], fontSize: '16px', fontWeight: 400, lineHeight: 1.5, letterSpacing: '-0.02em' },
  },
];

test('a minimal token set becomes a model', () => {
  const model = toModel(minimal());
  assert.deepEqual(model.themes, { light: { background: '#FFFFFF' }, dark: { background: '#000000' } });
  assert.deepEqual(model.space, { md: 16 });
  assert.deepEqual(model.fonts, { text: ['Sora', 'sans-serif'] });
  assert.deepEqual(model.type['body-large'], { font: 'text', size: 16, weight: 400, lineHeight: 1.5, letterSpacing: -0.02 });
});

test('colours must be upper-case #RRGGBB', () => {
  for (const bad of ['#ffffff', '#FFF', '#FFFFFF80', 'white', 'rgb(0,0,0)']) {
    const tokens = minimal();
    tokens[0] = color(['theme', 'light', 'background'], bad);
    assert.throws(() => toModel(tokens), (e) => e instanceof TokenError && e.message.includes('theme.light.background'), bad);
  }
});

test('a role present in only one theme names the role', () => {
  const tokens = [...minimal(), color(['theme', 'dark', 'accent'], '#3DDC97')];
  assert.throws(() => toModel(tokens), /only dark \[accent\]/);
});

test('unknown groups and non-kebab-case names are rejected', () => {
  assert.throws(() => toModel([...minimal(), color(['colour', 'x'], '#000000')]), /unknown group "colour"/);
  assert.throws(() => toModel([...minimal(), color(['theme', 'light', 'onSurface'], '#000000')]), /onSurface/);
  assert.throws(() => toModel([...minimal(), color(['theme', 'sepia', 'background'], '#000000')]), /theme\.<light\|dark>\.<role>/);
});

test('dimensions need px and typography must use a declared font', () => {
  const noUnit = minimal();
  noUnit[3] = { path: ['space', 'md'], type: 'dimension', value: '16' };
  assert.throws(() => toModel(noUnit), /space\.md/);
  const strayFont = minimal();
  strayFont[4] = { ...strayFont[4], value: { ...strayFont[4].value, fontFamily: ['Arial', 'sans-serif'] } };
  assert.throws(() => toModel(strayFont), /type\.body-large.*not a declared font/);
  const badWeight = minimal();
  badWeight[4] = { ...badWeight[4], value: { ...badWeight[4].value, fontWeight: 450 } };
  assert.throws(() => toModel(badWeight), /fontWeight/);
});

test('a broken reference fails while resolving', async () => {
  const dir = await mkdtemp(join(tmpdir(), 'tokens-'));
  const file = join(dir, 'tokens.json');
  await writeFile(file, JSON.stringify({ brand: { $type: 'color', mark: { $value: '{palette.missing}' } } }));
  await assert.rejects(resolveTokens(file));
});

test('the real tokens load, with the same roles in both themes', async () => {
  const model = await loadModel(TOKENS);
  const roles = MODES.map((m) => Object.keys(model.themes[m]).sort());
  assert.deepEqual(roles[0], roles[1]);
  assert.equal(model.themes.dark.accent, '#3DDC97');
  assert.equal(model.themes.light.accent, '#0B7A50');
  assert.equal(model.brand.mark, '#3DDC97');
  assert.equal(model.type['number-large'].font, 'number');
});
