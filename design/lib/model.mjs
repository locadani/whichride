import StyleDictionary from 'style-dictionary';

/** Both themes; every theme role must exist in each. */
export const MODES = ['light', 'dark'];

const HEX = /^#[0-9A-F]{6}$/;
const PX = /^(0|[1-9]\d*)(\.\d+)?px$/;
const EM = /^-?(0|[1-9]\d*)(\.\d+)?em$/;
const NAME = /^[a-z0-9]+(-[a-z0-9]+)*$/;
const WEIGHTS = [100, 200, 300, 400, 500, 600, 700, 800, 900];

/** A token file that doesn't follow the rules in design/README.md. */
export class TokenError extends Error {}

/** Reads a DTCG token file with Style Dictionary and resolves every `{reference}`. */
export async function resolveTokens(sourcePath) {
  const sd = new StyleDictionary({
    source: [sourcePath],
    usesDtcg: true,
    log: { verbosity: 'silent' },
    platforms: { model: { transforms: [] } },
  });
  const { allTokens } = await sd.getPlatformTokens('model');
  return allTokens.map((t) => ({ path: t.path, type: t.$type, value: t.$value }));
}

export async function loadModel(sourcePath) {
  return toModel(await resolveTokens(sourcePath));
}

/** Validates resolved tokens and groups them by what the generators need. */
export function toModel(tokens) {
  const model = { themes: { light: {}, dark: {} }, brand: {}, space: {}, radius: {}, size: {}, fonts: {}, type: {} };
  const typography = [];
  for (const t of tokens) {
    const where = t.path.join('.');
    for (const part of t.path) {
      if (!NAME.test(part)) throw new TokenError(`${where}: "${part}" is not a lower-case kebab-case name`);
    }
    const [group, ...rest] = t.path;
    switch (group) {
      case 'palette':
        hex(t, where);
        break;
      case 'brand':
        model.brand[leaf(rest, where)] = hex(t, where);
        break;
      case 'theme':
        if (rest.length !== 2 || !MODES.includes(rest[0])) {
          throw new TokenError(`${where}: theme tokens are theme.<light|dark>.<role>`);
        }
        model.themes[rest[0]][rest[1]] = hex(t, where);
        break;
      case 'space':
      case 'radius':
      case 'size':
        model[group][leaf(rest, where)] = px(t.value, t.type, where);
        break;
      case 'font':
        model.fonts[leaf(rest, where)] = family(t, where);
        break;
      case 'type':
        typography.push(t);
        break;
      default:
        throw new TokenError(`${where}: unknown group "${group}"`);
    }
  }
  for (const t of typography) {
    const where = t.path.join('.');
    model.type[leaf(t.path.slice(1), where)] = typeStyle(t, model.fonts, where);
  }
  const [light, dark] = MODES.map((m) => Object.keys(model.themes[m]));
  const onlyLight = light.filter((r) => !dark.includes(r));
  const onlyDark = dark.filter((r) => !light.includes(r));
  if (onlyLight.length || onlyDark.length) {
    throw new TokenError(`theme roles differ between modes: only light [${onlyLight}], only dark [${onlyDark}]`);
  }
  return model;
}

function leaf(rest, where) {
  if (rest.length !== 1) throw new TokenError(`${where}: expected exactly one name below the group`);
  return rest[0];
}

function hex(t, where) {
  if (t.type !== 'color' || typeof t.value !== 'string' || !HEX.test(t.value)) {
    throw new TokenError(`${where}: colours are upper-case #RRGGBB, got ${JSON.stringify(t.value)}`);
  }
  return t.value;
}

function px(value, type, where) {
  if (type !== 'dimension' || typeof value !== 'string' || !PX.test(value)) {
    throw new TokenError(`${where}: dimensions are written like "16px", got ${JSON.stringify(value)}`);
  }
  return Number(value.slice(0, -2));
}

function family(t, where) {
  const ok = t.type === 'fontFamily' && Array.isArray(t.value) && t.value.length >= 2 &&
    t.value.every((f) => typeof f === 'string' && f.length > 0);
  if (!ok) throw new TokenError(`${where}: a font is a list of a family and a generic fallback`);
  return t.value;
}

function typeStyle(t, fonts, where) {
  const v = t.value;
  if (t.type !== 'typography' || typeof v !== 'object' || v === null) {
    throw new TokenError(`${where}: expected a typography token`);
  }
  const font = Object.keys(fonts).find((role) => JSON.stringify(fonts[role]) === JSON.stringify(v.fontFamily));
  if (!font) throw new TokenError(`${where}: fontFamily ${JSON.stringify(v.fontFamily)} is not a declared font`);
  if (!WEIGHTS.includes(v.fontWeight)) throw new TokenError(`${where}: fontWeight must be one of ${WEIGHTS}`);
  if (typeof v.lineHeight !== 'number' || v.lineHeight < 1 || v.lineHeight > 2) {
    throw new TokenError(`${where}: lineHeight is a multiple of the size between 1 and 2`);
  }
  let letterSpacing;
  if (v.letterSpacing === '0') letterSpacing = 0;
  else if (typeof v.letterSpacing === 'string' && EM.test(v.letterSpacing)) letterSpacing = Number(v.letterSpacing.slice(0, -2));
  else throw new TokenError(`${where}: letterSpacing is "0" or in em, got ${JSON.stringify(v.letterSpacing)}`);
  return { font, size: px(v.fontSize, 'dimension', `${where}.fontSize`), weight: v.fontWeight, lineHeight: v.lineHeight, letterSpacing };
}
