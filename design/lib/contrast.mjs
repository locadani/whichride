import { MODES, TokenError } from './model.mjs';

/** WCAG 2.2 relative luminance of an sRGB #RRGGBB colour. */
export function relativeLuminance(hex) {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG contrast ratio, from 1 to 21; never rounded. */
export function contrastRatio(a, b) {
  const [hi, lo] = [relativeLuminance(a), relativeLuminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

/** Two decimals rounded down, so a failing 4.497 never prints as 4.50. */
export function floor2(n) {
  return (Math.floor(n * 100) / 100).toFixed(2);
}

/** Checks every theme pair in both modes and every brand pair once. */
export function checkContrast(model, rules) {
  const results = [];
  const check = (scope, colours, pair) => {
    const min = rules.minimum[pair.kind];
    if (typeof min !== 'number') throw new TokenError(`contrast.json: unknown kind "${pair.kind}"`);
    for (const role of [pair.fg, pair.bg]) {
      if (!(role in colours)) throw new TokenError(`contrast.json: "${role}" is not a ${scope === 'brand' ? 'brand colour' : 'theme role'}`);
    }
    const ratio = contrastRatio(colours[pair.fg], colours[pair.bg]);
    results.push({ scope, fg: pair.fg, bg: pair.bg, kind: pair.kind, ratio, min, ok: ratio >= min });
  };
  for (const mode of MODES) for (const pair of rules.pairs) check(mode, model.themes[mode], pair);
  for (const pair of rules.brand) check('brand', model.brand, pair);
  return results;
}
