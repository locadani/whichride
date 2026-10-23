import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { logoOutputs, parseLogo, resolvedSvg, roundedRectPath } from '../lib/logo.mjs';
import { loadModel, TokenError } from '../lib/model.mjs';

const here = (p) => fileURLToPath(new URL(p, import.meta.url));
const svg = (inner, viewBox = '0 0 32 32') => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${viewBox}">${inner}</svg>`;
const BRAND = { mark: '#3DDC97', 'on-mark': '#04261A', 'on-mark-behind': '#1E7852' };
const RECT = '<rect width="32" height="32" rx="10" fill="{brand.mark}"/>';
const PATH = '<path d="M16 22.75V16.75" fill="none" stroke="{brand.on-mark}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>';
const BEHIND = '<path d="M20 10H26" fill="none" stroke="{brand.on-mark-behind}" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" data-monochrome="omit"/>';
const FILL = '<path d="M4 20H28V24H4Z" fill="{brand.mark}"/>';

test('a valid logo parses, with colours resolved from the brand tokens', () => {
  assert.deepEqual(parseLogo(svg(RECT + PATH), BRAND), {
    size: 32,
    tile: { rx: 10, fill: '#3DDC97' },
    layers: [{ d: 'M16 22.75V16.75', stroke: '#04261A', strokeWidth: 1.8, monochrome: true }],
  });
});

test('layers keep their order: lines, fills, and lines left out of the themed icon', () => {
  assert.deepEqual(parseLogo(svg(RECT + BEHIND + FILL + PATH), BRAND).layers, [
    { d: 'M20 10H26', stroke: '#1E7852', strokeWidth: 1.4, monochrome: false },
    { d: 'M4 20H28V24H4Z', fill: '#3DDC97' },
    { d: 'M16 22.75V16.75', stroke: '#04261A', strokeWidth: 1.8, monochrome: true },
  ]);
});

test('anything outside the constrained format is rejected', () => {
  const cases = {
    'extra element': svg(RECT + PATH + '<circle r="2"/>'),
    'no rect': svg(PATH),
    'extra attribute': svg(RECT.replace('/>', ' transform="rotate(3)"/>') + PATH),
    'non-square': svg(RECT + PATH, '0 0 32 24'),
    'rect smaller than the box': svg(RECT.replace('width="32"', 'width="30"') + PATH),
    'literal tile colour': svg(RECT.replace('{brand.mark}', '#3DDC97') + PATH),
    'literal mark colour': svg(RECT + PATH.replace('{brand.on-mark}', '#04261A')),
    'unknown brand token': svg(RECT.replace('{brand.mark}', '{brand.markk}') + PATH),
    'theme role instead of brand': svg(RECT.replace('{brand.mark}', '{theme.dark.accent}') + PATH),
    'no path': svg(RECT),
    'filled and stroked path': svg(RECT + PATH.replace('fill="none"', 'fill="{brand.mark}"')),
    'literal fill colour': svg(RECT + FILL.replace('{brand.mark}', '#3DDC97') + PATH),
    'fill without a colour': svg(RECT + FILL.replace(' fill="{brand.mark}"', '') + PATH),
    'square caps': svg(RECT + PATH.replace('stroke-linecap="round"', 'stroke-linecap="square"')),
    'unknown monochrome value': svg(RECT + PATH.replace('/>', ' data-monochrome="keep"/>')),
    'monochrome on a fill': svg(RECT + FILL.replace('/>', ' data-monochrome="omit"/>') + PATH),
    'nothing left for the themed icon': svg(RECT + BEHIND + FILL),
  };
  for (const [name, text] of Object.entries(cases)) {
    assert.throws(() => parseLogo(text, BRAND), TokenError, name);
  }
});

test('the resolved SVG has hex colours and no references', () => {
  const out = resolvedSvg(svg(RECT + PATH), BRAND);
  assert.match(out, /fill="#3DDC97"/);
  assert.match(out, /stroke="#04261A"/);
  assert.doesNotMatch(out, /\{brand\./);
});

test('rounded rectangle as a vector path', () => {
  assert.equal(
    roundedRectPath(32, 10),
    'M10,0H22A10,10 0 0,1 32,10V22A10,10 0 0,1 22,32H10A10,10 0 0,1 0,22V10A10,10 0 0,1 10,0Z',
  );
});

test('adaptive icon: the tile fills the 72dp visible area of the 108dp canvas', () => {
  const out = logoOutputs(parseLogo(svg(RECT + PATH), BRAND));
  const fg = out['res/drawable/ic_launcher_foreground.xml'];
  assert.match(fg, /android:viewportWidth="108"/);
  assert.match(fg, /<group android:scaleX="2\.25" android:scaleY="2\.25" android:translateX="18" android:translateY="18">/);
  assert.match(fg, /android:strokeColor="#04261A"/);
  assert.match(out['res/values/ic_launcher_background.xml'], /<color name="ic_launcher_background">#3DDC97<\/color>/);
  assert.match(out['res/mipmap-anydpi/ic_launcher.xml'], /<monochrome android:drawable="@drawable\/ic_launcher_monochrome"\/>/);
  assert.match(out['res/drawable/whichride_logo.xml'], /android:fillColor="#3DDC97"/);
  for (const text of Object.values(out)) assert.match(text, /Generated from design\/logo\.svg/);
});

test('the themed icon draws only the lines not marked omit; the others draw every layer', () => {
  const out = logoOutputs(parseLogo(svg(RECT + BEHIND + FILL + PATH), BRAND));
  const mono = out['res/drawable/ic_launcher_monochrome.xml'];
  assert.match(mono, /android:pathData="M16 22\.75V16\.75"/);
  assert.doesNotMatch(mono, /M20 10H26|android:fillColor/);
  for (const file of ['res/drawable/ic_launcher_foreground.xml', 'res/drawable/whichride_logo.xml']) {
    const text = out[file];
    const order = ['M20 10H26', 'M4 20H28V24H4Z', 'M16 22.75V16.75'].map((d) => text.indexOf(`android:pathData="${d}"`));
    assert.ok(order.every((i, k) => i > 0 && (k === 0 || i > order[k - 1])), `${file} keeps the layer order`);
    assert.match(text, /android:pathData="M4 20H28V24H4Z" android:fillColor="#3DDC97"/);
    assert.match(text, /android:strokeColor="#1E7852"/);
  }
});

test('the real logo takes its colours from tokens.json', async () => {
  const model = await loadModel(here('../tokens.json'));
  const logo = parseLogo(await readFile(here('../logo.svg'), 'utf8'), model.brand);
  assert.equal(logo.tile.fill, model.brand.mark);
  const colours = new Set(logo.layers.map((l) => l.fill ?? l.stroke));
  assert.deepEqual([...colours].sort(), [model.brand.mark, model.brand['on-mark'], model.brand['on-mark-behind']].sort());
});
