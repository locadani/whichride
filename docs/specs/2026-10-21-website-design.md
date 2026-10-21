# Website — Design Spec

**Status:** Approved in brainstorming 2026-10-21 (milestone 7); reviewed against the code
2026-10-21 (§7). Amends §8.1, §10.4 and §12 of `2026-10-19-carsharing-compare-design.md`.

## 1. Purpose

A web page for people without the app (iPhone, desktop, a Google search): type a trip's km and
minutes, tick the cars near you, see which is cheapest. Same prices and same ranking as the app,
because both run the same Rust core.

**Owner's words:** "allow to put the values (distance, time) and allow to choose cars from the
available ones. nothing else. it must be simple."

**Success:** a visitor on a phone or desktop, in EN/IT/DE/TR, gets the same ranking the app would
show for the same km, minutes and cars; the page is a plain, indexable, accessible HTML page.

## 2. Scope

**In:**
- One comparison page per language: `/en/`, `/it/`, `/de/`, `/tr/`.
- Km and minutes typed by hand; a car picker; the ranked result; the "cheaper cars you did not
  add" line.
- Served by the existing Axum server on Cloud Run, at its current `run.app` URL.

**Out** (owner: "nothing else"):
- Google Maps links, address search, location, routing of any kind.
- Turnstile, web quotas, per-IP limits, any new API endpoint.
- "Any <provider> car", "Car not listed? Add it", Open <provider>, Open in Google Maps.
- Personal discounts, price reports, votes, price breakdown.
- Per-provider SEO pages, a privacy page, a custom domain.
- The design-token findings of 2026-10-21 (DTCG 2025.10 syntax, splash colour, WOFF2,
  favicon set): not part of this milestone.

## 3. Page

Layout "C" from the mockups (https://claude.ai/artifact/K5rWXiWxhKcAV3Am91G54p), one column,
centred on wide screens:

1. **Header:** logo + "whichride", language switcher (links to the four language pages).
2. **Heading:** "Which carsharing is cheapest in Berlin?" (web-only string).
3. **Trip:** two fields side by side, *Distance (km)* and *Time (min)*. Km accepts a decimal
   comma or point; limits as in the app: 0.1–2000 km, 1–1440 min, with the app's error texts.
4. **No cars yet** (first visit, or after expiry): a card with "Which cars are **near you?**",
   the app's intro text and the button **Add the cars near you**.
5. **Picker sheet** (a modal dialog): title "Which cars are near you?" (`intro_title`), the
   app's "Tick every car you see nearby. Prices are for this trip.", every offer grouped by
   provider in catalogue order (rows don't jump while the trip is typed), each with a checkbox
   and its price for the trip (no price while the trip is invalid), a close button, and the
   button **Add N cars** (N = cars ticked in the sheet; with nothing ticked it reads **Done**).
   The ticks are a draft: the button applies them, closing (✕ or Esc) drops them, as in the app.
   Focus then returns to the button that opens the sheet on the page as it now is.
6. **Result** (at least one car): the cars cheapest first as "Provider · offer" with the price,
   plus the app's "%1$s package" when a package beats the standard tariff; the cheapest
   highlighted with "Cheapest of your options"; under them the app's "N cars you did not add are
   cheaper, from …" or "No car you did not add is cheaper."; then **Add more cars** (reopens the
   sheet with the current ticks). While the trip is empty or invalid the ticked cars are listed
   without prices, highlight or cheaper line, and the field shows its error (an empty field
   shows none).
   Under the result: "Loading prices…" while the server's prices load, "built-in prices" if
   they could not be loaded, nothing once they are in.
7. **Footer:** "Not affiliated with any provider" and a link to the source code on GitHub.

**Memory:** the ticked cars and the trip are kept in the browser's `localStorage` for
**15 minutes** from the last change (app §4.1: cars move), then cleared. Nothing is sent
anywhere. If storage is unavailable the page works without it. Stored values are untrusted:
malformed entries are dropped and offers no longer in the catalogue are ignored.

**Without JavaScript:** a `<noscript>` line says the comparison needs JavaScript.

**Prices:** formatted with `Intl.NumberFormat` for the page's language, euros.

**Accessibility:** WCAG 2.2 AA. Real `<label>`s, native checkboxes, the sheet is a `<dialog>`
with focus moved in and returned on close, the result region is `aria-live="polite"`, 48 px
touch targets, text sizes in `rem`. Colours, sizes and fonts only from
`web/src/styles/tokens.css` (both themes follow `prefers-color-scheme`). The single-source guard
forbids literal lengths in `web/src`, and custom properties don't work in media queries, so the
page has **no media queries**: the centred column is `max-inline-size` from a new token
`size.content-max`.

## 4. Architecture

```
crates/wasm/        Rust → WebAssembly (wasm-bindgen): thin wrapper over carsharing-core
web/                Astro static site + one Svelte 5 island
crates/server/      serves web/dist next to the API
Dockerfile          builds WASM and the site, ships them in the server image
```

### 4.1 `crates/wasm`

The browser's counterpart of `crates/ffi`: the core stays free of wasm-bindgen (decision 58).

- `Prices.fromServer(json)`: reads the `catalogue` field of `GET /v1/catalogue`. Only accepted
  prices are used; pending reports and proposals are ignored (the web is read-only, spec §8.1).
- `Prices.fromSeed(json)`: the seed `crates/server/catalogue.json`.
- `prices.offers(km, minutes)`: every offer with provider name, offer name and price for the
  trip, in `compare()` order (for the sheet).
- `prices.options(km, minutes, offerIds)`: the core's `options()`: ranked chosen offers and the
  cheaper ones not added.
- Trip built from metres and seconds as the app does; no personal adjustments.
- Values cross the boundary as plain objects (`serde-wasm-bindgen`); money stays integer cents.
- Built with `cargo build --target wasm32-unknown-unknown --release` and `wasm-bindgen
  --target web`, the CLI version pinned to the crate version in `Cargo.lock`. Output is a build
  artefact, not committed (as the Kotlin bindings, decision 58).

### 4.2 `web/`

- **Astro** static build, `output: 'static'`, `site` from `WHICHRIDE_SITE_URL` (default the
  current Cloud Run URL), so a domain later is one setting.
- Pages `src/pages/[lang]/index.astro` for the four languages; `<html lang>`, `<title>`, meta
  description, canonical, `hreflang` alternates (incl. `x-default` → `/en/`), `robots.txt` with
  a sitemap of the four URLs.
- **One Svelte 5 island** (`client:load`) holds trip, picker and result; Astro renders the static
  shell and the island's first state (no cars) to HTML.
- **Prices at runtime:** the island fetches `/v1/catalogue` (same origin); on any failure it uses
  the seed catalogue embedded at build time. The seed is shown first, so a slow server never
  delays the page (as the app, decision 60).
- Styles: `tokens.css` (generated, decision 55) plus component CSS using only its variables; the
  existing single-source guard test is extended to `web/src`.

### 4.3 Strings (option 1)

The app's `android/app/src/main/res/values{,-it,-de,-tr}/strings.xml` stay the only source for
wording the app and the site share. Research (2026-10-21) found native files as the canonical
source is what projects usually do; a generated neutral source would be a layer translation tools
don't know.

- A build-time loader (`web/src/i18n/android.ts`) reads an **allowlist** of keys from those files,
  unescapes Android's escapes (`\'`, `\"`, `\@`, `\?`, `\n`, `\uXXXX`, surrounding quotes),
  converts `%1$s`/`%1$d`/`%d` placeholders to positional `{1}` and keeps `<plurals>` as
  `{one, other}`, chosen at runtime with `Intl.PluralRules`.
- Shared keys: `intro_title`, `intro_title_accent`, `intro_body` (its provider list built as the
  app's `orList`: names joined with ", " and the last one through `list_or`), `add_cars`,
  `add_more_cars`, `cheaper_not_added`, `nothing_cheaper`, `which_cars_body`, `add_n_cars`,
  `done`, `cheapest_of_options`, `package_meta`, `list_or`, `manual_km`, `manual_minutes`,
  `manual_km_invalid`, `manual_minutes_invalid`, `loading_prices`, `built_in_prices`.
- **Web-only strings** (heading, meta description, footer, language names, sheet close label) in
  `web/src/i18n/{en,it,de,tr}.json`.
- Tests: every allowlisted key exists in all four Android files; every web-only key exists in all
  four JSON files; placeholders match English in every language; the unescaper is tested on each
  escape.
- The km/min limits (0.1–2000 km, 1–1440 min) are repeated in TypeScript; they match the app's
  `format/Units.kt` and the shared error strings, and a test pins them.

### 4.4 Server

- `WEB_DIR` (optional): when set, `tower-http`'s `ServeDir` serves it for every path the API
  doesn't match; unknown paths get the site's `404.html` with status 404. Unknown `/v1/…`
  paths keep the API's JSON 404. `serve` refuses a `WEB_DIR` without a built site. Unset
  (tests, local API work) → no site, as today.
- `/` → **302** to `/{lang}/`, the best of `Accept-Language` among en/it/de/tr, default `en`;
  `Vary: Accept-Language`.
- Caching: successful `/_astro/*` (content-hashed) `Cache-Control: public, max-age=31536000,
  immutable`; everything else `no-cache` (revalidated through `ServeDir`'s `Last-Modified`).
- Compression: Cloud Run doesn't compress, so the build writes `.br` and `.gz` next to each text
  file (and the WASM and fonts) and `ServeDir` serves them by `Accept-Encoding`.
- Security headers: `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`,
  `X-Frame-Options: DENY`. The page's CSP is Astro's `security.csp` `<meta>` (hashes for its own
  inline scripts), `script-src` with `'wasm-unsafe-eval'` for the WebAssembly, everything else
  `'self'`.
- Static requests go through the existing request-ID and trace layers; nothing new is logged.
- **Dockerfile:** the build stage also builds the WASM package; a Node stage runs the Astro build;
  `web/dist` is copied to `/srv/web` in the runtime image with `WEB_DIR=/srv/web`.

## 5. Testing

- **Rust:** `crates/wasm` logic tested natively (`cargo test`), the wasm-bindgen layer kept thin;
  results checked against `carsharing-core` for the same inputs.
- **Web (Vitest):** strings loader and unescaper; trip parsing and limits; storage expiry;
  the Svelte island with Testing Library (first visit, picking cars, ranking, cheaper line,
  dialog focus, seed fallback when the fetch fails), with the real WASM package.
- **Server:** `/` redirect by `Accept-Language`; a language page served; cache headers on
  `/_astro/` and HTML; API 404 unchanged; site 404 page.
- **CI:** a `web` job builds the WASM package and the site and runs Vitest; the Docker build
  runs in the existing deploy workflow.

## 6. Decisions to record

- Website served by the Cloud Run server at its `run.app` URL (no separate hosting, no domain
  for now).
- Web input is km and minutes only: no routing from the web, so **no Turnstile** (amends §8.1).
- Layout C: starts with no car, cars added through a sheet; kept 15 minutes in the browser.
- One Svelte 5 island inside Astro, chosen over plain TypeScript to avoid hand-written DOM
  updates.
- `crates/wasm` wraps the core as `crates/ffi` does; output not committed.
- Shared wording read from the app's `strings.xml` (option 1); web-only strings in JSON.
- No per-provider SEO pages (amends §10.4).
- Site delivery: precompressed files, immutable `/_astro/`, CSP meta + security headers.

## 7. Review against the code (2026-10-21)

Changes made to this spec after checking it against the repository:

1. **Provider list in `intro_body`:** the app builds it with `list_or` ("A, B or C"), not
   `Intl.ListFormat` (which would write "A, B, or C" in English); the site now does the same.
2. **Sheet title and row text** were unnamed: the sheet uses `intro_title`; result rows use the
   app's `package_meta` for a winning package. Both keys added to the allowlist.
3. **Sheet ticks are a draft** (as the app's sheet): undefined before.
4. **Empty or invalid trip with cars ticked** was unspecified: cars listed without prices.
5. **No literal lengths and no media queries** in `web/src` (the single-source guard already
   covers `.svelte`/`.astro`): one new token, `size.content-max`.
6. **API 404 kept:** a site fallback must not turn `/v1/unknown` into an HTML page; the app
   reads the JSON error.
7. **Compression and security headers** were missing; Cloud Run sends files as they are, and
   the WASM needs `'wasm-unsafe-eval'` under a CSP.
8. **Toolchain facts:** the core builds for `wasm32-unknown-unknown` unchanged (checked);
   `wasm-bindgen` 0.2.129 is already in `Cargo.lock`; `@astrojs/svelte` 9 accepts TypeScript
   ≤ 6, so TypeScript is 6.0.3, not 7.
9. **`<noscript>`** line added; stored values treated as untrusted.
