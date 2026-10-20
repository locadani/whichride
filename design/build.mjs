import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { css, kotlin } from './lib/formats.mjs';
import { logoOutputs, parseLogo, resolvedSvg } from './lib/logo.mjs';
import { loadModel, TokenError } from './lib/model.mjs';

const DESIGN = dirname(fileURLToPath(import.meta.url));
const ANDROID_MAIN = 'android/designsystem/src/main';
const KOTLIN_FILE = `${ANDROID_MAIN}/kotlin/io/github/locadani/whichride/designsystem/WhichrideTokens.kt`;

/** Folders that hold only build output; anything else in them is a leftover from an older build. */
export const OUTPUT_DIRS = [`${ANDROID_MAIN}/res/font`, 'web/public/fonts'];

/** Bundled fonts: the first family name in tokens.json → files in design/fonts/. */
const FONTS = [
  { family: 'Sora', file: 'Sora-Variable.ttf', licence: 'Sora-OFL.txt', android: 'sora.ttf' },
  { family: 'JetBrains Mono', file: 'JetBrainsMono-Variable.ttf', licence: 'JetBrainsMono-OFL.txt', android: 'jetbrains_mono.ttf' },
];

/** Writes every generated file under `root` (the repository root); returns their paths. */
export async function build(root) {
  const model = await loadModel(join(DESIGN, 'tokens.json'));
  for (const [role, families] of Object.entries(model.fonts)) {
    if (!FONTS.some((f) => f.family === families[0])) throw new TokenError(`font.${role}: no bundled file for "${families[0]}" in build.mjs`);
  }
  const logoText = await readFile(join(DESIGN, 'logo.svg'), 'utf8');
  const logo = parseLogo(logoText, model.brand);

  const texts = {
    [KOTLIN_FILE]: kotlin(model),
    'web/src/styles/tokens.css': css(model, FONTS.map((f) => ({ family: f.family, url: `/fonts/${f.file}` }))),
    'web/public/favicon.svg': resolvedSvg(logoText, model.brand),
  };
  for (const [path, text] of Object.entries(logoOutputs(logo))) texts[`${ANDROID_MAIN}/${path}`] = text;
  const copies = {};
  for (const f of FONTS) {
    copies[`${ANDROID_MAIN}/res/font/${f.android}`] = f.file;
    copies[`web/public/fonts/${f.file}`] = f.file;
    copies[`web/public/fonts/${f.licence}`] = f.licence;
  }

  for (const [path, text] of Object.entries(texts)) {
    await mkdir(dirname(join(root, path)), { recursive: true });
    await writeFile(join(root, path), text);
  }
  for (const [path, source] of Object.entries(copies)) {
    await mkdir(dirname(join(root, path)), { recursive: true });
    await copyFile(join(DESIGN, 'fonts', source), join(root, path));
  }
  return [...Object.keys(texts), ...Object.keys(copies)].sort();
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const written = await build(join(DESIGN, '..'));
  console.log(`wrote ${written.length} files:\n  ${written.join('\n  ')}`);
}
