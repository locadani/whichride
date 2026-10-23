import { XMLParser } from 'fast-xml-parser';
import { TokenError } from './model.mjs';

const NUMBER = /^\d+(\.\d+)?$/;
const REFERENCE = /^\{brand\.([a-z0-9-]+)\}$/;
const HEADER = '<!-- Generated from design/logo.svg by design/build.mjs. Do not edit. -->';
const ANDROID_NS = 'xmlns:android="http://schemas.android.com/apk/res/android"';
const ICON = 108; // adaptive icon canvas, dp
const VISIBLE = 72; // visible area inside the launcher's mask, dp

const fail = (msg) => {
  throw new TokenError(`logo.svg: ${msg}`);
};

/** A `{brand.<name>}` reference resolved against tokens.json; literal colours are refused. */
function brandColour(value, brand, what) {
  const name = REFERENCE.exec(value)?.[1];
  if (!name) fail(`the ${what} colour must be a {brand.<name>} reference to tokens.json, got "${value}"`);
  if (!(name in brand)) fail(`{brand.${name}} is not in tokens.json`);
  return brand[name];
}

/** The logo with every `{brand.<name>}` replaced by its colour: a plain SVG for browsers. */
export function resolvedSvg(text, brand) {
  parseLogo(text, brand);
  return text.replace(/\{brand\.([a-z0-9-]+)\}/g, (_, name) => brand[name]);
}

/**
 * Reads the logo in the only shape the generators understand: a square <svg> holding one
 * <rect> (the tile) and then <path> layers, painted in order and coloured by brand token
 * references. A layer is either filled (a tone that hides what is behind it) or stroked with
 * round caps and joins (a line). Android's themed icon draws the lines in one colour, so it
 * leaves out the fills and any line marked `data-monochrome="omit"`.
 * Anything else is an error, so the Android icons can never silently lose part of the logo.
 */
export function parseLogo(text, brand) {
  const parser = new XMLParser({ ignoreAttributes: false, attributeNamePrefix: '@_', isArray: (_name, _path, _leaf, isAttribute) => !isAttribute });
  const doc = parser.parse(text);
  const roots = Object.keys(doc);
  if (roots.length !== 1 || roots[0] !== 'svg' || doc.svg.length !== 1) fail('the file must contain exactly one <svg> element');
  const root = doc.svg[0];
  const attrs = (node, allowed, where, optional = []) => {
    const names = Object.keys(node).filter((k) => k.startsWith('@_')).map((k) => k.slice(2));
    for (const n of names) if (!allowed.includes(n) && !optional.includes(n)) fail(`<${where}> may not have "${n}"`);
    for (const n of allowed) if (!names.includes(n)) fail(`<${where}> needs "${n}"`);
    return Object.fromEntries(names.map((n) => [n, node[`@_${n}`]]));
  };
  const elements = Object.keys(root).filter((k) => !k.startsWith('@_'));
  for (const e of elements) if (!['rect', 'path'].includes(e)) fail(`<${e}> is not allowed`);
  if (root.rect?.length !== 1 || !root.path?.length) fail('exactly one <rect> and at least one <path> are needed');

  const svg = attrs(root, ['xmlns', 'viewBox'], 'svg');
  const box = svg.viewBox.split(' ');
  if (box.length !== 4 || box[0] !== '0' || box[1] !== '0' || box[2] !== box[3] || !NUMBER.test(box[2])) fail('viewBox must be "0 0 N N"');
  const size = Number(box[2]);

  const rect = attrs(root.rect[0], ['width', 'height', 'rx', 'fill'], 'rect');
  if (Number(rect.width) !== size || Number(rect.height) !== size) fail('the <rect> must fill the viewBox');
  if (!NUMBER.test(rect.rx) || Number(rect.rx) * 2 > size) fail('rx must be a number no larger than half the size');
  const tileFill = brandColour(rect.fill, brand, 'tile');

  const layers = root.path.map((node) => {
    if (!('@_stroke' in node)) {
      const path = attrs(node, ['d', 'fill'], 'path');
      return { d: path.d, fill: brandColour(path.fill, brand, 'fill') };
    }
    const path = attrs(node, ['d', 'fill', 'stroke', 'stroke-width', 'stroke-linecap', 'stroke-linejoin'], 'path', ['data-monochrome']);
    if (path.fill !== 'none') fail('a stroked path may not also be filled: fill must be "none"');
    if (!NUMBER.test(path['stroke-width'])) fail('stroke-width must be a number');
    if (path['stroke-linecap'] !== 'round' || path['stroke-linejoin'] !== 'round') fail('lines use round caps and joins');
    if (path['data-monochrome'] !== undefined && path['data-monochrome'] !== 'omit') fail('data-monochrome can only be "omit"');
    return {
      d: path.d,
      stroke: brandColour(path.stroke, brand, 'line'),
      strokeWidth: Number(path['stroke-width']),
      monochrome: path['data-monochrome'] !== 'omit',
    };
  });
  if (!layers.some((l) => l.monochrome)) fail('the themed icon needs at least one line not marked data-monochrome="omit"');

  return { size, tile: { rx: Number(rect.rx), fill: tileFill }, layers };
}

/** A rounded square as VectorDrawable path data (VectorDrawable has no <rect>). */
export function roundedRectPath(n, r) {
  return `M${r},0H${n - r}A${r},${r} 0 0,1 ${n},${r}V${n - r}A${r},${r} 0 0,1 ${n - r},${n}` +
    `H${r}A${r},${r} 0 0,1 0,${n - r}V${r}A${r},${r} 0 0,1 ${r},0Z`;
}

const layerPath = (layer, indent) =>
  layer.fill
    ? `${indent}<path android:pathData="${layer.d}" android:fillColor="${layer.fill}"/>`
    : `${indent}<path android:pathData="${layer.d}" android:strokeColor="${layer.stroke}" ` +
      `android:strokeWidth="${layer.strokeWidth}" android:strokeLineCap="round" android:strokeLineJoin="round"/>`;

/** Android resources, keyed by path relative to the module's src/main/. */
export function logoOutputs(logo) {
  const n = logo.size;
  const scale = VISIBLE / n;
  const offset = (ICON - VISIBLE) / 2;
  const xml = (body) => `<?xml version="1.0" encoding="utf-8"?>\n${HEADER}\n${body}\n`;
  const paths = (layers, indent) => layers.map((l) => layerPath(l, indent)).join('\n');
  const launcher = (layers) => xml(
    `<vector ${ANDROID_NS}\n    android:width="${ICON}dp" android:height="${ICON}dp"\n` +
      `    android:viewportWidth="${ICON}" android:viewportHeight="${ICON}">\n` +
      `    <group android:scaleX="${scale}" android:scaleY="${scale}" android:translateX="${offset}" android:translateY="${offset}">\n` +
      `${paths(layers, '        ')}\n    </group>\n</vector>`,
  );
  return {
    'res/drawable/whichride_logo.xml': xml(
      `<vector ${ANDROID_NS}\n    android:width="${n}dp" android:height="${n}dp"\n` +
        `    android:viewportWidth="${n}" android:viewportHeight="${n}">\n` +
        `    <path android:fillColor="${logo.tile.fill}" android:pathData="${roundedRectPath(n, logo.tile.rx)}"/>\n` +
        `${paths(logo.layers, '    ')}\n</vector>`,
    ),
    'res/drawable/ic_launcher_foreground.xml': launcher(logo.layers),
    'res/drawable/ic_launcher_monochrome.xml': launcher(logo.layers.filter((l) => l.monochrome)),
    'res/values/ic_launcher_background.xml': xml(
      `<resources>\n    <color name="ic_launcher_background">${logo.tile.fill}</color>\n</resources>`,
    ),
    'res/mipmap-anydpi/ic_launcher.xml': xml(
      `<adaptive-icon ${ANDROID_NS}>\n` +
        '    <background android:drawable="@color/ic_launcher_background"/>\n' +
        '    <foreground android:drawable="@drawable/ic_launcher_foreground"/>\n' +
        '    <monochrome android:drawable="@drawable/ic_launcher_monochrome"/>\n' +
        '</adaptive-icon>',
    ),
  };
}
