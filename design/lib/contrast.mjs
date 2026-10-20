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

/**
 * Checks every theme pair in both modes and every brand pair once. A pair with
 * `"fgTheme": "other"` takes its foreground from the opposite theme, as Material's
 * `inversePrimary` does on the inverse surface.
 */
export function checkContrast(model, rules) {
  const results = [];
  const check = (scope, fgColours, bgColours, pair, fgLabel) => {
    const min = Object.hasOwn(rules.minimum, pair.kind) ? rules.minimum[pair.kind] : undefined;
    if (typeof min !== 'number') throw new TokenError(`contrast.json: unknown kind "${pair.kind}"`);
    const what = scope === 'brand' ? 'brand colour' : 'theme role';
    for (const [role, colours] of [[pair.fg, fgColours], [pair.bg, bgColours]]) {
      if (!Object.hasOwn(colours, role)) throw new TokenError(`contrast.json: "${role}" is not a ${what}`);
    }
    const ratio = contrastRatio(fgColours[pair.fg], bgColours[pair.bg]);
    results.push({ scope, fg: fgLabel, bg: pair.bg, kind: pair.kind, ratio, min, ok: ratio >= min });
  };
  for (const mode of MODES) {
    const other = MODES.find((m) => m !== mode);
    for (const pair of rules.pairs) {
      if (pair.fgTheme !== undefined && pair.fgTheme !== 'other') {
        throw new TokenError(`contrast.json: fgTheme is "other" or absent, got "${pair.fgTheme}"`);
      }
      const fgMode = pair.fgTheme === 'other' ? other : mode;
      const label = pair.fgTheme === 'other' ? `${pair.fg} (${other})` : pair.fg;
      check(mode, model.themes[fgMode], model.themes[mode], pair, label);
    }
  }
  for (const pair of rules.brand) check('brand', model.brand, model.brand, pair, pair.fg);
  return results;
}
