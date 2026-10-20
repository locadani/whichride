# Milestone 4: Design system — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One `design/tokens.json` and one `design/logo.svg` that generate the Android theme
(colours, type, shapes, launcher icon) and the website's `tokens.css` and favicon. CI fails when a
generated file is stale or a colour pair misses WCAG AA.

**Architecture:** `design/` is a small Node project. Style Dictionary reads and resolves the W3C
(DTCG) token file. Our own pure functions then validate the resolved tokens into a *model*
and write Kotlin and CSS from it. A separate pure function turns the constrained logo SVG into
Android vector drawables. Generated files are committed where they are used:
`android/designsystem/…` (a new Gradle library module, the only module until milestone 5) and
`web/…` (the Astro site that wraps them comes in milestone 7). The Android module wraps the
generated values in `WhichrideTheme`, a Material 3 theme with every colour role mapped.

**Tech Stack:** Node 24 (`node:test`), style-dictionary 5.6.1, fast-xml-parser 5.11.2; Gradle
9.8.1, Android Gradle Plugin 9.4.1 (built-in Kotlin), Kotlin Compose plugin 2.4.21, Compose BOM
2026.09.00 (Material 3), JUnit 4.13.2, compileSdk 37, minSdk 26 (the versions the UniFFI spike
proved on the owner's phone); GitHub Actions.

**Spec:** `docs/specs/2026-10-19-carsharing-compare-design.md`: §10.5 (shared brand assets),
§11 (accessibility), §2 (one source for brand assets). Decisions: `docs/DECISIONS.md` (rows 1–50;
this plan adds 51–57). **Design source:** the claude.ai Design canvas
https://claude.ai/artifact/P2A5aUCR45ZBKhnRRqCW2b (artboards Home, Main, PriceCheck, Discounts,
NightWeb, ProviderWeb). Every colour, font, radius and size below was taken from it. Where this
plan departs from the canvas, the decision rows say why.

## What the canvas specifies (reference for every task)

- **Colours** (each artboard's `renderVals`, `dark` default `true`):
  dark: bg `#0B0F14`, surface `#151B23`, sheet `#11161D`, field-in-sheet `#1D252F`, chip
  `#232B36`, line `#2C3642`, scrim `#06090C`, ink `#E8EEF5`, muted `#9AA7B6`, accent `#3DDC97`,
  accentInk `#04261A`, hero = accent with accentInk, disabled `#1D252F` with muted.
  light: bg `#F3F6F8`, surface/sheet `#FFFFFF`, field-in-sheet `#EEF2F5`, chip `#E3E9EE`, line
  `#CBD4DC`, scrim `#C9D1D8`, ink `#0B1218`, muted `#4D5966`, accent `#0B7A50`, accentInk
  `#FFFFFF`, hero `#0B1218` with `#E8EEF5`, disabled `#E3E9EE` with muted.
- **Fonts:** Sora 400–700 for text, JetBrains Mono 500/700 for prices, distances and times.
- **Logo:** a 32 px tile with 10 px radius in the accent colour, holding a "Y" fork (`M12 21v-8M12 13L6 4M12 13l6-9`
  in a 24-unit box drawn at 18 px, stroke 2.4, round caps and joins) in accentInk.
- **Radii:** 10 (logo tile), 14 (rows, inputs), 16 (buttons), 20 (cards), 28 (bottom sheet top),
  999 (pills).
- **Controls:** primary buttons 56 px high, secondary 48–54, icon buttons 44.

## Global Constraints

- Work on branch `feat/m4-design-system` (cut from `main` at `ac0e4f0`; this plan is its first
  commit). Never commit to `main`. Pushes go to `git@github-locadani:locadani/whichride.git`
  (SSH alias for the `locadani` key). Never use `gh` for this repo unless `gh auth status` shows
  `locadani` as the active account; the owner opens PRs and squash-merges.
- Before every commit, `git config user.name` must print `locadani` and `git config user.email`
  must print `61902944+locadani@users.noreply.github.com`. Never put any email address in a
  tracked file.
- The gitleaks pre-commit hook must be active (`git config core.hooksPath .githooks`). No secret
  is needed or handled in this milestone.
- Every commit message ends with a blank line followed by
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Commit subjects use Conventional Commits: `feat(design): …`, `feat(android): …`, `ci: …`,
  `build: …`, `docs: …`.
- Doc comments explain *what* and *why*; no tutorial-style comments.
- **One file for every graphic value (owner requirement, spec §10.5):** every colour, font,
  weight, size, spacing, radius and the logo's colours live only in `design/tokens.json`.
  `design/logo.svg` holds only the logo's *shape* and names its colours as token references
  (`{brand.mark}`); `design/fonts/` holds the font files tokens.json names. Hand-written Kotlin
  (and later CSS/Astro) reads generated values and never writes a literal colour, `dp` or `sp`
  number. A test enforces it (Task 6). Every file the build writes starts
  with a "Generated … Do not edit." comment (except fonts, which are copied byte for byte).
- **Accessibility (spec §11):** text sizes are `sp` on Android and `rem` on the web, never `px`,
  so the user's font size setting applies. Text colour pairs need ≥ 4.5:1 and component
  boundaries ≥ 3:1 (WCAG 2.2 AA 1.4.3 and 1.4.11), checked without rounding. Minimum touch
  target 48 dp.
- **Privacy (spec §9):** no font or asset is loaded from a third-party server. Fonts are bundled
  in the app and self-hosted on the website.
- Exact dependency versions only (`"5.6.1"`, never `"^5.6.1"`); `package-lock.json` is
  committed and CI uses `npm ci`.
- GitHub Actions: first-party `actions/*` by major tag (as `ci.yml` does today), third-party
  actions pinned to a full commit SHA with the version in a comment (as the docker actions are).
- Disk: don't run `cargo clean`, `docker rmi` or `rm -rf` on build folders. The owner declined
  disk cleanup.
- Anything outward-facing (pushing, PRs) is done by the owner. Tasks that need it say **OWNER**
  and stop.

## Review Focus

1. **A Material component showing Material's default purple.** A `FilterChip`, `Switch`,
   `Snackbar` or `NavigationBar` reads a `ColorScheme` role we never set (`secondaryContainer`,
   `inversePrimary`, the `*Fixed` roles…). Every colour in both schemes must come from our
   tokens (reflection test over every `ColorScheme` colour, Task 5).
2. **The user's large font setting ignored.** A text size written in `px` on the web or `dp` on
   Android doesn't scale. Type and spacing are `rem` in CSS and `sp` in Compose (tests in
   Tasks 3 and 5).
3. **A contrast of 4.48 reported as passing.** Rounding to one decimal before comparing turns
   4.48 into 4.5. The check compares the unrounded ratio and prints it rounded *down*
   (`#777777` on white must fail, Task 2).
4. **A typo that silently disables a check.** A contrast rule naming a role that doesn't exist,
   or a role added to only one of the two themes, must stop the build with the name of the
   role, not skip it (tests in Tasks 1 and 2).
5. **A graphic value typed outside tokens.json.** Someone writes `Color(0xFF…)` or `12.dp`
   in a screen, or a hex colour into `logo.svg`, and the app, the icon and the website drift
   apart. `logo.svg` accepts only `{brand.*}` references (Task 4), and a test scans the
   hand-written Kotlin for literal colours, `dp` and `sp` (Task 6).

---

## File map

```
design/
  package.json, package-lock.json, .nvmrc, README.md
  tokens.json              source: palette, brand, two themes, fonts, type, space, radius, size
  contrast.json            source: colour pairs that must meet WCAG AA
  logo.svg                 source: the logo (tile + mark), constrained format
  fonts/                   source: Sora-Variable.ttf, JetBrainsMono-Variable.ttf + OFL licences
  lib/model.mjs            resolve tokens with Style Dictionary, validate into a model
  lib/contrast.mjs         WCAG maths and the pair check
  lib/formats.mjs          model → Kotlin, model → CSS
  lib/logo.mjs             logo.svg → Android vector drawables
  build.mjs                writes every generated file
  contrast.mjs             CLI: prints the contrast table, exits 1 on a failure
  test/*.test.mjs          node:test suites
android/
  settings.gradle.kts, build.gradle.kts, gradle.properties, gradle/libs.versions.toml,
  gradlew, gradlew.bat, gradle/wrapper/*
  designsystem/build.gradle.kts
  designsystem/src/main/kotlin/io/github/locadani/whichride/designsystem/
    WhichrideTokens.kt     GENERATED
    WhichrideColors.kt, TypeToken.kt, Type.kt, Shapes.kt, MaterialColors.kt, WhichrideTheme.kt,
    ThemePreview.kt
  designsystem/src/main/res/  GENERATED: drawable/whichride_logo.xml,
    drawable/ic_launcher_foreground.xml, values/ic_launcher_background.xml,
    mipmap-anydpi/ic_launcher.xml, font/sora.ttf, font/jetbrains_mono.ttf
  designsystem/src/test/kotlin/io/github/locadani/whichride/designsystem/ThemeTest.kt
web/
  src/styles/tokens.css    GENERATED
  public/favicon.svg       GENERATED (design/logo.svg with its colours resolved)
  public/fonts/            GENERATED (font and licence copies)
```

---

### Task 1: Token source and model

**Files:**
- Create: `design/package.json`, `design/.nvmrc`, `design/tokens.json`, `design/lib/model.mjs`,
  `design/test/model.test.mjs`
- Modify: `.gitignore`

**Interfaces:**
- Produces: `resolveTokens(sourcePath: string): Promise<Token[]>` where
  `Token = { path: string[], type: string, value: unknown }`;
  `toModel(tokens: Token[]): Model`; `loadModel(sourcePath: string): Promise<Model>`;
  `class TokenError extends Error`; `MODES = ['light', 'dark']`.
  `Model = { themes: { light: Record<role, hex>, dark: Record<role, hex> }, brand: Record<name, hex>, space: Record<name, px:number>, radius: Record<name, number>, size: Record<name, number>, fonts: Record<role, string[]>, type: Record<role, { font: string, size: number, weight: number, lineHeight: number, letterSpacing: number }> }`.
  Hex values are `#RRGGBB` upper-case; `letterSpacing` is in `em`; `lineHeight` is a multiple
  of `size`.

- [ ] **Step 1: Create the Node project**

```bash
mkdir -p design/lib design/test
printf '24\n' > design/.nvmrc
```

`design/package.json`:

```json
{
  "name": "whichride-design",
  "private": true,
  "type": "module",
  "description": "Design tokens and logo for whichride; generates the Android theme and the website's CSS.",
  "engines": { "node": ">=24" },
  "scripts": {
    "test": "node --test",
    "contrast": "node contrast.mjs",
    "build": "node build.mjs"
  },
  "devDependencies": {
    "fast-xml-parser": "5.11.2",
    "style-dictionary": "5.6.1"
  }
}
```

```bash
cd design && npm install && cd ..
```

Append to `.gitignore`:

```
# Design tokens (Node)
node_modules/

# Android
/android/.gradle/
/android/**/build/
/android/.kotlin/
/android/local.properties
```

- [ ] **Step 2: Write `design/tokens.json`**

All values are the canvas values listed above. Role names follow Material 3's colour roles, so
the mapping in Task 5 is obvious. The error colours are not in the canvas; they are Material 3's
baseline error colours (decision 51).

```json
{
  "$description": "whichride design tokens (W3C DTCG format). The only place colours, fonts and sizes are edited; run `npm run build` in design/ after changing anything.",
  "palette": {
    "$type": "color",
    "gray-0": { "$value": "#FFFFFF" },
    "gray-25": { "$value": "#F3F6F8" },
    "gray-50": { "$value": "#EEF2F5" },
    "gray-75": { "$value": "#E8EEF5" },
    "gray-100": { "$value": "#E3E9EE" },
    "gray-200": { "$value": "#CBD4DC" },
    "gray-225": { "$value": "#C9D1D8" },
    "gray-400": { "$value": "#9AA7B6" },
    "gray-600": { "$value": "#4D5966" },
    "gray-800": { "$value": "#2C3642" },
    "gray-825": { "$value": "#232B36" },
    "gray-850": { "$value": "#1D252F" },
    "gray-875": { "$value": "#151B23" },
    "gray-900": { "$value": "#11161D" },
    "gray-925": { "$value": "#0B1218" },
    "gray-950": { "$value": "#0B0F14" },
    "gray-975": { "$value": "#06090C" },
    "green-400": { "$value": "#3DDC97" },
    "green-700": { "$value": "#0B7A50" },
    "green-950": { "$value": "#04261A" },
    "red-200": { "$value": "#F2B8B5" },
    "red-700": { "$value": "#B3261E" },
    "red-900": { "$value": "#601410" }
  },
  "brand": {
    "$type": "color",
    "$description": "Logo colours. The same in both themes, so the icon is recognisable everywhere.",
    "mark": { "$value": "{palette.green-400}" },
    "on-mark": { "$value": "{palette.green-950}" }
  },
  "theme": {
    "$type": "color",
    "light": {
      "background": { "$value": "{palette.gray-25}" },
      "surface": { "$value": "{palette.gray-0}", "$description": "Cards and lists" },
      "surface-low": { "$value": "{palette.gray-0}", "$description": "Bottom sheets" },
      "surface-high": { "$value": "{palette.gray-50}", "$description": "Fields inside sheets" },
      "surface-highest": { "$value": "{palette.gray-100}", "$description": "Chips, provider monograms" },
      "scrim": { "$value": "{palette.gray-225}" },
      "on-surface": { "$value": "{palette.gray-925}" },
      "on-surface-muted": { "$value": "{palette.gray-600}" },
      "outline": { "$value": "{palette.gray-600}", "$description": "Boundaries of controls: needs 3:1" },
      "outline-variant": { "$value": "{palette.gray-200}", "$description": "Decorative dividers" },
      "accent": { "$value": "{palette.green-700}" },
      "on-accent": { "$value": "{palette.gray-0}" },
      "accent-text": { "$value": "{palette.green-700}" },
      "hero": { "$value": "{palette.gray-925}", "$description": "The cheapest option's card" },
      "on-hero": { "$value": "{palette.gray-75}" },
      "disabled": { "$value": "{palette.gray-100}" },
      "on-disabled": { "$value": "{palette.gray-600}" },
      "error": { "$value": "{palette.red-700}" },
      "on-error": { "$value": "{palette.gray-0}" }
    },
    "dark": {
      "background": { "$value": "{palette.gray-950}" },
      "surface": { "$value": "{palette.gray-875}" },
      "surface-low": { "$value": "{palette.gray-900}" },
      "surface-high": { "$value": "{palette.gray-850}" },
      "surface-highest": { "$value": "{palette.gray-825}" },
      "scrim": { "$value": "{palette.gray-975}" },
      "on-surface": { "$value": "{palette.gray-75}" },
      "on-surface-muted": { "$value": "{palette.gray-400}" },
      "outline": { "$value": "{palette.gray-400}" },
      "outline-variant": { "$value": "{palette.gray-800}" },
      "accent": { "$value": "{palette.green-400}" },
      "on-accent": { "$value": "{palette.green-950}" },
      "accent-text": { "$value": "{palette.green-400}" },
      "hero": { "$value": "{palette.green-400}" },
      "on-hero": { "$value": "{palette.green-950}" },
      "disabled": { "$value": "{palette.gray-850}" },
      "on-disabled": { "$value": "{palette.gray-400}" },
      "error": { "$value": "{palette.red-200}" },
      "on-error": { "$value": "{palette.red-900}" }
    }
  },
  "font": {
    "$type": "fontFamily",
    "text": { "$value": ["Sora", "sans-serif"] },
    "number": { "$value": ["JetBrains Mono", "monospace"], "$description": "Prices, distances, times" }
  },
  "type": {
    "$type": "typography",
    "headline-large": { "$value": { "fontFamily": "{font.text}", "fontSize": "30px", "fontWeight": 700, "lineHeight": 1.1, "letterSpacing": "-0.03em" } },
    "headline-medium": { "$value": { "fontFamily": "{font.text}", "fontSize": "26px", "fontWeight": 700, "lineHeight": 1.15, "letterSpacing": "-0.02em" } },
    "headline-small": { "$value": { "fontFamily": "{font.text}", "fontSize": "22px", "fontWeight": 700, "lineHeight": 1.25, "letterSpacing": "-0.02em" } },
    "title-large": { "$value": { "fontFamily": "{font.text}", "fontSize": "20px", "fontWeight": 700, "lineHeight": 1.2, "letterSpacing": "0" } },
    "title-medium": { "$value": { "fontFamily": "{font.text}", "fontSize": "16px", "fontWeight": 600, "lineHeight": 1.35, "letterSpacing": "0" } },
    "title-small": { "$value": { "fontFamily": "{font.text}", "fontSize": "15px", "fontWeight": 600, "lineHeight": 1.35, "letterSpacing": "0" } },
    "body-large": { "$value": { "fontFamily": "{font.text}", "fontSize": "16px", "fontWeight": 400, "lineHeight": 1.5, "letterSpacing": "0" } },
    "body-medium": { "$value": { "fontFamily": "{font.text}", "fontSize": "14px", "fontWeight": 400, "lineHeight": 1.55, "letterSpacing": "0" } },
    "body-small": { "$value": { "fontFamily": "{font.text}", "fontSize": "13px", "fontWeight": 400, "lineHeight": 1.5, "letterSpacing": "0" } },
    "label-large": { "$value": { "fontFamily": "{font.text}", "fontSize": "16px", "fontWeight": 700, "lineHeight": 1.25, "letterSpacing": "0" } },
    "label-medium": { "$value": { "fontFamily": "{font.text}", "fontSize": "13px", "fontWeight": 600, "lineHeight": 1.3, "letterSpacing": "0" } },
    "label-small": { "$value": { "fontFamily": "{font.text}", "fontSize": "12px", "fontWeight": 600, "lineHeight": 1.3, "letterSpacing": "0.06em" } },
    "number-large": { "$value": { "fontFamily": "{font.number}", "fontSize": "36px", "fontWeight": 700, "lineHeight": 1.0, "letterSpacing": "-0.02em" } },
    "number-medium": { "$value": { "fontFamily": "{font.number}", "fontSize": "18px", "fontWeight": 700, "lineHeight": 1.3, "letterSpacing": "0" } },
    "number-small": { "$value": { "fontFamily": "{font.number}", "fontSize": "12px", "fontWeight": 500, "lineHeight": 1.4, "letterSpacing": "0" } }
  },
  "space": {
    "$type": "dimension",
    "xxs": { "$value": "4px" },
    "xs": { "$value": "8px" },
    "sm": { "$value": "12px" },
    "md": { "$value": "16px" },
    "lg": { "$value": "20px" },
    "xl": { "$value": "24px" },
    "xxl": { "$value": "32px" },
    "xxxl": { "$value": "40px" }
  },
  "radius": {
    "$type": "dimension",
    "sm": { "$value": "10px" },
    "md": { "$value": "14px" },
    "lg": { "$value": "16px" },
    "xl": { "$value": "20px" },
    "xxl": { "$value": "28px" },
    "full": { "$value": "999px" }
  },
  "size": {
    "$type": "dimension",
    "touch-target": { "$value": "48px" },
    "control-md": { "$value": "48px" },
    "control-lg": { "$value": "56px" }
  }
}
```

- [ ] **Step 3: Write the failing tests** — `design/test/model.test.mjs`

```js
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadModel, resolveTokens, toModel, TokenError, MODES } from '../lib/model.mjs';

const TOKENS = fileURLToPath(new URL('../tokens.json', import.meta.url));

const color = (path, value) => ({ path, type: 'color', value });
const minimal = () => [
  color(['theme', 'light', 'background'], '#FFFFFF'),
  color(['theme', 'dark', 'background'], '#000000'),
  { path: ['font', 'text'], type: 'fontFamily', value: ['Sora', 'sans-serif'] },
  { path: ['space', 'md'], type: 'dimension', value: '16px' },
  {
    path: ['type', 'body-large'], type: 'typography',
    value: { fontFamily: ['Sora', 'sans-serif'], fontSize: '16px', fontWeight: 400, lineHeight: 1.5, letterSpacing: '-0.02em' },
  },
];

test('a minimal token set becomes a model', () => {
  const model = toModel(minimal());
  assert.deepEqual(model.themes, { light: { background: '#FFFFFF' }, dark: { background: '#000000' } });
  assert.deepEqual(model.space, { md: 16 });
  assert.deepEqual(model.fonts, { text: ['Sora', 'sans-serif'] });
  assert.deepEqual(model.type['body-large'], { font: 'text', size: 16, weight: 400, lineHeight: 1.5, letterSpacing: -0.02 });
});

test('colours must be upper-case #RRGGBB', () => {
  for (const bad of ['#ffffff', '#FFF', '#FFFFFF80', 'white', 'rgb(0,0,0)']) {
    const tokens = minimal();
    tokens[0] = color(['theme', 'light', 'background'], bad);
    assert.throws(() => toModel(tokens), (e) => e instanceof TokenError && e.message.includes('theme.light.background'), bad);
  }
});

test('a role present in only one theme names the role', () => {
  const tokens = [...minimal(), color(['theme', 'dark', 'accent'], '#3DDC97')];
  assert.throws(() => toModel(tokens), /only dark \[accent\]/);
});

test('unknown groups and non-kebab-case names are rejected', () => {
  assert.throws(() => toModel([...minimal(), color(['colour', 'x'], '#000000')]), /unknown group "colour"/);
  assert.throws(() => toModel([...minimal(), color(['theme', 'light', 'onSurface'], '#000000')]), /onSurface/);
  assert.throws(() => toModel([...minimal(), color(['theme', 'sepia', 'background'], '#000000')]), /theme\.<light\|dark>\.<role>/);
});

test('dimensions need px and typography must use a declared font', () => {
  const noUnit = minimal();
  noUnit[3] = { path: ['space', 'md'], type: 'dimension', value: '16' };
  assert.throws(() => toModel(noUnit), /space\.md/);
  const strayFont = minimal();
  strayFont[4] = { ...strayFont[4], value: { ...strayFont[4].value, fontFamily: ['Arial', 'sans-serif'] } };
  assert.throws(() => toModel(strayFont), /type\.body-large.*not a declared font/);
  const badWeight = minimal();
  badWeight[4] = { ...badWeight[4], value: { ...badWeight[4].value, fontWeight: 450 } };
  assert.throws(() => toModel(badWeight), /fontWeight/);
});

test('a broken reference fails while resolving', async () => {
  const dir = await mkdtemp(join(tmpdir(), 'tokens-'));
  const file = join(dir, 'tokens.json');
  await writeFile(file, JSON.stringify({ brand: { $type: 'color', mark: { $value: '{palette.missing}' } } }));
  await assert.rejects(resolveTokens(file));
});

test('the real tokens load, with the same roles in both themes', async () => {
  const model = await loadModel(TOKENS);
  const roles = MODES.map((m) => Object.keys(model.themes[m]).sort());
  assert.deepEqual(roles[0], roles[1]);
  assert.equal(model.themes.dark.accent, '#3DDC97');
  assert.equal(model.themes.light.accent, '#0B7A50');
  assert.equal(model.brand.mark, '#3DDC97');
  assert.equal(model.type['number-large'].font, 'number');
});
```

- [ ] **Step 4: Run them to see them fail**

Run: `cd design && npm test`
Expected: FAIL, `Cannot find module '…/lib/model.mjs'`.

- [ ] **Step 5: Implement** — `design/lib/model.mjs`

```js
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
```

- [ ] **Step 6: Run the tests**

Run: `cd design && npm test`
Expected: all 7 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add .gitignore design/package.json design/package-lock.json design/.nvmrc design/tokens.json design/lib/model.mjs design/test/model.test.mjs
git commit -m "feat(design): add design tokens from the canvas and their validated model

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Contrast check

**Files:**
- Create: `design/contrast.json`, `design/lib/contrast.mjs`, `design/contrast.mjs`,
  `design/test/contrast.test.mjs`
- Modify: `design/package.json` (nothing to add: `contrast` script exists)

**Interfaces:**
- Consumes: `loadModel`, `TokenError`, `MODES` from Task 1.
- Produces: `relativeLuminance(hex): number`, `contrastRatio(a, b): number`,
  `floor2(n): string`, `checkContrast(model, rules): Result[]` where
  `Result = { scope: 'light'|'dark'|'brand', fg, bg, kind: 'text'|'non-text', ratio: number, min: number, ok: boolean }`.

- [ ] **Step 1: Write `design/contrast.json`**

```json
{
  "$description": "Colour pairs the UI uses, checked in both themes against WCAG 2.2 AA (1.4.3 text, 1.4.11 non-text). Add a pair whenever a screen puts a new foreground on a new background.",
  "minimum": { "text": 4.5, "non-text": 3 },
  "pairs": [
    { "fg": "on-surface", "bg": "background", "kind": "text" },
    { "fg": "on-surface", "bg": "surface", "kind": "text" },
    { "fg": "on-surface", "bg": "surface-low", "kind": "text" },
    { "fg": "on-surface", "bg": "surface-high", "kind": "text" },
    { "fg": "on-surface", "bg": "surface-highest", "kind": "text" },
    { "fg": "on-surface-muted", "bg": "background", "kind": "text" },
    { "fg": "on-surface-muted", "bg": "surface", "kind": "text" },
    { "fg": "on-surface-muted", "bg": "surface-low", "kind": "text" },
    { "fg": "on-surface-muted", "bg": "surface-high", "kind": "text" },
    { "fg": "on-surface-muted", "bg": "surface-highest", "kind": "text" },
    { "fg": "on-accent", "bg": "accent", "kind": "text" },
    { "fg": "accent-text", "bg": "background", "kind": "text" },
    { "fg": "accent-text", "bg": "surface", "kind": "text" },
    { "fg": "accent-text", "bg": "surface-low", "kind": "text" },
    { "fg": "on-hero", "bg": "hero", "kind": "text" },
    { "fg": "on-disabled", "bg": "disabled", "kind": "text" },
    { "fg": "error", "bg": "background", "kind": "text" },
    { "fg": "error", "bg": "surface", "kind": "text" },
    { "fg": "error", "bg": "surface-low", "kind": "text" },
    { "fg": "on-error", "bg": "error", "kind": "text" },
    { "fg": "outline", "bg": "background", "kind": "non-text" },
    { "fg": "outline", "bg": "surface", "kind": "non-text" },
    { "fg": "outline", "bg": "surface-low", "kind": "non-text" },
    { "fg": "outline", "bg": "surface-high", "kind": "non-text" },
    { "fg": "accent", "bg": "background", "kind": "non-text" },
    { "fg": "accent", "bg": "surface", "kind": "non-text" },
    { "fg": "accent", "bg": "surface-low", "kind": "non-text" }
  ],
  "brand": [
    { "fg": "on-mark", "bg": "mark", "kind": "non-text" }
  ]
}
```

`accent-text` on `surface-highest` is deliberately absent: in light mode it is 4.38:1. Chips
carry `on-surface` text, as in the canvas.

- [ ] **Step 2: Write the failing tests** — `design/test/contrast.test.mjs`

```js
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { checkContrast, contrastRatio, floor2, relativeLuminance } from '../lib/contrast.mjs';
import { loadModel, TokenError } from '../lib/model.mjs';

const here = (p) => fileURLToPath(new URL(p, import.meta.url));

test('luminance of white and black', () => {
  assert.equal(relativeLuminance('#FFFFFF'), 1);
  assert.equal(relativeLuminance('#000000'), 0);
});

test('black on white is 21:1, in either order', () => {
  assert.equal(contrastRatio('#000000', '#FFFFFF'), 21);
  assert.equal(contrastRatio('#FFFFFF', '#000000'), 21);
});

test('#777777 on white fails 4.5 although it rounds to 4.5', () => {
  const ratio = contrastRatio('#777777', '#FFFFFF');
  assert.ok(ratio > 4.47 && ratio < 4.49, String(ratio));
  const model = { themes: { light: { fg: '#777777', bg: '#FFFFFF' }, dark: { fg: '#000000', bg: '#FFFFFF' } }, brand: {} };
  const rules = { minimum: { text: 4.5, 'non-text': 3 }, pairs: [{ fg: 'fg', bg: 'bg', kind: 'text' }], brand: [] };
  const [light, dark] = checkContrast(model, rules);
  assert.equal(light.ok, false);
  assert.equal(dark.ok, true);
  assert.equal(floor2(light.ratio), '4.47');
});

test('a rule naming a missing role or an unknown kind stops the check', () => {
  const model = { themes: { light: { a: '#000000' }, dark: { a: '#000000' } }, brand: { mark: '#000000' } };
  const typo = { minimum: { text: 4.5, 'non-text': 3 }, pairs: [{ fg: 'a', bg: 'backgroud', kind: 'text' }], brand: [] };
  assert.throws(() => checkContrast(model, typo), (e) => e instanceof TokenError && /backgroud/.test(e.message));
  const kind = { minimum: { text: 4.5, 'non-text': 3 }, pairs: [{ fg: 'a', bg: 'a', kind: 'large-text' }], brand: [] };
  assert.throws(() => checkContrast(model, kind), /large-text/);
  const brand = { minimum: { text: 4.5, 'non-text': 3 }, pairs: [], brand: [{ fg: 'mark', bg: 'nope', kind: 'non-text' }] };
  assert.throws(() => checkContrast(model, brand), /nope/);
});

test('every pair of the real tokens passes', async () => {
  const model = await loadModel(here('../tokens.json'));
  const rules = JSON.parse(await readFile(here('../contrast.json'), 'utf8'));
  const results = checkContrast(model, rules);
  assert.equal(results.length, rules.pairs.length * 2 + rules.brand.length);
  assert.deepEqual(results.filter((r) => !r.ok), []);
});
```

- [ ] **Step 3: Run them to see them fail**

Run: `cd design && npm test`
Expected: FAIL, `Cannot find module '…/lib/contrast.mjs'`.

- [ ] **Step 4: Implement** — `design/lib/contrast.mjs`

```js
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
```

`design/contrast.mjs`:

```js
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { checkContrast, floor2 } from './lib/contrast.mjs';
import { loadModel } from './lib/model.mjs';

const here = (p) => fileURLToPath(new URL(p, import.meta.url));
const model = await loadModel(here('tokens.json'));
const rules = JSON.parse(await readFile(here('contrast.json'), 'utf8'));
const results = checkContrast(model, rules);
for (const r of results) {
  const line = `${r.ok ? 'ok  ' : 'FAIL'} ${r.scope.padEnd(5)} ${r.fg.padEnd(17)} on ${r.bg.padEnd(16)} ${floor2(r.ratio).padStart(5)}:1 (needs ${r.min}, ${r.kind})`;
  console.log(line);
}
const failed = results.filter((r) => !r.ok).length;
if (failed) {
  console.error(`${failed} colour pair(s) below WCAG 2.2 AA`);
  process.exitCode = 1;
}
```

- [ ] **Step 5: Run the tests and the check**

Run: `cd design && npm test && npm run contrast`
Expected: all tests PASS; 55 lines (27 pairs × 2 themes + 1 brand pair), every one starting
with `ok`; the lowest text ratio is `accent-text on background 4.94:1` (light); exit 0.

- [ ] **Step 6: Commit**

```bash
git add design/contrast.json design/lib/contrast.mjs design/contrast.mjs design/test/contrast.test.mjs
git commit -m "feat(design): check every colour pair against WCAG 2.2 AA

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Kotlin and CSS generators

**Files:**
- Create: `design/lib/formats.mjs`, `design/test/formats.test.mjs`

**Interfaces:**
- Consumes: `Model` from Task 1.
- Produces: `kotlin(model): string`; `css(model, fontFaces): string` where
  `fontFaces = [{ family: string, url: string }]`; `camel(name): string`
  (`'on-surface-muted'` → `'onSurfaceMuted'`). Kotlin names consumed by Task 5:
  `WhichrideTokens.LightColors`, `WhichrideTokens.DarkColors` (type `WhichrideColors`),
  `WhichrideTokens.BrandMark`, `WhichrideTokens.BrandOnMark` (`Color`),
  `WhichrideTokens.Space.<name>`, `.Radius.<name>`, `.Size.<camelName>` (`Dp`),
  `WhichrideTokens.Type.<camelName>` (`TypeToken`, internal), `WhichrideTokens.Type.weights`
  (`List<Int>`, every weight the type tokens use, ascending). CSS custom properties:
  `--wr-color-<role>`, `--wr-brand-<name>`, `--wr-space-<name>` (rem), `--wr-radius-<name>` (px),
  `--wr-size-<name>` (px), `--wr-font-<role>`, `--wr-type-<role>` (font shorthand, rem) and
  `--wr-type-<role>-tracking` (em).

- [ ] **Step 1: Write the failing tests** — `design/test/formats.test.mjs`

```js
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { camel, css, kotlin } from '../lib/formats.mjs';

const model = {
  themes: {
    light: { background: '#F3F6F8', 'on-surface-muted': '#4D5966' },
    dark: { background: '#0B0F14', 'on-surface-muted': '#9AA7B6' },
  },
  brand: { mark: '#3DDC97', 'on-mark': '#04261A' },
  space: { md: 16, lg: 20 },
  radius: { md: 14 },
  size: { 'touch-target': 48 },
  fonts: { text: ['Sora', 'sans-serif'], number: ['JetBrains Mono', 'monospace'] },
  type: {
    'body-large': { font: 'text', size: 16, weight: 400, lineHeight: 1.5, letterSpacing: 0 },
    'number-large': { font: 'number', size: 36, weight: 700, lineHeight: 1, letterSpacing: -0.02 },
  },
};
const faces = [{ family: 'Sora', url: '/fonts/Sora-Variable.ttf' }, { family: 'JetBrains Mono', url: '/fonts/JetBrainsMono-Variable.ttf' }];

test('camel case', () => {
  assert.equal(camel('on-surface-muted'), 'onSurfaceMuted');
  assert.equal(camel('xxl'), 'xxl');
});

test('Kotlin: colours become WhichrideColors constructors with ARGB literals', () => {
  const out = kotlin(model);
  assert.match(out, /^\/\/ Generated from design\/tokens\.json by design\/build\.mjs\. Do not edit\.\n/);
  assert.match(out, /package io\.github\.locadani\.whichride\.designsystem\n/);
  assert.match(out, /val LightColors: WhichrideColors = WhichrideColors\(\n        background = Color\(0xFFF3F6F8\),\n        onSurfaceMuted = Color\(0xFF4D5966\),\n    \)/);
  assert.match(out, /val DarkColors: WhichrideColors = WhichrideColors\(\n        background = Color\(0xFF0B0F14\),/);
  assert.match(out, /val BrandOnMark: Color = Color\(0xFF04261A\)/);
});

test('Kotlin: sizes are dp, type sizes are plain sp numbers', () => {
  const out = kotlin(model);
  assert.match(out, /object Space \{\n        val lg: Dp = 20\.dp\n        val md: Dp = 16\.dp\n    \}/);
  assert.match(out, /object Size \{\n        val touchTarget: Dp = 48\.dp\n    \}/);
  assert.match(out, /internal object Type \{\n        val weights: List<Int> = listOf\(400, 700\)\n/);
  assert.match(out, /val numberLarge: TypeToken = TypeToken\(FontRole\.NUMBER, sizeSp = 36f, weight = 700, lineHeight = 1f, letterSpacingEm = -0\.02f\)/);
  assert.match(out, /val bodyLarge: TypeToken = TypeToken\(FontRole\.TEXT, sizeSp = 16f, weight = 400, lineHeight = 1\.5f, letterSpacingEm = 0f\)/);
});

test('CSS: light by default, dark by system setting unless forced light, or forced dark', () => {
  const out = css(model, faces);
  assert.match(out, /^\/\* Generated from design\/tokens\.json by design\/build\.mjs\. Do not edit\. \*\/\n/);
  assert.match(out, /:root \{\n  color-scheme: light dark;\n[\s\S]*  --wr-color-background: #F3F6F8;\n/);
  assert.match(out, /@media \(prefers-color-scheme: dark\) \{\n  :root:not\(\[data-theme="light"\]\) \{\n    color-scheme: dark;\n    --wr-color-background: #0B0F14;\n/);
  assert.match(out, /:root\[data-theme="dark"\] \{\n  color-scheme: dark;\n  --wr-color-background: #0B0F14;\n/);
  assert.match(out, /:root\[data-theme="light"\] \{\n  color-scheme: light;\n\}/);
});

test('CSS: text and spacing scale with the user font size (rem), radii stay px', () => {
  const out = css(model, faces);
  assert.match(out, /--wr-space-lg: 1\.25rem;/);
  assert.match(out, /--wr-radius-md: 14px;/);
  assert.match(out, /--wr-type-body-large: 400 1rem\/1\.5 var\(--wr-font-text\);/);
  assert.match(out, /--wr-type-number-large: 700 2\.25rem\/1 var\(--wr-font-number\);/);
  assert.match(out, /--wr-type-number-large-tracking: -0\.02em;/);
  assert.doesNotMatch(out, /--wr-type-[a-z-]+: \d+ \d+px/);
});

test('CSS: self-hosted font faces; generic families are not quoted', () => {
  const out = css(model, faces);
  assert.match(out, /@font-face \{\n  font-family: "Sora";\n  src: url\("\/fonts\/Sora-Variable\.ttf"\) format\("truetype"\);\n  font-weight: 100 800;\n  font-display: swap;\n\}/);
  assert.match(out, /--wr-font-number: "JetBrains Mono", monospace;/);
  assert.doesNotMatch(out, /fonts\.googleapis|fonts\.gstatic/);
});

test('output does not depend on key order', () => {
  const shuffled = structuredClone(model);
  shuffled.space = { lg: 20, md: 16 };
  assert.equal(kotlin(shuffled), kotlin(model));
  assert.equal(css(shuffled, faces), css(model, faces));
});
```

- [ ] **Step 2: Run them to see them fail**

Run: `cd design && npm test`
Expected: FAIL, `Cannot find module '…/lib/formats.mjs'`.

- [ ] **Step 3: Implement** — `design/lib/formats.mjs`

```js
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
```

- [ ] **Step 4: Run the tests**

Run: `cd design && npm test`
Expected: all tests PASS.

- [ ] **Step 5: Commit**

```bash
git add design/lib/formats.mjs design/test/formats.test.mjs
git commit -m "feat(design): generate Kotlin and CSS from the token model

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Logo, fonts and the build

**Files:**
- Create: `design/logo.svg`, `design/fonts/Sora-Variable.ttf`, `design/fonts/Sora-OFL.txt`,
  `design/fonts/JetBrainsMono-Variable.ttf`, `design/fonts/JetBrainsMono-OFL.txt`,
  `design/lib/logo.mjs`, `design/build.mjs`, `design/test/logo.test.mjs`,
  `design/test/build.test.mjs`, `design/README.md`
- Generated (committed): every file under `android/designsystem/src/main/` listed in the file
  map except the hand-written Kotlin, and everything under `web/`.

**Interfaces:**
- Consumes: `loadModel` (Task 1), `kotlin`, `css` (Task 3).
- Produces: `parseLogo(svgText, brand: Record<name, hex>): Logo` where
  `Logo = { size: number, tile: { rx: number, fill: hex }, mark: { d: string, stroke: hex, strokeWidth: number } }`
  (colours resolved from `{brand.<name>}` references); `resolvedSvg(svgText, brand): string`
  (the same SVG with references replaced by hex values);
  `roundedRectPath(size, rx): string`; `logoOutputs(logo): Record<relativePath, string>`
  (paths relative to `android/designsystem/src/main/`); `build(root: string): Promise<string[]>`
  (writes everything under `root`, returns the written paths relative to `root`, sorted).
  Android resource names consumed by Task 5 and milestone 5: `R.font.sora`,
  `R.font.jetbrains_mono`, `R.drawable.whichride_logo`, `R.mipmap.ic_launcher`.

- [ ] **Step 1: Add the fonts** (Google Fonts repository at a pinned commit; both are SIL OFL 1.1)

```bash
mkdir -p design/fonts
G=https://raw.githubusercontent.com/google/fonts/47d54ca1ebb14fd864695f1d39dc734f39017c33/ofl
curl -fsSL -o design/fonts/Sora-Variable.ttf "$G/sora/Sora%5Bwght%5D.ttf"
curl -fsSL -o design/fonts/Sora-OFL.txt "$G/sora/OFL.txt"
curl -fsSL -o design/fonts/JetBrainsMono-Variable.ttf "$G/jetbrainsmono/JetBrainsMono%5Bwght%5D.ttf"
curl -fsSL -o design/fonts/JetBrainsMono-OFL.txt "$G/jetbrainsmono/OFL.txt"
shasum -a 256 design/fonts/*
```

Expected (stop if any hash differs):

```
48715a42ec242c21e9f02692891e147d022299a52e48d5e413e1a942193ffeda  design/fonts/JetBrainsMono-Variable.ttf
b2fe5e8987594e9ffd1d2ca52a2f5d73eb8335243893c5d6254b5ad69269591d  design/fonts/JetBrainsMono-OFL.txt
ba0b9729c9428ba79a0459ab8ec575791b51509dbec213e383d0316d37fec299  design/fonts/Sora-OFL.txt
84ff7096ae3ec6c8be47d906d1a0ba4de7f2ce78c615275c77301964a316e16c  design/fonts/Sora-Variable.ttf
```

- [ ] **Step 2: Write `design/logo.svg`**

The canvas mark (24-unit box drawn at 18 px, centred in a 32 px tile) converted to tile
coordinates: ×0.75, +7. Its geometry (stroke width, corner radius) is the logo's shape, not a
design token.

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
  <rect width="32" height="32" rx="10" fill="{brand.mark}"/>
  <path d="M16 22.75V16.75M16 16.75L11.5 10M16 16.75L20.5 10" fill="none" stroke="{brand.on-mark}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
```

The colours are references into `tokens.json`, not values: the logo's colours are changed there
only. The build writes a plain SVG with the resolved colours as `web/public/favicon.svg`.

- [ ] **Step 3: Write the failing logo tests** — `design/test/logo.test.mjs`

```js
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { logoOutputs, parseLogo, resolvedSvg, roundedRectPath } from '../lib/logo.mjs';
import { loadModel, TokenError } from '../lib/model.mjs';

const here = (p) => fileURLToPath(new URL(p, import.meta.url));
const svg = (inner, viewBox = '0 0 32 32') => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${viewBox}">${inner}</svg>`;
const BRAND = { mark: '#3DDC97', 'on-mark': '#04261A' };
const RECT = '<rect width="32" height="32" rx="10" fill="{brand.mark}"/>';
const PATH = '<path d="M16 22.75V16.75" fill="none" stroke="{brand.on-mark}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>';

test('a valid logo parses, with colours resolved from the brand tokens', () => {
  assert.deepEqual(parseLogo(svg(RECT + PATH), BRAND), {
    size: 32,
    tile: { rx: 10, fill: '#3DDC97' },
    mark: { d: 'M16 22.75V16.75', stroke: '#04261A', strokeWidth: 1.8 },
  });
});

test('anything outside the constrained format is rejected', () => {
  const cases = {
    'extra element': svg(RECT + PATH + '<circle r="2"/>'),
    'two paths': svg(RECT + PATH + PATH),
    'no rect': svg(PATH),
    'extra attribute': svg(RECT.replace('/>', ' transform="rotate(3)"/>') + PATH),
    'non-square': svg(RECT + PATH, '0 0 32 24'),
    'rect smaller than the box': svg(RECT.replace('width="32"', 'width="30"') + PATH),
    'literal tile colour': svg(RECT.replace('{brand.mark}', '#3DDC97') + PATH),
    'literal mark colour': svg(RECT + PATH.replace('{brand.on-mark}', '#04261A')),
    'unknown brand token': svg(RECT.replace('{brand.mark}', '{brand.markk}') + PATH),
    'theme role instead of brand': svg(RECT.replace('{brand.mark}', '{theme.dark.accent}') + PATH),
    'filled path': svg(RECT + PATH.replace('fill="none"', 'fill="{brand.mark}"')),
  };
  for (const [name, text] of Object.entries(cases)) {
    assert.throws(() => parseLogo(text, BRAND), TokenError, name);
  }
});

test('the resolved SVG has hex colours and no references', () => {
  const out = resolvedSvg(svg(RECT + PATH), BRAND);
  assert.match(out, /fill="#3DDC97"/);
  assert.match(out, /stroke="#04261A"/);
  assert.doesNotMatch(out, /\{brand\./);
});

test('rounded rectangle as a vector path', () => {
  assert.equal(
    roundedRectPath(32, 10),
    'M10,0H22A10,10 0 0,1 32,10V22A10,10 0 0,1 22,32H10A10,10 0 0,1 0,22V10A10,10 0 0,1 10,0Z',
  );
});

test('adaptive icon: the tile fills the 72dp visible area of the 108dp canvas', () => {
  const out = logoOutputs(parseLogo(svg(RECT + PATH), BRAND));
  const fg = out['res/drawable/ic_launcher_foreground.xml'];
  assert.match(fg, /android:viewportWidth="108"/);
  assert.match(fg, /<group android:scaleX="2\.25" android:scaleY="2\.25" android:translateX="18" android:translateY="18">/);
  assert.match(fg, /android:strokeColor="#04261A"/);
  assert.match(out['res/values/ic_launcher_background.xml'], /<color name="ic_launcher_background">#3DDC97<\/color>/);
  assert.match(out['res/mipmap-anydpi/ic_launcher.xml'], /<monochrome android:drawable="@drawable\/ic_launcher_foreground"\/>/);
  assert.match(out['res/drawable/whichride_logo.xml'], /android:fillColor="#3DDC97"/);
  for (const text of Object.values(out)) assert.match(text, /Generated from design\/logo\.svg/);
});

test('the real logo takes its colours from tokens.json', async () => {
  const model = await loadModel(here('../tokens.json'));
  const logo = parseLogo(await readFile(here('../logo.svg'), 'utf8'), model.brand);
  assert.equal(logo.tile.fill, model.brand.mark);
  assert.equal(logo.mark.stroke, model.brand['on-mark']);
});
```

- [ ] **Step 4: Run them to see them fail**

Run: `cd design && npm test`
Expected: FAIL, `Cannot find module '…/lib/logo.mjs'`.

- [ ] **Step 5: Implement** — `design/lib/logo.mjs`

```js
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
 * <rect> (the tile) and one stroked <path> (the mark), coloured by brand token references.
 * Anything else is an error, so the Android icons can never silently lose part of the logo.
 */
export function parseLogo(text, brand) {
  const parser = new XMLParser({ ignoreAttributes: false, attributeNamePrefix: '@_', isArray: (_name, _path, _leaf, isAttribute) => !isAttribute });
  const doc = parser.parse(text);
  const roots = Object.keys(doc);
  if (roots.length !== 1 || roots[0] !== 'svg' || doc.svg.length !== 1) fail('the file must contain exactly one <svg> element');
  const root = doc.svg[0];
  const attrs = (node, allowed, where) => {
    const names = Object.keys(node).filter((k) => k.startsWith('@_')).map((k) => k.slice(2));
    for (const n of names) if (!allowed.includes(n)) fail(`<${where}> may not have "${n}"`);
    for (const n of allowed) if (!names.includes(n)) fail(`<${where}> needs "${n}"`);
    return Object.fromEntries(names.map((n) => [n, node[`@_${n}`]]));
  };
  const elements = Object.keys(root).filter((k) => !k.startsWith('@_'));
  for (const e of elements) if (!['rect', 'path'].includes(e)) fail(`<${e}> is not allowed`);
  if (root.rect?.length !== 1 || root.path?.length !== 1) fail('exactly one <rect> and one <path> are allowed');

  const svg = attrs(root, ['xmlns', 'viewBox'], 'svg');
  const box = svg.viewBox.split(' ');
  if (box.length !== 4 || box[0] !== '0' || box[1] !== '0' || box[2] !== box[3] || !NUMBER.test(box[2])) fail('viewBox must be "0 0 N N"');
  const size = Number(box[2]);

  const rect = attrs(root.rect[0], ['width', 'height', 'rx', 'fill'], 'rect');
  if (Number(rect.width) !== size || Number(rect.height) !== size) fail('the <rect> must fill the viewBox');
  if (!NUMBER.test(rect.rx) || Number(rect.rx) * 2 > size) fail('rx must be a number no larger than half the size');
  const tileFill = brandColour(rect.fill, brand, 'tile');

  const path = attrs(root.path[0], ['d', 'fill', 'stroke', 'stroke-width', 'stroke-linecap', 'stroke-linejoin'], 'path');
  if (path.fill !== 'none') fail('the mark is a stroke: fill must be "none"');
  const markStroke = brandColour(path.stroke, brand, 'mark');
  if (!NUMBER.test(path['stroke-width'])) fail('stroke-width must be a number');
  if (path['stroke-linecap'] !== 'round' || path['stroke-linejoin'] !== 'round') fail('the mark uses round caps and joins');

  return {
    size,
    tile: { rx: Number(rect.rx), fill: tileFill },
    mark: { d: path.d, stroke: markStroke, strokeWidth: Number(path['stroke-width']) },
  };
}

/** A rounded square as VectorDrawable path data (VectorDrawable has no <rect>). */
export function roundedRectPath(n, r) {
  return `M${r},0H${n - r}A${r},${r} 0 0,1 ${n},${r}V${n - r}A${r},${r} 0 0,1 ${n - r},${n}` +
    `H${r}A${r},${r} 0 0,1 0,${n - r}V${r}A${r},${r} 0 0,1 ${r},0Z`;
}

const markPath = (logo, indent) =>
  `${indent}<path android:pathData="${logo.mark.d}" android:strokeColor="${logo.mark.stroke}" ` +
  `android:strokeWidth="${logo.mark.strokeWidth}" android:strokeLineCap="round" android:strokeLineJoin="round"/>`;

/** Android resources, keyed by path relative to the module's src/main/. */
export function logoOutputs(logo) {
  const n = logo.size;
  const scale = VISIBLE / n;
  const offset = (ICON - VISIBLE) / 2;
  const xml = (body) => `<?xml version="1.0" encoding="utf-8"?>\n${HEADER}\n${body}\n`;
  return {
    'res/drawable/whichride_logo.xml': xml(
      `<vector ${ANDROID_NS}\n    android:width="${n}dp" android:height="${n}dp"\n` +
        `    android:viewportWidth="${n}" android:viewportHeight="${n}">\n` +
        `    <path android:fillColor="${logo.tile.fill}" android:pathData="${roundedRectPath(n, logo.tile.rx)}"/>\n` +
        `${markPath(logo, '    ')}\n</vector>`,
    ),
    'res/drawable/ic_launcher_foreground.xml': xml(
      `<vector ${ANDROID_NS}\n    android:width="${ICON}dp" android:height="${ICON}dp"\n` +
        `    android:viewportWidth="${ICON}" android:viewportHeight="${ICON}">\n` +
        `    <group android:scaleX="${scale}" android:scaleY="${scale}" android:translateX="${offset}" android:translateY="${offset}">\n` +
        `${markPath(logo, '        ')}\n    </group>\n</vector>`,
    ),
    'res/values/ic_launcher_background.xml': xml(
      `<resources>\n    <color name="ic_launcher_background">${logo.tile.fill}</color>\n</resources>`,
    ),
    'res/mipmap-anydpi/ic_launcher.xml': xml(
      `<adaptive-icon ${ANDROID_NS}>\n` +
        '    <background android:drawable="@color/ic_launcher_background"/>\n' +
        '    <foreground android:drawable="@drawable/ic_launcher_foreground"/>\n' +
        '    <monochrome android:drawable="@drawable/ic_launcher_foreground"/>\n' +
        '</adaptive-icon>',
    ),
  };
}
```

- [ ] **Step 6: Run the logo tests**

Run: `cd design && npm test`
Expected: all tests PASS.

- [ ] **Step 7: Write the failing build test** — `design/test/build.test.mjs`

```js
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
```

- [ ] **Step 8: Implement** — `design/build.mjs`

```js
import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { css, kotlin } from './lib/formats.mjs';
import { logoOutputs, parseLogo, resolvedSvg } from './lib/logo.mjs';
import { loadModel, TokenError } from './lib/model.mjs';

const DESIGN = dirname(fileURLToPath(import.meta.url));
const ANDROID_MAIN = 'android/designsystem/src/main';
const KOTLIN_FILE = `${ANDROID_MAIN}/kotlin/io/github/locadani/whichride/designsystem/WhichrideTokens.kt`;

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

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const written = await build(join(DESIGN, '..'));
  console.log(`wrote ${written.length} files:\n  ${written.join('\n  ')}`);
}
```

- [ ] **Step 9: Run the tests, then build for real**

Run: `cd design && npm test && npm run build`
Expected: all tests PASS; `wrote 13 files:` followed by the list from the build test.

Then inspect: `sed -n 1,30p android/designsystem/src/main/kotlin/io/github/locadani/whichride/designsystem/WhichrideTokens.kt`
Expected: `LightColors` with 19 roles starting `accent = Color(0xFF0B7A50),`.

- [ ] **Step 10: Write `design/README.md`**

````markdown
# Design tokens

The only place whichride's colours, fonts, sizes and logo are edited (spec §10.5). Everything
the Android app and the website use is generated from here and committed; CI fails if a
generated file is stale or a colour pair misses WCAG 2.2 AA.

| Source | What it holds |
|---|---|
| `tokens.json` | Palette, brand colours, light and dark theme roles, fonts, type scale, spacing, radii, sizes (W3C design token format) |
| `contrast.json` | Colour pairs the UI uses; each must meet 4.5:1 (text) or 3:1 (control boundaries) in both themes |
| `logo.svg` | The logo's shape: one `<rect>` tile and one stroked `<path>` mark, coloured by `{brand.*}` references to `tokens.json` |
| `fonts/` | Sora and JetBrains Mono (SIL Open Font License 1.1, licences next to them) |

```sh
npm ci              # once
npm test            # generators and checks
npm run contrast    # prints every pair and its ratio
npm run build       # rewrites android/designsystem/… and web/…; commit the result
```

**Changing a colour:** edit `tokens.json`, run `npm run contrast` and `npm run build`, commit
the source and the generated files together. **New foreground/background pair in a screen:**
add it to `contrast.json` first. **Changing the logo's colours:** change `brand` in
`tokens.json`; `logo.svg` only references them.

Hand-written UI code never contains a colour, `dp` or `sp` literal: it reads `WhichrideTheme`
and `WhichrideTokens` (Android) or the `--wr-*` custom properties (web). `test/single-source.test.mjs`
enforces it.

The design comes from the claude.ai Design canvas
(https://claude.ai/artifact/P2A5aUCR45ZBKhnRRqCW2b); where the tokens depart from it,
`docs/DECISIONS.md` rows 51–57 say why.
````

- [ ] **Step 11: Commit** (sources, generators and generated files together)

```bash
git add design/logo.svg design/fonts design/lib/logo.mjs design/build.mjs design/test/logo.test.mjs design/test/build.test.mjs design/README.md android/designsystem/src/main web
git status --short   # only the files above; nothing under node_modules
git commit -m "feat(design): generate the Android icon, fonts, Kotlin tokens and tokens.css

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Android project and `WhichrideTheme`

**Files:**
- Create: `android/settings.gradle.kts`, `android/build.gradle.kts`, `android/gradle.properties`,
  `android/gradle/libs.versions.toml`, Gradle wrapper files,
  `android/designsystem/build.gradle.kts`,
  `android/designsystem/src/main/kotlin/io/github/locadani/whichride/designsystem/{WhichrideColors,TypeToken,Type,Shapes,MaterialColors,WhichrideTheme,ThemePreview}.kt`,
  `android/designsystem/src/test/kotlin/io/github/locadani/whichride/designsystem/ThemeTest.kt`

**Interfaces:**
- Consumes: the generated `WhichrideTokens` (Task 4) and the resources `R.font.sora`,
  `R.font.jetbrains_mono`.
- Produces (public API for milestone 5): `@Composable fun WhichrideTheme(darkTheme: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit)`;
  `object WhichrideTheme { val colors: WhichrideColors; val numbers: NumberStyles }`;
  `data class WhichrideColors(…19 roles…)`; `data class NumberStyles(large, medium, small: TextStyle)`;
  `WhichrideTokens.Space/Radius/Size`. Internal: `TypeToken`, `FontRole`,
  `TypeToken.toTextStyle()`, `WhichrideTypography`, `WhichrideNumberStyles`, `WhichrideShapes`,
  `WhichrideColors.toMaterial(inversePrimary: Color): ColorScheme`.

- [ ] **Step 1: Gradle project**

`android/settings.gradle.kts`:

```kotlin
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}
rootProject.name = "whichride"
include(":designsystem")
```

`android/gradle/libs.versions.toml`:

```toml
[versions]
agp = "9.4.1"
kotlin = "2.4.21"
compose-bom = "2026.09.00"
junit = "4.13.2"

[libraries]
compose-bom = { group = "androidx.compose", name = "compose-bom", version.ref = "compose-bom" }
compose-material3 = { group = "androidx.compose.material3", name = "material3" }
compose-ui-tooling = { group = "androidx.compose.ui", name = "ui-tooling" }
compose-ui-tooling-preview = { group = "androidx.compose.ui", name = "ui-tooling-preview" }
junit = { group = "junit", name = "junit", version.ref = "junit" }

[plugins]
android-library = { id = "com.android.library", version.ref = "agp" }
kotlin-compose = { id = "org.jetbrains.kotlin.plugin.compose", version.ref = "kotlin" }
```

`android/build.gradle.kts`:

```kotlin
plugins {
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.kotlin.compose) apply false
}
```

`android/gradle.properties`:

```properties
org.gradle.jvmargs=-Xmx2g
android.useAndroidX=true
```

`android/designsystem/build.gradle.kts`:

```kotlin
plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.compose)
}

android {
    namespace = "io.github.locadani.whichride.designsystem"
    compileSdk = 37
    defaultConfig {
        minSdk = 26
    }
    buildFeatures {
        compose = true
    }
}

dependencies {
    implementation(platform(libs.compose.bom))
    api(libs.compose.material3)
    implementation(libs.compose.ui.tooling.preview)
    debugImplementation(libs.compose.ui.tooling)
    testImplementation(libs.junit)
}
```

Wrapper and SDK location, copied from the spike (already Gradle 9.8.1 and the owner's SDK
path), then pinned to the distribution's checksum:

```bash
cp spikes/uniffi-android/android/local.properties android/local.properties
cp spikes/uniffi-android/android/gradlew spikes/uniffi-android/android/gradlew.bat android/
mkdir -p android/gradle && cp -R spikes/uniffi-android/android/gradle/wrapper android/gradle/
cd android
export JAVA_HOME=/opt/homebrew/opt/openjdk@21
./gradlew wrapper --gradle-version 9.8.1 --distribution-type bin \
  --gradle-distribution-sha256-sum dce76f55f8e251a3a1f130eb120f30b3d271de2b76c9b0729d316b5a1b6dc01f
./gradlew --version   # Gradle 9.8.1
grep distributionSha256Sum gradle/wrapper/gradle-wrapper.properties
```

`./gradlew wrapper` configures the project, so run it after the build files above exist.

- [ ] **Step 2: Hand-written types the generated file needs**

`WhichrideColors.kt`:

```kotlin
package io.github.locadani.whichride.designsystem

import androidx.compose.runtime.Immutable
import androidx.compose.ui.graphics.Color

/** One theme's colour roles, named as in design/tokens.json (`theme.<mode>.<role>`). */
@Immutable
data class WhichrideColors(
    val accent: Color,
    val accentText: Color,
    val background: Color,
    val disabled: Color,
    val error: Color,
    val hero: Color,
    val onAccent: Color,
    val onDisabled: Color,
    val onError: Color,
    val onHero: Color,
    val onSurface: Color,
    val onSurfaceMuted: Color,
    val outline: Color,
    val outlineVariant: Color,
    val scrim: Color,
    val surface: Color,
    val surfaceHigh: Color,
    val surfaceHighest: Color,
    val surfaceLow: Color,
)
```

`TypeToken.kt`:

```kotlin
package io.github.locadani.whichride.designsystem

/** Which bundled font a text style uses: Sora for text, JetBrains Mono for prices, distances and times. */
internal enum class FontRole { TEXT, NUMBER }

/** One text style from design/tokens.json; [lineHeight] is a multiple of [sizeSp]. */
internal data class TypeToken(
    val font: FontRole,
    val sizeSp: Float,
    val weight: Int,
    val lineHeight: Float,
    val letterSpacingEm: Float,
)
```

Run: `./gradlew :designsystem:compileDebugKotlin`
Expected: BUILD SUCCESSFUL. The generated `WhichrideTokens.kt` compiles against these types.
A role in `tokens.json` without a constructor parameter here is a compile error.

- [ ] **Step 3: Write the failing tests** —
`android/designsystem/src/test/kotlin/io/github/locadani/whichride/designsystem/ThemeTest.kt`

```kotlin
package io.github.locadani.whichride.designsystem

import androidx.compose.material3.ColorScheme
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.unit.TextUnitType
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ThemeTest {
    /** Every `Color` property of [target], read through the getters Kotlin compiles value classes to. */
    private fun colorsOf(target: Any): Map<String, Color> =
        target.javaClass.methods
            .filter { it.parameterCount == 0 && it.returnType == java.lang.Long.TYPE && it.name.startsWith("get") }
            .associate { it.name.substringAfter("get").substringBefore('-') to Color((it.invoke(target) as Long).toULong()) }

    private fun assertOnlyOurColors(scheme: ColorScheme, colors: WhichrideColors, inversePrimary: Color) {
        val allowed = colorsOf(colors).values.toSet() + inversePrimary
        val roles = colorsOf(scheme)
        assertTrue("ColorScheme exposes no colours: the reflection broke", roles.size >= 25)
        val foreign = roles.filterValues { it !in allowed }
        assertTrue("ColorScheme roles not mapped from tokens: ${foreign.keys}", foreign.isEmpty())
    }

    @Test
    fun `every Material colour role comes from our tokens, in both themes`() {
        val light = WhichrideTokens.LightColors
        val dark = WhichrideTokens.DarkColors
        assertOnlyOurColors(light.toMaterial(inversePrimary = dark.accent), light, dark.accent)
        assertOnlyOurColors(dark.toMaterial(inversePrimary = light.accent), dark, light.accent)
    }

    @Test
    fun `the main roles map as documented`() {
        val c = WhichrideTokens.DarkColors
        val m = c.toMaterial(inversePrimary = WhichrideTokens.LightColors.accent)
        assertEquals(c.accent, m.primary)
        assertEquals(c.onAccent, m.onPrimary)
        assertEquals(c.background, m.background)
        assertEquals(c.surface, m.surfaceContainer)
        assertEquals(c.outline, m.outline)
        assertEquals(c.onSurfaceMuted, m.onSurfaceVariant)
        assertEquals(Color(0xFF3DDC97), m.primary)
    }

    private fun allTextStyles(): List<Pair<String, TextStyle>> = with(WhichrideTypography) {
        listOf(
            "displayLarge" to displayLarge, "displayMedium" to displayMedium, "displaySmall" to displaySmall,
            "headlineLarge" to headlineLarge, "headlineMedium" to headlineMedium, "headlineSmall" to headlineSmall,
            "titleLarge" to titleLarge, "titleMedium" to titleMedium, "titleSmall" to titleSmall,
            "bodyLarge" to bodyLarge, "bodyMedium" to bodyMedium, "bodySmall" to bodySmall,
            "labelLarge" to labelLarge, "labelMedium" to labelMedium, "labelSmall" to labelSmall,
        )
    } + listOf(
        "numbers.large" to WhichrideNumberStyles.large,
        "numbers.medium" to WhichrideNumberStyles.medium,
        "numbers.small" to WhichrideNumberStyles.small,
    )

    @Test
    fun `text sizes scale with the user's font size`() {
        for ((name, style) in allTextStyles()) {
            assertEquals("$name fontSize", TextUnitType.Sp, style.fontSize.type)
            assertEquals("$name lineHeight", TextUnitType.Sp, style.lineHeight.type)
        }
    }

    @Test
    fun `every style uses a bundled font at a bundled weight`() {
        for ((name, style) in allTextStyles()) {
            assertTrue("$name font", style.fontFamily == Sora || style.fontFamily == JetBrainsMono)
            assertTrue("$name weight ${style.fontWeight}", style.fontWeight!!.weight in WhichrideTokens.Type.weights)
        }
        assertEquals(WhichrideTypography.headlineLarge, WhichrideTypography.displayLarge)
        assertEquals(JetBrainsMono, WhichrideNumberStyles.large.fontFamily)
        assertEquals(36f, WhichrideNumberStyles.large.fontSize.value)
        assertEquals(30f * 1.1f, WhichrideTypography.headlineLarge.lineHeight.value, 0.001f)
    }
}
```

Run: `./gradlew :designsystem:testDebugUnitTest`
Expected: FAIL to compile (`toMaterial`, `WhichrideTypography`, `Sora` unresolved).

- [ ] **Step 4: Implement type, shapes and colour mapping**

`Type.kt`:

```kotlin
package io.github.locadani.whichride.designsystem

import androidx.compose.material3.Typography
import androidx.compose.runtime.Immutable
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp

private fun variableFamily(resId: Int) = FontFamily(
    WhichrideTokens.Type.weights.map { w ->
        Font(resId, FontWeight(w), variationSettings = FontVariation.Settings(FontVariation.weight(w)))
    },
)

internal val Sora = variableFamily(R.font.sora)
internal val JetBrainsMono = variableFamily(R.font.jetbrains_mono)

internal fun TypeToken.toTextStyle() = TextStyle(
    fontFamily = when (font) {
        FontRole.TEXT -> Sora
        FontRole.NUMBER -> JetBrainsMono
    },
    fontWeight = FontWeight(weight),
    fontSize = sizeSp.sp,
    lineHeight = (sizeSp * lineHeight).sp,
    letterSpacing = letterSpacingEm.em,
)

/** Prices, distances and times; not part of Material's type scale. */
@Immutable
data class NumberStyles(val large: TextStyle, val medium: TextStyle, val small: TextStyle)

internal val WhichrideNumberStyles = with(WhichrideTokens.Type) {
    NumberStyles(numberLarge.toTextStyle(), numberMedium.toTextStyle(), numberSmall.toTextStyle())
}

/** The design has no display sizes; Material's display slots use our largest headline. */
internal val WhichrideTypography: Typography = with(WhichrideTokens.Type) {
    Typography(
        displayLarge = headlineLarge.toTextStyle(),
        displayMedium = headlineLarge.toTextStyle(),
        displaySmall = headlineLarge.toTextStyle(),
        headlineLarge = headlineLarge.toTextStyle(),
        headlineMedium = headlineMedium.toTextStyle(),
        headlineSmall = headlineSmall.toTextStyle(),
        titleLarge = titleLarge.toTextStyle(),
        titleMedium = titleMedium.toTextStyle(),
        titleSmall = titleSmall.toTextStyle(),
        bodyLarge = bodyLarge.toTextStyle(),
        bodyMedium = bodyMedium.toTextStyle(),
        bodySmall = bodySmall.toTextStyle(),
        labelLarge = labelLarge.toTextStyle(),
        labelMedium = labelMedium.toTextStyle(),
        labelSmall = labelSmall.toTextStyle(),
    )
}
```

If the compiler reports `FontVariation` or the `variationSettings` parameter as experimental,
add `@OptIn(ExperimentalTextApi::class)` to `variableFamily` (import
`androidx.compose.ui.text.ExperimentalTextApi`). Don't change anything else.

`Shapes.kt`:

```kotlin
package io.github.locadani.whichride.designsystem

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Shapes

/** Canvas radii: 10 logo tile, 14 rows and inputs, 16 buttons, 20 cards, 28 bottom sheets. */
internal val WhichrideShapes = with(WhichrideTokens.Radius) {
    Shapes(
        extraSmall = RoundedCornerShape(sm),
        small = RoundedCornerShape(md),
        medium = RoundedCornerShape(lg),
        large = RoundedCornerShape(xl),
        extraLarge = RoundedCornerShape(xxl),
    )
}
```

`MaterialColors.kt` (map **every** colour parameter `lightColorScheme` accepts; the reflection
test fails on any role left at Material's default):

```kotlin
package io.github.locadani.whichride.designsystem

import androidx.compose.material3.ColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color

/**
 * Material 3 colour scheme built only from our roles, so no Material component falls back to
 * Material's default purple. [inversePrimary] is the other theme's accent (snackbar actions on
 * the inverse surface). Light and dark differ only in the values, so one builder serves both.
 */
internal fun WhichrideColors.toMaterial(inversePrimary: Color): ColorScheme = lightColorScheme(
    primary = accent,
    onPrimary = onAccent,
    primaryContainer = surfaceHighest,
    onPrimaryContainer = onSurface,
    inversePrimary = inversePrimary,
    secondary = accent,
    onSecondary = onAccent,
    secondaryContainer = surfaceHighest,
    onSecondaryContainer = onSurface,
    tertiary = accent,
    onTertiary = onAccent,
    tertiaryContainer = surfaceHighest,
    onTertiaryContainer = onSurface,
    background = background,
    onBackground = onSurface,
    surface = background,
    onSurface = onSurface,
    surfaceVariant = surfaceHighest,
    onSurfaceVariant = onSurfaceMuted,
    surfaceTint = background,
    inverseSurface = onSurface,
    inverseOnSurface = background,
    error = error,
    onError = onError,
    errorContainer = surfaceHighest,
    onErrorContainer = onSurface,
    outline = outline,
    outlineVariant = outlineVariant,
    scrim = scrim,
    surfaceBright = surface,
    surfaceContainer = surface,
    surfaceContainerHigh = surfaceHigh,
    surfaceContainerHighest = surfaceHighest,
    surfaceContainerLow = surfaceLow,
    surfaceContainerLowest = background,
    surfaceDim = background,
)
```

If this Material version's `lightColorScheme` has more colour parameters (e.g. the "fixed"
roles `primaryFixed`, `primaryFixedDim`, `onPrimaryFixed`, `onPrimaryFixedVariant` and the same
for secondary/tertiary), the reflection test lists them. Map every `*Fixed` and `*FixedDim` to
`accent`, and every `on*Fixed` and `on*FixedVariant` to `onAccent`. Repeat until the test passes.

- [ ] **Step 5: The theme**

`WhichrideTheme.kt`:

```kotlin
package io.github.locadani.whichride.designsystem

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.LocalMinimumInteractiveComponentSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf

private val LocalWhichrideColors = staticCompositionLocalOf { WhichrideTokens.LightColors }
private val LocalNumberStyles = staticCompositionLocalOf { WhichrideNumberStyles }

/**
 * The app's theme: follows the system's light/dark setting, never the wallpaper (no dynamic
 * colour: decision 51), so every colour pair is one CI has checked for contrast.
 */
@Composable
fun WhichrideTheme(darkTheme: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
    val colors = if (darkTheme) WhichrideTokens.DarkColors else WhichrideTokens.LightColors
    val other = if (darkTheme) WhichrideTokens.LightColors else WhichrideTokens.DarkColors
    CompositionLocalProvider(
        LocalWhichrideColors provides colors,
        LocalNumberStyles provides WhichrideNumberStyles,
        LocalMinimumInteractiveComponentSize provides WhichrideTokens.Size.touchTarget,
    ) {
        MaterialTheme(
            colorScheme = colors.toMaterial(inversePrimary = other.accent),
            typography = WhichrideTypography,
            shapes = WhichrideShapes,
            content = content,
        )
    }
}

/** Roles Material has no slot for (hero card, accent text, disabled), and the number styles. */
object WhichrideTheme {
    val colors: WhichrideColors
        @Composable @ReadOnlyComposable get() = LocalWhichrideColors.current
    val numbers: NumberStyles
        @Composable @ReadOnlyComposable get() = LocalNumberStyles.current
}
```

`ThemePreview.kt` (for the owner to look at in Android Studio; no logic):

```kotlin
package io.github.locadani.whichride.designsystem

import android.content.res.Configuration
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.tooling.preview.Preview

@Preview(name = "Light", widthDp = 390)
@Preview(name = "Dark", widthDp = 390, uiMode = Configuration.UI_MODE_NIGHT_YES)
@Composable
private fun ThemePreview() = WhichrideTheme {
    val space = WhichrideTokens.Space
    Column(
        Modifier.background(MaterialTheme.colorScheme.background).fillMaxWidth().padding(space.md),
        verticalArrangement = Arrangement.spacedBy(space.sm),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(space.sm)) {
            Image(painterResource(R.drawable.whichride_logo), contentDescription = null)
            Text("whichride", style = MaterialTheme.typography.titleLarge, color = MaterialTheme.colorScheme.onBackground)
        }
        Text("Where to?", style = MaterialTheme.typography.headlineLarge, color = MaterialTheme.colorScheme.onBackground)
        Row(
            Modifier.fillMaxWidth()
                .background(WhichrideTheme.colors.hero, RoundedCornerShape(WhichrideTokens.Radius.xl))
                .padding(space.md),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text("MILES · S/M", style = MaterialTheme.typography.titleLarge, color = WhichrideTheme.colors.onHero)
            Text("€8.49", style = WhichrideTheme.numbers.large, color = WhichrideTheme.colors.onHero)
        }
        Text("7.4 km · 22 min", style = WhichrideTheme.numbers.small, color = WhichrideTheme.colors.onSurfaceMuted)
        Button(onClick = {}, Modifier.fillMaxWidth()) { Text("Open MILES") }
        OutlinedButton(onClick = {}, Modifier.fillMaxWidth()) { Text("Add the cars near you") }
        Row(horizontalArrangement = Arrangement.spacedBy(space.sm)) {
            Switch(checked = true, onCheckedChange = {})
            Switch(checked = false, onCheckedChange = {})
        }
    }
}
```

- [ ] **Step 6: Run the tests, lint and a release build**

Run: `./gradlew :designsystem:testDebugUnitTest :designsystem:lintDebug :designsystem:assembleRelease`
Expected: BUILD SUCCESSFUL, 4 tests passed. If lint reports errors, fix them. Don't change lint
severities.

- [ ] **Step 7: OWNER — look at the preview** (optional, not blocking)

Open `android/` in Android Studio, open `ThemePreview.kt`, compare the Light and Dark previews
with the canvas's Main artboard. The off switch in Light now has a visible border (decision 51).

- [ ] **Step 8: Commit**

```bash
cd ..
git add android
git status --short   # no local.properties, no build/ or .gradle/ folders
git commit -m "feat(android): add the designsystem module with WhichrideTheme

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Single-source guard, CI, Dependabot, decisions and docs

**Files:**
- Create: `design/test/single-source.test.mjs`
- Modify: `.github/workflows/ci.yml`, `.github/dependabot.yml`, `docs/DECISIONS.md`,
  `docs/plans/ROADMAP.md`, `README.md`

- [ ] **Step 0: Guard the single source** — `design/test/single-source.test.mjs`

Every graphic value must come from `tokens.json`. This test scans hand-written UI source (every
`.kt` under `android/*/src/main/`, and later `.css`/`.astro` under `web/src/` except the
generated `tokens.css`) for literal colours and sizes. It runs with `npm test`, so CI enforces
it from the `design` job.

```js
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
```

Run: `cd design && npm test`
Expected: PASS. If the first test lists a line in the hand-written Kotlin from Task 5, replace
the literal with the token it stands for (the plan's Kotlin has none).

- [ ] **Step 1: CI jobs** — append to `jobs:` in `.github/workflows/ci.yml`

```yaml
  design:
    name: design tokens · contrast · generated files
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: design
    steps:
      - uses: actions/checkout@v7
        with:
          persist-credentials: false
      - uses: actions/setup-node@v7
        with:
          node-version-file: design/.nvmrc
          cache: npm
          cache-dependency-path: design/package-lock.json
      - run: npm ci
      - run: npm test
      - run: npm run contrast
      - run: npm run build
      - name: generated files are committed and up to date
        working-directory: .
        run: |
          if [ -n "$(git status --porcelain -- android web)" ]; then
            git status --porcelain -- android web
            echo "::error::Generated files are stale: run 'npm run build' in design/ and commit the result"
            exit 1
          fi

  android:
    name: android designsystem
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: android
    steps:
      - uses: actions/checkout@v7
        with:
          persist-credentials: false
      - uses: actions/setup-java@v6
        with:
          distribution: temurin
          java-version: "21"
      - uses: gradle/actions/setup-gradle@3f5f9adaf7d9fecd50b5935e54106014257a94e6 # v6.4.0
      - run: ./gradlew --no-daemon :designsystem:testDebugUnitTest :designsystem:lintDebug :designsystem:assembleRelease
```

`setup-gradle` validates the wrapper's checksum. The runner's Android SDK has its licences
accepted, so the Android Gradle Plugin downloads platform 37 if it is missing.

- [ ] **Step 2: Dependabot** — append to `updates:` in `.github/dependabot.yml`

```yaml
  - package-ecosystem: npm
    directory: /design
    schedule:
      interval: weekly
    groups:
      design-dependencies:
        patterns: ["*"]
  - package-ecosystem: gradle
    directory: /android
    schedule:
      interval: weekly
    groups:
      android-dependencies:
        patterns: ["*"]
```

- [ ] **Step 3: Decisions** — append rows 51–57 to `docs/DECISIONS.md`

```markdown
| 51 | 2026-10-20 | Design | Colours are the Design canvas's light and dark palettes, as 19 roles named after Material 3's; the app **follows the system theme** and never uses dynamic (wallpaper) colour. Control boundaries use the muted colour (`outline`, ≥ 3:1), dividers keep the canvas line colour (`outline-variant`). Error colours, absent from the canvas, are Material's baseline (`#B3261E`, `#F2B8B5`). | Dynamic colour; one theme; the canvas's line colour for control borders | Dynamic colour would show pairs CI never checked; the canvas's light-mode off switch (white knob on `#CBD4DC`) is 1.5:1, below WCAG 1.4.11 |
| 52 | 2026-10-20 | Design | Fonts **Sora** (text) and **JetBrains Mono** (prices, distances, times), as in the canvas, **bundled** in the app and **self-hosted** on the website from the variable font files of the Google Fonts repository (SIL OFL 1.1) | Google Fonts CDN; Android downloadable fonts; system fonts | No request to a third party carries the user's IP (spec §9; LG München I, 3 O 17493/20, ruled embedding Google Fonts a GDPR breach); works offline and without Play services; looks like the design |
| 53 | 2026-10-20 | Design | Type scale: the canvas's 13 sizes become Material 3's 12 headline/title/body/label roles plus 3 number styles (36/18/12, mono); the 24 px screen title becomes 26 (`headline-medium`). Sizes are `sp` on Android and `rem` on the web. | Exact canvas sizes per screen | A named scale keeps screens consistent; `sp`/`rem` follow the user's font size (spec §11) |
| 54 | 2026-10-20 | Design | Spacing on a 4-point scale (4–40); radii 10/14/16/20/28/full from the canvas; minimum touch target **48 dp** (the canvas's 44 px icon buttons grow to 48) | Canvas's 44 px; every canvas gap as a token | 48 dp is Android's and Material's minimum; a short scale is easier to apply than the canvas's 12 different gaps |
| 55 | 2026-10-20 | Design | Style Dictionary **resolves** `tokens.json`; our own small formatters write the Kotlin (`WhichrideColors` constructors) and CSS (both themes in one file). Generated files are **committed where they are used**; CI rebuilds them and fails on any difference, and fails on any listed colour pair below WCAG AA. | Style Dictionary's built-in `compose/object` and `css/variables` formats; generating at build time | Built-in formats write one theme per file and plain `Color` constants, so a role missing in one theme would compile; committed output is readable in reviews and needs no Node in the Android or Astro builds |
| 56 | 2026-10-20 | Design | **Every graphic value lives in `tokens.json` only** (owner requirement): `logo.svg` holds only the logo's shape (a `<rect>` tile and one stroked `<path>`) and names its colours as `{brand.*}` references, in **fixed brand colours** for both themes; our own ~100-line generator turns it into the vector logo, the adaptive launcher icon (with themed-icon layer) and the favicon; a test fails on any literal colour, `dp` or `sp` in hand-written UI code | Colours written in `logo.svg` and checked against the tokens; `svg2vectordrawable` (last release June 2022); Android Studio's manual import | One place to change any value; reproducible in CI with no unmaintained dependency; a constrained source can't lose detail silently |
| 57 | 2026-10-20 | Android | The Gradle project `android/` starts now with one library module, **`:designsystem`** (`WhichrideTheme`), using the spike's proven versions; the app module joins it in milestone 5 | Wait for milestone 5; put the theme in the app module | The theme is tested and compiled in CI before any screen uses it; screens and a future widget share it |
```

- [ ] **Step 4: Roadmap and README**

`docs/plans/ROADMAP.md`, row 4: change the plan cell from `—` to `04-design-system.md ✅`.

`README.md`: update the status line to say milestones 1–4 are done, and add to the Development
section:

```sh
cd design && npm ci && npm test && npm run contrast   # design tokens (Node 24)
npm run build                                          # regenerate android/… and web/…; commit the result
cd ../android && ./gradlew :designsystem:testDebugUnitTest   # JAVA_HOME = a JDK 21
```

and under Design: `- [Design tokens and logo](design/README.md)`.

- [ ] **Step 5: Full verification**

```bash
cd design && npm ci && npm test && npm run contrast && npm run build && cd ..
test -z "$(git status --porcelain -- android web)" && echo "generated files up to date"
cd android && ./gradlew :designsystem:testDebugUnitTest :designsystem:lintDebug :designsystem:assembleRelease && cd ..
cargo test --workspace --locked   # untouched, must still pass
```

Expected: all pass; `generated files up to date`.

- [ ] **Step 6: Commit**

```bash
git add design/test/single-source.test.mjs .github/workflows/ci.yml .github/dependabot.yml docs/DECISIONS.md docs/plans/ROADMAP.md README.md
git commit -m "ci: check design tokens, contrast and the Android theme; record decisions 51-57

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 7: OWNER — push and PR**

```bash
git push -u origin feat/m4-design-system
```

The owner opens the PR on GitHub and squash-merges once CI is green.

---

## Notes for milestones 5–7

- **Milestone 5:** the app module depends on `:designsystem`, wraps every screen in
  `WhichrideTheme`, sets `android:icon="@mipmap/ic_launcher"`, and lists the two font
  licences (`design/fonts/*-OFL.txt`) in an open-source licences screen (OFL requires the
  licence to travel with the fonts). New colour pairs a screen introduces go into
  `contrast.json` first.
- **Milestone 6:** the Play Store needs a 512 × 512 PNG icon; render it from `logo.svg` then.
- **Milestone 7:** Astro imports `web/src/styles/tokens.css`; `web/public/` already holds the
  favicon and the fonts. Converting the fonts to WOFF2 belongs there.
