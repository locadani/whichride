# Design tokens

The only place whichride's colours, fonts, sizes and logo are edited (spec §10.5). Everything
the Android app and the website use is generated from here and committed; CI fails if a
generated file is stale or a colour pair misses WCAG 2.2 AA.

| Source | What it holds |
|---|---|
| `tokens.json` | Palette, brand colours, light and dark theme roles, fonts, type scale, spacing, radii, sizes (W3C design token format) |
| `contrast.json` | Colour pairs the UI uses; each must meet 4.5:1 (text) or 3:1 (control boundaries) in both themes |
| `logo.svg` | The logo's shape: one `<rect>` tile, then `<path>` layers that are either filled or stroked, coloured by `{brand.*}` references to `tokens.json`. Android's themed icon draws only the stroked layers not marked `data-monochrome="omit"` |
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
