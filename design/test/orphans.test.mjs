import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readdir, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { build, OUTPUT_DIRS } from '../build.mjs';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));

async function* files(dir) {
  let entries;
  try {
    entries = await readdir(dir, { withFileTypes: true });
  } catch {
    return;
  }
  for (const e of entries) {
    if (['node_modules', 'build', '.gradle', 'dist', '.astro', 'generated'].includes(e.name)) continue;
    const path = join(dir, e.name);
    if (e.isDirectory()) yield* files(path);
    else yield path;
  }
}

/** Files in the repository that look generated (header or output folder) but the build no longer writes. */
async function orphans(root, written) {
  const found = [];
  for await (const path of files(join(root, 'android'))) found.push(path);
  for await (const path of files(join(root, 'web'))) found.push(path);
  const result = [];
  for (const path of found) {
    const rel = relative(root, path).split('\\').join('/');
    if (written.includes(rel)) continue;
    const inOutputDir = OUTPUT_DIRS.some((d) => rel.startsWith(`${d}/`));
    const text = inOutputDir ? '' : await readFile(path, 'utf8').catch(() => '');
    if (inOutputDir || text.includes('by design/build.mjs. Do not edit.')) result.push(rel);
  }
  return result;
}

test('no generated file is left behind after an output is renamed or dropped', async () => {
  const written = await build(await mkdtemp(join(tmpdir(), 'whichride-')));
  assert.deepEqual(await orphans(ROOT, written), []);
});

test('an orphaned output is found', async () => {
  const { mkdir, writeFile } = await import('node:fs/promises');
  const root = await mkdtemp(join(tmpdir(), 'orphan-'));
  await mkdir(join(root, 'android/designsystem/src/main/res/drawable'), { recursive: true });
  await writeFile(join(root, 'android/designsystem/src/main/res/drawable/old_logo.xml'), '<!-- Generated from design/logo.svg by design/build.mjs. Do not edit. -->\n');
  await mkdir(join(root, 'web/public/fonts'), { recursive: true });
  await writeFile(join(root, 'web/public/fonts/Old.ttf'), '');
  await mkdir(join(root, 'android/designsystem/src/main/kotlin'), { recursive: true });
  await writeFile(join(root, 'android/designsystem/src/main/kotlin/Theme.kt'), 'package x\n');
  assert.deepEqual((await orphans(root, [])).sort(), [
    'android/designsystem/src/main/res/drawable/old_logo.xml',
    'web/public/fonts/Old.ttf',
  ]);
});
