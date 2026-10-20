import { test } from 'node:test';
import assert from 'node:assert/strict';
import { camel, css, kotlin } from '../lib/formats.mjs';

const model = {
  themes: {
    light: { background: '#F3F6F8', 'on-surface-muted': '#4D5966' },
    dark: { background: '#0B0F14', 'on-surface-muted': '#9AA7B6' },
  },
  brand: { mark: '#3DDC97', 'on-mark': '#04261A' },
  space: { md: 16, lg: 20 },
  radius: { md: 14 },
  size: { 'touch-target': 48 },
  fonts: { text: ['Sora', 'sans-serif'], number: ['JetBrains Mono', 'monospace'] },
  type: {
    'body-large': { font: 'text', size: 16, weight: 400, lineHeight: 1.5, letterSpacing: 0 },
    'number-large': { font: 'number', size: 36, weight: 700, lineHeight: 1, letterSpacing: -0.02 },
  },
};
const faces = [{ family: 'Sora', url: '/fonts/Sora-Variable.ttf' }, { family: 'JetBrains Mono', url: '/fonts/JetBrainsMono-Variable.ttf' }];

test('camel case', () => {
  assert.equal(camel('on-surface-muted'), 'onSurfaceMuted');
  assert.equal(camel('xxl'), 'xxl');
});

test('Kotlin: colours become WhichrideColors constructors with ARGB literals', () => {
  const out = kotlin(model);
  assert.match(out, /^\/\/ Generated from design\/tokens\.json by design\/build\.mjs\. Do not edit\.\n/);
  assert.match(out, /package io\.github\.locadani\.whichride\.designsystem\n/);
  assert.match(out, /val LightColors: WhichrideColors = WhichrideColors\(\n        background = Color\(0xFFF3F6F8\),\n        onSurfaceMuted = Color\(0xFF4D5966\),\n    \)/);
  assert.match(out, /val DarkColors: WhichrideColors = WhichrideColors\(\n        background = Color\(0xFF0B0F14\),/);
  assert.match(out, /val BrandOnMark: Color = Color\(0xFF04261A\)/);
});

test('Kotlin: sizes are dp, type sizes are plain sp numbers', () => {
  const out = kotlin(model);
  assert.match(out, /object Space \{\n        val lg: Dp = 20\.dp\n        val md: Dp = 16\.dp\n    \}/);
  assert.match(out, /object Size \{\n        val touchTarget: Dp = 48\.dp\n    \}/);
  assert.match(out, /internal object Type \{\n        val weights: List<Int> = listOf\(400, 700\)\n/);
  assert.match(out, /val numberLarge: TypeToken = TypeToken\(FontRole\.NUMBER, sizeSp = 36f, weight = 700, lineHeight = 1f, letterSpacingEm = -0\.02f\)/);
  assert.match(out, /val bodyLarge: TypeToken = TypeToken\(FontRole\.TEXT, sizeSp = 16f, weight = 400, lineHeight = 1\.5f, letterSpacingEm = 0f\)/);
});

test('CSS: light by default, dark by system setting unless forced light, or forced dark', () => {
  const out = css(model, faces);
  assert.match(out, /^\/\* Generated from design\/tokens\.json by design\/build\.mjs\. Do not edit\. \*\/\n/);
  assert.match(out, /:root \{\n  color-scheme: light dark;\n[\s\S]*  --wr-color-background: #F3F6F8;\n/);
  assert.match(out, /@media \(prefers-color-scheme: dark\) \{\n  :root:not\(\[data-theme="light"\]\) \{\n    color-scheme: dark;\n    --wr-color-background: #0B0F14;\n/);
  assert.match(out, /:root\[data-theme="dark"\] \{\n  color-scheme: dark;\n  --wr-color-background: #0B0F14;\n/);
  assert.match(out, /:root\[data-theme="light"\] \{\n  color-scheme: light;\n\}/);
});

test('CSS: text and spacing scale with the user font size (rem), radii stay px', () => {
  const out = css(model, faces);
  assert.match(out, /--wr-space-lg: 1\.25rem;/);
  assert.match(out, /--wr-radius-md: 14px;/);
  assert.match(out, /--wr-type-body-large: 400 1rem\/1\.5 var\(--wr-font-text\);/);
  assert.match(out, /--wr-type-number-large: 700 2\.25rem\/1 var\(--wr-font-number\);/);
  assert.match(out, /--wr-type-number-large-tracking: -0\.02em;/);
  assert.doesNotMatch(out, /--wr-type-[a-z-]+: \d+ \d+px/);
});

test('CSS: self-hosted font faces; generic families are not quoted', () => {
  const out = css(model, faces);
  assert.match(out, /@font-face \{\n  font-family: "Sora";\n  src: url\("\/fonts\/Sora-Variable\.ttf"\) format\("truetype"\);\n  font-weight: 100 800;\n  font-display: swap;\n\}/);
  assert.match(out, /--wr-font-number: "JetBrains Mono", monospace;/);
  assert.doesNotMatch(out, /fonts\.googleapis|fonts\.gstatic/);
});

test('output does not depend on key order', () => {
  const shuffled = structuredClone(model);
  shuffled.space = { lg: 20, md: 16 };
  assert.equal(kotlin(shuffled), kotlin(model));
  assert.equal(css(shuffled, faces), css(model, faces));
});
