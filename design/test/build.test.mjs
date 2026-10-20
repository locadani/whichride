import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { build } from '../build.mjs';

test('the build writes every output, identically on a second run', async () => {
  const root = await mkdtemp(join(tmpdir(), 'whichride-'));
  const written = await build(root);
  assert.deepEqual(written, [
    'android/designsystem/src/main/kotlin/io/github/locadani/whichride/designsystem/WhichrideTokens.kt',
    'android/designsystem/src/main/res/drawable/ic_launcher_foreground.xml',
    'android/designsystem/src/main/res/drawable/whichride_logo.xml',
    'android/designsystem/src/main/res/font/jetbrains_mono.ttf',
    'android/designsystem/src/main/res/font/sora.ttf',
    'android/designsystem/src/main/res/mipmap-anydpi/ic_launcher.xml',
    'android/designsystem/src/main/res/values/ic_launcher_background.xml',
    'web/public/favicon.svg',
    'web/public/fonts/JetBrainsMono-OFL.txt',
    'web/public/fonts/JetBrainsMono-Variable.ttf',
    'web/public/fonts/Sora-OFL.txt',
    'web/public/fonts/Sora-Variable.ttf',
    'web/src/styles/tokens.css',
  ]);
  const first = await Promise.all(written.map((p) => readFile(join(root, p))));
  await build(root);
  const second = await Promise.all(written.map((p) => readFile(join(root, p))));
  assert.deepEqual(second, first);
  const css = await readFile(join(root, 'web/src/styles/tokens.css'), 'utf8');
  assert.match(css, /url\("\/fonts\/Sora-Variable\.ttf"\)/);
  const favicon = await readFile(join(root, 'web/public/favicon.svg'), 'utf8');
  assert.match(favicon, /fill="#3DDC97"/);
});
