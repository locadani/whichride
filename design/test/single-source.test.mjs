import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdir, readFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));
const ANDROID_RES = 'android/designsystem/src/main/res';
const GENERATED = new Set([
  'android/designsystem/src/main/kotlin/io/github/locadani/whichride/designsystem/WhichrideTokens.kt',
  `${ANDROID_RES}/drawable/ic_launcher_foreground.xml`,
  `${ANDROID_RES}/drawable/whichride_logo.xml`,
  `${ANDROID_RES}/mipmap-anydpi/ic_launcher.xml`,
  `${ANDROID_RES}/values/ic_launcher_background.xml`,
  'web/src/styles/tokens.css',
]);

/** Every Android source set except tests, which may compare against literal expected values. */
const ANDROID_SOURCE = /^android\/[^/]+\/src\/(?!test\/|androidTest\/)/;
const KOTLIN = [
  /\b\d+(\.\d+)?f?\.(dp|sp|em)\b/, // 12.dp, 1.5f.sp
  /\(\s*\d+(\.\d+)?f?\s*\)\.(dp|sp|em)\b/, // (16).dp
  /\b(Dp|TextUnit|FontWeight)\(\s*\d/, // Dp(16f), FontWeight(700)
  /\bColor\(\s*(color\s*=\s*)?(0x|\d)/, // Color(0xFF…), Color(255, 0, 0)
  /\bColor\.[A-Z]/, // Color.White
  /\bFontWeight\.[A-Z]/, // FontWeight.Bold
  /\b0x[0-9A-Fa-f]{6,8}\b/, // bare ARGB
  /#[0-9A-Fa-f]{6}\b/, // "#3DDC97"
  /\bRoundedCornerShape\(\s*\d/, // Int overload is a percentage, Float is px
  /\balpha\s*=\s*\d/, // .copy(alpha = 0.5f)
];
const ANDROID_XML = [/#[0-9A-Fa-f]{3,8}\b/, /\b\d+(\.\d+)?(dp|dip|sp|px|pt)\b/];
const WEB = [
  /#[0-9A-Fa-f]{3,8}\b/,
  /\b\d*\.?\d+(px|rem|em|pt)\b/,
  /\b(rgba?|hsla?|hwb|lab|lch|oklab|oklch|color)\(/,
  /\b(color|background(-color)?|fill|stroke|border(-[a-z]+)*|outline(-color)?)\s*:[^;{}]*\b(white|black|red|green|blue|gr[ae]y|yellow|orange|purple|pink|silver|navy|teal|transparent)\b/,
  /font-weight\s*:\s*(\d|bold|bolder|lighter|normal)/,
];
const WEB_SOURCE = /^web\/src\//;
const RULES = [
  { ext: ['.kt'], under: ANDROID_SOURCE, literals: KOTLIN },
  { ext: ['.xml'], under: ANDROID_SOURCE, literals: ANDROID_XML },
  { ext: ['.css', '.scss', '.astro', '.html', '.js', '.jsx', '.ts', '.tsx', '.svelte', '.vue'], under: WEB_SOURCE, literals: WEB },
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
      const rule = RULES.find((r) => r.ext.some((e) => rel.endsWith(e)) && r.under.test(rel));
      if (!rule || GENERATED.has(rel)) continue;
      const lines = (await readFile(path, 'utf8')).split('\n');
      lines.forEach((line, i) => {
        if (rule.literals.some((re) => re.test(line))) found.push(`${rel}:${i + 1}: ${line.trim()}`);
      });
    }
  }
  return found;
}

test('hand-written UI code takes every colour and size from tokens.json', async () => {
  assert.deepEqual(await literalGraphicValues(ROOT), []);
});

async function guardOn(files) {
  const { mkdtemp, mkdir, writeFile } = await import('node:fs/promises');
  const { tmpdir } = await import('node:os');
  const { dirname } = await import('node:path');
  const root = await mkdtemp(join(tmpdir(), 'guard-'));
  for (const [path, lines] of Object.entries(files)) {
    await mkdir(dirname(join(root, path)), { recursive: true });
    await writeFile(join(root, path), lines.join('\n') + '\n');
  }
  return literalGraphicValues(root);
}

const KOTLIN_LITERALS = [
  'val a = 12.dp', 'val a = 1.5f.sp', 'val a = (16).dp', 'val a = Dp(16f)', 'val a = TextUnit(16f, TextUnitType.Sp)',
  'val a = Color(0xFF000000)', 'val a = Color( 0xFF000000)', 'val a = Color(color = 0xFF000000)', 'val a = Color(255, 0, 0)',
  'val a = Color.White', 'val a = 0xFF3DDC97', 'val a = "#3DDC97"',
  'val a = RoundedCornerShape(12)', 'val a = RoundedCornerShape(12f)', 'val a = c.copy(alpha = 0.5f)',
  'val a = FontWeight.Bold', 'val a = FontWeight(700)',
];
const KOTLIN_ALLOWED = [
  'val a = WhichrideTokens.Space.md', 'val a = FontWeight(weight)', 'val a = sizeSp.sp', 'val a = letterSpacingEm.em',
  'val a = RoundedCornerShape(WhichrideTokens.Radius.md)', 'val a = MaterialTheme.colorScheme.primary',
];
const XML_LITERALS = ['<color name="x">#FF6750A4</color>', '<item name="android:windowBackground">#FFFFFF</item>', 'android:layout_width="12dp"', 'android:textSize="14sp"'];
const WEB_LITERALS = [
  'a { color: #fff; }', 'a { margin: 12px; }', 'a { font-size: 1.1em; }', 'a { font-size: 1rem; }', 'a { color: rgb(0 0 0); }',
  'a { color: hsl(0 0% 0%); }', 'a { color: oklch(0.7 0.1 150); }', 'a { color: white; }', 'a { background: black; }',
  'a { font-weight: 700; }', 'a { font-weight: bold; }',
];
const WEB_ALLOWED = ['a { margin: var(--wr-space-md); color: var(--wr-color-on-surface); font: var(--wr-type-body-large); width: 100%; }'];

test('the guard catches every literal colour, size and weight', async () => {
  const found = await guardOn({
    'android/app/src/main/kotlin/Screen.kt': KOTLIN_LITERALS,
    'android/app/src/debug/kotlin/Debug.kt': ['val a = 12.dp'],
    'android/app/src/main/res/values/themes.xml': XML_LITERALS,
    'web/src/styles/site.css': WEB_LITERALS,
    'web/src/components/Card.tsx': ['const style = { color: "#fff" };'],
  });
  const count = (prefix) => found.filter((f) => f.startsWith(prefix)).length;
  assert.equal(count('android/app/src/main/kotlin/Screen.kt'), KOTLIN_LITERALS.length, found.join('\n'));
  assert.equal(count('android/app/src/debug/'), 1, found.join('\n'));
  assert.equal(count('android/app/src/main/res/'), XML_LITERALS.length, found.join('\n'));
  assert.equal(count('web/src/styles/site.css'), WEB_LITERALS.length, found.join('\n'));
  assert.equal(count('web/src/components/'), 1, found.join('\n'));
});

test('the guard allows token references, tests and generated files', async () => {
  const found = await guardOn({
    'android/app/src/main/kotlin/Screen.kt': KOTLIN_ALLOWED,
    'android/app/src/test/kotlin/ScreenTest.kt': ['assertEquals(Color(0xFF3DDC97), x)'],
    'android/app/src/main/AndroidManifest.xml': ['<application android:icon="@mipmap/ic_launcher"/>'],
    'android/designsystem/src/main/res/values/ic_launcher_background.xml': ['<color name="ic_launcher_background">#3DDC97</color>'],
    'web/src/styles/site.css': WEB_ALLOWED,
  });
  assert.deepEqual(found, []);
});
