import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { logoOutputs, parseLogo, resolvedSvg, roundedRectPath } from '../lib/logo.mjs';
import { loadModel, TokenError } from '../lib/model.mjs';

const here = (p) => fileURLToPath(new URL(p, import.meta.url));
const svg = (inner, viewBox = '0 0 32 32') => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${viewBox}">${inner}</svg>`;
const BRAND = { mark: '#3DDC97', 'on-mark': '#04261A' };
const RECT = '<rect width="32" height="32" rx="10" fill="{brand.mark}"/>';
const PATH = '<path d="M16 22.75V16.75" fill="none" stroke="{brand.on-mark}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>';

test('a valid logo parses, with colours resolved from the brand tokens', () => {
  assert.deepEqual(parseLogo(svg(RECT + PATH), BRAND), {
    size: 32,
    tile: { rx: 10, fill: '#3DDC97' },
    mark: { d: 'M16 22.75V16.75', stroke: '#04261A', strokeWidth: 1.8 },
  });
});

test('anything outside the constrained format is rejected', () => {
  const cases = {
    'extra element': svg(RECT + PATH + '<circle r="2"/>'),
    'two paths': svg(RECT + PATH + PATH),
    'no rect': svg(PATH),
    'extra attribute': svg(RECT.replace('/>', ' transform="rotate(3)"/>') + PATH),
    'non-square': svg(RECT + PATH, '0 0 32 24'),
    'rect smaller than the box': svg(RECT.replace('width="32"', 'width="30"') + PATH),
    'literal tile colour': svg(RECT.replace('{brand.mark}', '#3DDC97') + PATH),
    'literal mark colour': svg(RECT + PATH.replace('{brand.on-mark}', '#04261A')),
    'unknown brand token': svg(RECT.replace('{brand.mark}', '{brand.markk}') + PATH),
    'theme role instead of brand': svg(RECT.replace('{brand.mark}', '{theme.dark.accent}') + PATH),
    'filled path': svg(RECT + PATH.replace('fill="none"', 'fill="{brand.mark}"')),
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
  assert.match(out['res/mipmap-anydpi/ic_launcher.xml'], /<monochrome android:drawable="@drawable\/ic_launcher_foreground"\/>/);
  assert.match(out['res/drawable/whichride_logo.xml'], /android:fillColor="#3DDC97"/);
  for (const text of Object.values(out)) assert.match(text, /Generated from design\/logo\.svg/);
});

test('the real logo takes its colours from tokens.json', async () => {
  const model = await loadModel(here('../tokens.json'));
  const logo = parseLogo(await readFile(here('../logo.svg'), 'utf8'), model.brand);
  assert.equal(logo.tile.fill, model.brand.mark);
  assert.equal(logo.mark.stroke, model.brand['on-mark']);
});
