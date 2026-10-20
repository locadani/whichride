import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdir, readFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));
const GENERATED = new Set([
  'android/designsystem/src/main/kotlin/io/github/locadani/whichride/designsystem/WhichrideTokens.kt',
  'web/src/styles/tokens.css',
]);
const RULES = [
  { ext: '.kt', under: /^android\/[^/]+\/src\/main\//, literal: /Color\(0x|\b\d+(\.\d+)?f?\.(dp|sp|em)\b|#[0-9A-Fa-f]{6}\b/ },
  { ext: '.css', under: /^web\/src\//, literal: /#[0-9A-Fa-f]{3,8}\b|\b\d*\.?\d+(px|rem)\b|rgba?\(/ },
  { ext: '.astro', under: /^web\/src\//, literal: /#[0-9A-Fa-f]{3,8}\b|\b\d*\.?\d+(px|rem)\b|rgba?\(/ },
];

async function* files(dir) {
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const e of entries) {
    if (['node_modules', 'build', '.gradle'].includes(e.name)) continue;
    const path = join(dir, e.name);
    if (e.isDirectory()) yield* files(path);
    else yield path;
  }
}

/** Literal graphic values in hand-written UI source, as "path:line: text". */
export async function literalGraphicValues(root) {
  const found = [];
  for (const top of ['android', 'web']) {
    for await (const path of files(join(root, top))) {
      const rel = relative(root, path).split('\\').join('/');
      const rule = RULES.find((r) => rel.endsWith(r.ext) && r.under.test(rel));
      if (!rule || GENERATED.has(rel)) continue;
      const lines = (await readFile(path, 'utf8')).split('\n');
      lines.forEach((line, i) => {
        if (rule.literal.test(line)) found.push(`${rel}:${i + 1}: ${line.trim()}`);
      });
    }
  }
  return found;
}

test('hand-written UI code takes every colour and size from tokens.json', async () => {
  assert.deepEqual(await literalGraphicValues(ROOT), []);
});

test('the guard catches literals', async () => {
  const { mkdtemp, mkdir, writeFile } = await import('node:fs/promises');
  const { tmpdir } = await import('node:os');
  const root = await mkdtemp(join(tmpdir(), 'guard-'));
  await mkdir(join(root, 'android/app/src/main/kotlin'), { recursive: true });
  await writeFile(join(root, 'android/app/src/main/kotlin/Screen.kt'), 'val a = 12.dp\nval b = Color(0xFF000000)\nval c = WhichrideTokens.Space.md\n');
  await mkdir(join(root, 'web/src/styles'), { recursive: true });
  await writeFile(join(root, 'web/src/styles/site.css'), 'a { color: #fff; margin: var(--wr-space-md); }\n');
  const found = await literalGraphicValues(root);
  assert.equal(found.length, 3, found.join('\n'));
});
