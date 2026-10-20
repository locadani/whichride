import { MODES } from './model.mjs';

const KOTLIN_HEADER = '// Generated from design/tokens.json by design/build.mjs. Do not edit.\n';
const CSS_HEADER = '/* Generated from design/tokens.json by design/build.mjs. Do not edit. */\n';
const GENERIC_FAMILIES = new Set(['serif', 'sans-serif', 'monospace', 'cursive', 'fantasy', 'system-ui']);

export const camel = (name) => name.replace(/-([a-z0-9])/g, (_, c) => c.toUpperCase());
const pascal = (name) => camel(name).replace(/^./, (c) => c.toUpperCase());
const sorted = (record) => Object.keys(record).sort().map((k) => [k, record[k]]);
const argb = (hex) => `Color(0xFF${hex.slice(1)})`;
const kfloat = (n) => `${n}f`;
const rem = (px) => `${px / 16}rem`;

/** Kotlin for the designsystem module: colour sets, dimensions and type tokens. */
export function kotlin(model) {
  const lines = [
    KOTLIN_HEADER.trimEnd(),
    'package io.github.locadani.whichride.designsystem',
    '',
    'import androidx.compose.ui.graphics.Color',
    'import androidx.compose.ui.unit.Dp',
    'import androidx.compose.ui.unit.dp',
    '',
    '/** Values from design/tokens.json. Use [WhichrideTheme] for colours and text styles. */',
    'object WhichrideTokens {',
  ];
  for (const mode of MODES) {
    lines.push(`    val ${pascal(mode)}Colors: WhichrideColors = WhichrideColors(`);
    for (const [role, hex] of sorted(model.themes[mode])) lines.push(`        ${camel(role)} = ${argb(hex)},`);
    lines.push('    )', '');
  }
  for (const [name, hex] of sorted(model.brand)) lines.push(`    val Brand${pascal(name)}: Color = ${argb(hex)}`);
  for (const group of ['space', 'radius', 'size']) {
    lines.push('', `    object ${pascal(group)} {`);
    for (const [name, px] of sorted(model[group])) lines.push(`        val ${camel(name)}: Dp = ${px}.dp`);
    lines.push('    }');
  }
  const weights = [...new Set(Object.values(model.type).map((t) => t.weight))].sort((a, b) => a - b);
  lines.push('', '    internal object Type {', `        val weights: List<Int> = listOf(${weights.join(', ')})`);
  for (const [role, t] of sorted(model.type)) {
    lines.push(
      `        val ${camel(role)}: TypeToken = TypeToken(FontRole.${t.font.toUpperCase()}, ` +
        `sizeSp = ${kfloat(t.size)}, weight = ${t.weight}, lineHeight = ${kfloat(t.lineHeight)}, ` +
        `letterSpacingEm = ${kfloat(t.letterSpacing)})`,
    );
  }
  lines.push('    }', '}', '');
  return lines.join('\n');
}

/** CSS custom properties for the website; light by default, dark by preference or `data-theme`. */
export function css(model, fontFaces) {
  const out = [CSS_HEADER];
  for (const face of fontFaces) {
    out.push(
      '@font-face {',
      `  font-family: "${face.family}";`,
      `  src: url("${face.url}") format("truetype");`,
      '  font-weight: 100 800;',
      '  font-display: swap;',
      '}',
      '',
    );
  }
  const colours = (mode, indent) => sorted(model.themes[mode]).map(([role, hex]) => `${indent}--wr-color-${role}: ${hex};`);
  out.push(':root {', '  color-scheme: light dark;');
  for (const [name, hex] of sorted(model.brand)) out.push(`  --wr-brand-${name}: ${hex};`);
  for (const [role, families] of sorted(model.fonts)) {
    const list = families.map((f) => (GENERIC_FAMILIES.has(f) ? f : `"${f}"`)).join(', ');
    out.push(`  --wr-font-${role}: ${list};`);
  }
  for (const [name, px] of sorted(model.space)) out.push(`  --wr-space-${name}: ${rem(px)};`);
  for (const [name, px] of sorted(model.radius)) out.push(`  --wr-radius-${name}: ${px}px;`);
  for (const [name, px] of sorted(model.size)) out.push(`  --wr-size-${name}: ${px}px;`);
  for (const [role, t] of sorted(model.type)) {
    out.push(`  --wr-type-${role}: ${t.weight} ${rem(t.size)}/${t.lineHeight} var(--wr-font-${t.font});`);
    out.push(`  --wr-type-${role}-tracking: ${t.letterSpacing}em;`);
  }
  out.push(...colours('light', '  '), '}', '');
  out.push('@media (prefers-color-scheme: dark) {', '  :root:not([data-theme="light"]) {', '    color-scheme: dark;');
  out.push(...colours('dark', '    '), '  }', '}', '');
  out.push(':root[data-theme="dark"] {', '  color-scheme: dark;', ...colours('dark', '  '), '}', '');
  out.push(':root[data-theme="light"] {', '  color-scheme: light;', '}', '');
  return out.join('\n');
}
