# Website (Milestone 7) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One comparison page per language (`/en/`, `/it/`, `/de/`, `/tr/`) where a visitor types km and minutes, ticks the cars near them and sees the same ranking the app would show, served by the existing Cloud Run server.

**Architecture:** A new crate `crates/wasm` wraps `carsharing-core` for the browser with wasm-bindgen (logic tested natively, as `crates/ffi` does for Android). `web/` is an Astro static site with one Svelte 5 island that loads the WASM, the server's `/v1/catalogue` (seed fallback), and renders trip fields, a car sheet and the result. Wording shared with the app is read at build time from the app's `strings.xml`; web-only wording lives in JSON. The server serves `web/dist` through `tower-http`'s `ServeDir` behind the API routes; the Dockerfile builds WASM and site into the same image.

**Tech Stack:** Rust (wasm-bindgen 0.2.129, serde-wasm-bindgen 0.6.5, Axum 0.8, tower-http 0.7), Astro 7.3.6, Svelte 5.57.2, @astrojs/svelte 9.0.1, TypeScript 6.0.3, Vitest 5.0.3 + Testing Library + jsdom, Node 24, Docker, GitHub Actions.

**Spec:** `docs/specs/2026-10-21-website-design.md` (approved 2026-10-21, reviewed against the code 2026-10-21, §7).

## Global Constraints

- Pages: `/en/`, `/it/`, `/de/`, `/tr/`; `/` → **302** to the best `Accept-Language` match, default `en`, with `Vary: Accept-Language`.
- Input is km and minutes only. No routing, no Turnstile, no new API endpoint, no Maps/Open-provider buttons, no discounts, no reports, no SEO provider pages, no privacy page, no domain.
- Limits exactly as the app (`android/app/src/main/kotlin/io/github/locadani/whichride/format/Units.kt`): km `^(\d{0,4})(?:[.,](\d{1,3}))?$` → metres in 100..=2 000 000; minutes `^\d{1,4}$` in 1..=1440 → seconds = minutes × 60.
- Money is integer cents everywhere; shown with `Intl.NumberFormat(lang, { style: 'currency', currency: 'EUR' })`.
- Browser memory: `localStorage` key `whichride.trip`, kept **15 minutes** from the last change; nothing is sent anywhere; storage failures are silent.
- Every colour, length and font in `web/src` comes from `web/src/styles/tokens.css` (`var(--wr-…)`); no literal `px`/`rem`/`em`/`pt`, no hex/`rgb()`, no colour keywords (incl. `transparent`), no `font-weight:`; **no media queries**. The guard `design/test/single-source.test.mjs` enforces it.
- WCAG 2.2 AA: real `<label>`s, native checkboxes, `<dialog>` sheet, result region `aria-live="polite"`, touch targets `var(--wr-size-touch-target)`.
- Shared wording only from `android/app/src/main/res/values{,-it,-de,-tr}/strings.xml`; never copy it into JSON.
- Exact npm versions (no `^`/`~`): astro 7.3.6, svelte 5.57.2, @astrojs/svelte 9.0.1, @astrojs/check 0.9.10, typescript 6.0.3, vitest 5.0.3, @testing-library/svelte 5.4.2, @testing-library/jest-dom 7.0.1, @testing-library/user-event 14.6.7, jsdom 30.1.2, fast-xml-parser 5.11.2.
- `wasm-bindgen` CLI version = the `wasm-bindgen` version in `Cargo.lock` (0.2.129 today), in the generate script, CI and the Dockerfile; a test checks it.
- WASM output, the copied seed and `dist/` are build artefacts, never committed.
- Site URL from `WHICHRIDE_SITE_URL`, default `https://whichride-1034831276867.europe-west3.run.app`.
- New decisions are numbered from **84**.
- Commit messages end with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Rust DB tests need `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres` (smoke-pg container); without it they skip locally.
- Work on branch `feat/m7-website` (already exists, spec commit on it).

## Review Focus

1. **Cars ticked but the trip empty or invalid** (a visitor clears the km field): the page must list the cars without prices, never show `NaN`/`€0.00`/a "Cheapest" badge, and show the field's error. Pinned in Task 6 (`an invalid trip lists the cars without prices`).
2. **Stale or tampered browser memory** (malformed JSON, offers removed from the catalogue, a future timestamp, storage that throws): the page starts cleanly and "Add N cars" counts only real cars. Pinned in Task 5 (memory tests) and Task 6 (`remembered cars that no longer exist are ignored`).
3. **The server's catalogue is slow, fails, or answers non-200/invalid JSON**: the seed prices stay usable and "built-in prices" shows; nothing waits for the server. Pinned in Task 3 (`serverEngine` rejects non-200 and invalid bodies) and Task 6 (`built-in prices when the server answers an error`; every other test runs offline).
4. **Closing the sheet with Esc or ✕, and focus afterwards**: ticks made in the sheet are dropped, and focus lands on the opener that exists *now* (after the first commit "Add the cars near you" is gone, so focus goes to "Add more cars"). Pinned in Task 6.
5. **The site fallback swallowing API paths**: `/v1/unknown` must still answer the JSON 404 the app expects, `/en` must redirect to `/en/`, and a 404 under `/_astro/` must not be cached as immutable. Pinned in Task 8.

---

### Task 1: Design tokens — content width, guard skips build output

**Files:**
- Modify: `design/tokens.json` (`size` group)
- Modify: `design/test/single-source.test.mjs` (skip list in `files()`)
- Modify: `design/test/orphans.test.mjs` (skip list in `files()`)
- Regenerate: `web/src/styles/tokens.css`, `android/designsystem/src/main/kotlin/io/github/locadani/whichride/designsystem/WhichrideTokens.kt`

**Interfaces:**
- Produces: CSS custom property `--wr-size-content-max` (600px) used by Tasks 6 and 7.

- [ ] **Step 1: Add the token**

In `design/tokens.json`, inside `"size"`, after `"stroke-strong"`:

```json
    "stroke-strong": { "$value": "2px", "$description": "The selected option's ring and a ticked car's border" },
    "content-max": { "$value": "600px", "$description": "Web only: width of the page's centred column (no media queries on the site)" }
```

- [ ] **Step 2: Skip build output in both repository walkers**

`web/dist`, `web/.astro` and `web/src/generated` hold built or copied files (wasm-bindgen glue, Astro output) that are not hand-written UI code and not design outputs. In **both** `design/test/single-source.test.mjs` and `design/test/orphans.test.mjs`, change the skip line in `files()`:

```js
    if (['node_modules', 'build', '.gradle', 'dist', '.astro', 'generated'].includes(e.name)) continue;
```

- [ ] **Step 3: Regenerate and run the design checks**

Run: `cd design && npm run build && npm test && npm run contrast`
Expected: PASS. `git diff --stat` shows `tokens.json`, the two tests, `web/src/styles/tokens.css` (new line `--wr-size-content-max: 600px;`) and `WhichrideTokens.kt` (new `contentMax`). If a design test lists size tokens explicitly (e.g. `formats.test.mjs`/`build.test.mjs`), add `content-max` to its expected list.

- [ ] **Step 4: Commit**

```bash
git add design web/src/styles/tokens.css android/designsystem/src/main/kotlin
git commit -m "design: size.content-max for the website column; guards skip build output

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: `crates/wasm` — the core for the browser

**Files:**
- Modify: `Cargo.toml` (workspace dependencies, new profile `web`)
- Modify: `rust-toolchain.toml` (targets)
- Create: `crates/wasm/Cargo.toml`
- Create: `crates/wasm/src/lib.rs` (wasm-bindgen layer)
- Create: `crates/wasm/src/prices.rs` (logic + native tests)

**Interfaces:**
- Consumes: `carsharing_core::{Catalogue, compare, options, Choice, OfferId, Trip, Adjustments, Quote}`.
- Produces (JavaScript, after wasm-bindgen `--target web`; module `whichride_wasm.js`):
  - `default init({ module_or_path })`, `initSync({ module })`
  - `class Prices { static fromServer(json: string): Prices; static fromSeed(json: string): Prices; providers(): ProviderInfo[]; offers(metres: number, seconds: number): Priced[]; options(metres: number, seconds: number, offerIds: string[]): Ranking }` (throws on invalid JSON/catalogue)
  - plain objects, camelCase: `ProviderInfo { id, name, offers: { id, name }[] }`, `Priced { providerId, offerId, offerName, cardLabel, totalCents }`, `Ranking { ranked: Priced[], cheaperNotAdded: Priced[] }`
- Build output path: `target/wasm32-unknown-unknown/web/whichride_wasm.wasm`.

- [ ] **Step 1: Workspace wiring**

`Cargo.toml`, in `[workspace.dependencies]` (alphabetical):

```toml
serde-wasm-bindgen = "0.6.5"
```
```toml
wasm-bindgen = "0.2.129"
```

After `[profile.android]`:

```toml
# The website's WebAssembly (cargo build --profile web --target wasm32-unknown-unknown): small
# download, no unwinding in wasm32 anyway.
[profile.web]
inherits = "release"
opt-level = "s"
lto = true
codegen-units = 1
```

`rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
targets = ["wasm32-unknown-unknown"]
```

`crates/wasm/Cargo.toml`:

```toml
[package]
name = "whichride-wasm"
version = "0.1.0"
description = "WebAssembly bindings of carsharing-core for the website"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
repository.workspace = true

[lib]
name = "whichride_wasm"
crate-type = ["cdylib", "rlib"]

[dependencies]
carsharing-core.workspace = true
serde.workspace = true
serde_json.workspace = true
serde-wasm-bindgen.workspace = true
wasm-bindgen.workspace = true

# Not the workspace lints: wasm-bindgen's macros expand to `unsafe` glue, and the workspace
# forbids `unsafe_code`. This crate's own code has none (as crates/ffi).
[lints.clippy]
all = { level = "warn", priority = -1 }
```

- [ ] **Step 2: Write the failing tests** (`crates/wasm/src/prices.rs`, tests module first, with empty stubs above so it compiles)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const SEED: &str = include_str!("../../server/catalogue.json");
    const METRES: u32 = 7_400;
    const SECONDS: u32 = 22 * 60;

    fn seed() -> WebPrices {
        WebPrices::from_seed(SEED).unwrap()
    }

    fn core_quotes(catalogue: &cc::Catalogue) -> Vec<cc::Quote> {
        let trip = cc::Trip {
            metres: METRES,
            seconds: SECONDS,
        };
        cc::compare(catalogue, &trip, &cc::Adjustments::default())
    }

    fn ids(priced: &[Priced]) -> Vec<&str> {
        priced.iter().map(|p| p.offer_id.as_str()).collect()
    }

    #[test]
    fn every_offer_is_priced_in_the_cores_order() {
        let core = core_quotes(&cc::Catalogue::from_json(SEED).unwrap());
        let offers = seed().offers(METRES, SECONDS);
        let expected: Vec<(&str, i64)> = core.iter().map(|q| (q.offer.0.as_str(), q.total.0)).collect();
        let got: Vec<(&str, i64)> = offers.iter().map(|o| (o.offer_id.as_str(), o.total_cents)).collect();
        assert!(!got.is_empty());
        assert_eq!(got, expected);
    }

    #[test]
    fn ticked_offers_are_ranked_as_the_core_ranks_them() {
        let ticked = ["bolt.vw-polo".to_owned(), "miles.s".to_owned()];
        let ranking = seed().options(METRES, SECONDS, &ticked);
        let choices: Vec<cc::Choice> = ticked
            .iter()
            .map(|id| cc::Choice::Offer {
                offer: cc::OfferId(id.clone()),
            })
            .collect();
        let core = cc::options(&core_quotes(&cc::Catalogue::from_json(SEED).unwrap()), &choices);
        assert_eq!(ranking.ranked.len(), 2);
        assert_eq!(
            ids(&ranking.ranked),
            core.ranked.iter().map(|o| o.quote.offer.0.as_str()).collect::<Vec<_>>()
        );
        assert_eq!(
            ids(&ranking.cheaper_not_added),
            core.cheaper_not_added.iter().map(|q| q.offer.0.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn nothing_or_unknown_offers_rank_nothing() {
        for ticked in [vec![], vec!["gone.car".to_owned()]] {
            let ranking = seed().options(METRES, SECONDS, &ticked);
            assert!(ranking.ranked.is_empty());
            assert!(ranking.cheaper_not_added.is_empty());
        }
    }

    #[test]
    fn providers_and_offers_keep_the_catalogue_order() {
        let catalogue = cc::Catalogue::from_json(SEED).unwrap();
        let providers = seed().providers();
        assert_eq!(
            providers.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            catalogue.providers.iter().map(|p| p.id.0.as_str()).collect::<Vec<_>>()
        );
        assert_eq!(
            providers[0].offers.iter().map(|o| o.id.as_str()).collect::<Vec<_>>(),
            catalogue.providers[0].offers.iter().map(|o| o.id.0.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_server_catalogue_is_used_and_crowd_extras_are_ignored() {
        let mut catalogue: serde_json::Value = serde_json::from_str(SEED).unwrap();
        catalogue["providers"][0]["offers"][0]["cards"][0]["per_km"] = 1.into();
        let body = serde_json::json!({
            "catalogue": catalogue,
            "offers": [{"offer": "miles.s", "confirmed_on": null, "locked": false,
                        "pending": {"report": "11111111-2222-4333-8444-555555555555", "cards": []}}],
            "proposals": [{"report": "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee", "provider": "bolt",
                           "name": "Fiat 500e", "cards": []}]
        });
        let server = WebPrices::from_server(&body.to_string()).unwrap();
        let edited: cc::Catalogue = serde_json::from_value(catalogue).unwrap();
        let expected: Vec<(String, i64)> =
            core_quotes(&edited).iter().map(|q| (q.offer.0.clone(), q.total.0)).collect();
        let got: Vec<(String, i64)> = server
            .offers(METRES, SECONDS)
            .iter()
            .map(|o| (o.offer_id.clone(), o.total_cents))
            .collect();
        assert_eq!(got, expected, "server prices, no proposals");
    }

    #[test]
    fn invalid_input_is_an_error_not_a_panic() {
        assert!(WebPrices::from_seed("{").is_err());
        assert!(WebPrices::from_server("{}").is_err(), "no catalogue field");
        assert!(WebPrices::from_server(r#"{"catalogue": {"providers": "x"}}"#).is_err());
    }
}
```

- [ ] **Step 3: Run to see them fail**

Run: `cargo test -p whichride-wasm`
Expected: FAIL (stubs return nothing / `todo!()`).

- [ ] **Step 4: Implement `prices.rs`** (above the tests module)

```rust
//! The website's view of the core (website spec §4.1): every offer priced for a typed trip, and
//! the visitor's ticked cars ranked. No personal adjustments: the web has no discounts.

use carsharing_core as cc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub offers: Vec<OfferInfo>,
}

/// One offer's best price for the trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Priced {
    pub provider_id: String,
    pub offer_id: String,
    pub offer_name: String,
    /// The tariff that gave the price, e.g. "Standard" or "3 h / 40 km".
    pub card_label: String,
    pub total_cents: i64,
}

impl From<&cc::Quote> for Priced {
    fn from(q: &cc::Quote) -> Self {
        Priced {
            provider_id: q.provider.0.clone(),
            offer_id: q.offer.0.clone(),
            offer_name: q.offer_name.clone(),
            card_label: q.card_label.clone(),
            total_cents: q.total.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ranking {
    pub ranked: Vec<Priced>,
    pub cheaper_not_added: Vec<Priced>,
}

/// The one field of `GET /v1/catalogue` the website uses.
#[derive(Deserialize)]
struct ServerCatalogue {
    catalogue: cc::Catalogue,
}

#[derive(Debug, Clone)]
pub struct WebPrices {
    catalogue: cc::Catalogue,
}

impl WebPrices {
    /// The body of `GET /v1/catalogue`: accepted prices only; pending reports and proposals are
    /// the app's (the web is read-only, spec §8.1).
    pub fn from_server(json: &str) -> Result<WebPrices, String> {
        let response: ServerCatalogue = serde_json::from_str(json).map_err(|e| e.to_string())?;
        response.catalogue.validate().map_err(|e| e.to_string())?;
        Ok(WebPrices {
            catalogue: response.catalogue,
        })
    }

    /// The seed catalogue (`crates/server/catalogue.json`) built into the site.
    pub fn from_seed(json: &str) -> Result<WebPrices, String> {
        cc::Catalogue::from_json(json)
            .map(|catalogue| WebPrices { catalogue })
            .map_err(|e| e.to_string())
    }

    /// Providers and their offers in catalogue order (the sheet's order).
    pub fn providers(&self) -> Vec<ProviderInfo> {
        self.catalogue
            .providers
            .iter()
            .map(|p| ProviderInfo {
                id: p.id.0.clone(),
                name: p.name.clone(),
                offers: p
                    .offers
                    .iter()
                    .map(|o| OfferInfo {
                        id: o.id.0.clone(),
                        name: o.name.clone(),
                    })
                    .collect(),
            })
            .collect()
    }

    /// Every offer's price for the trip, in `compare()` order.
    pub fn offers(&self, metres: u32, seconds: u32) -> Vec<Priced> {
        self.quotes(metres, seconds).iter().map(Priced::from).collect()
    }

    /// The ticked offers ranked, and the cheaper ones not ticked (the core's `options()`).
    /// Unknown offer IDs are skipped.
    pub fn options(&self, metres: u32, seconds: u32, offer_ids: &[String]) -> Ranking {
        let choices: Vec<cc::Choice> = offer_ids
            .iter()
            .map(|id| cc::Choice::Offer {
                offer: cc::OfferId(id.clone()),
            })
            .collect();
        let options = cc::options(&self.quotes(metres, seconds), &choices);
        Ranking {
            ranked: options.ranked.iter().map(|o| Priced::from(&o.quote)).collect(),
            cheaper_not_added: options.cheaper_not_added.iter().map(Priced::from).collect(),
        }
    }

    fn quotes(&self, metres: u32, seconds: u32) -> Vec<cc::Quote> {
        let trip = cc::Trip { metres, seconds };
        cc::compare(&self.catalogue, &trip, &cc::Adjustments::default())
    }
}
```

- [ ] **Step 5: Implement `lib.rs`**

```rust
//! The core for the website, through wasm-bindgen. Logic lives in `prices` and is tested
//! natively; this layer only converts to and from JavaScript values (as `crates/ffi` for UniFFI).

mod prices;

pub use prices::{OfferInfo, Priced, ProviderInfo, Ranking, WebPrices};

use serde::Serialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Prices(WebPrices);

#[wasm_bindgen]
impl Prices {
    #[wasm_bindgen(js_name = fromServer)]
    pub fn from_server(json: &str) -> Result<Prices, JsError> {
        WebPrices::from_server(json).map(Prices).map_err(|e| JsError::new(&e))
    }

    #[wasm_bindgen(js_name = fromSeed)]
    pub fn from_seed(json: &str) -> Result<Prices, JsError> {
        WebPrices::from_seed(json).map(Prices).map_err(|e| JsError::new(&e))
    }

    pub fn providers(&self) -> Result<JsValue, JsError> {
        to_js(&self.0.providers())
    }

    pub fn offers(&self, metres: u32, seconds: u32) -> Result<JsValue, JsError> {
        to_js(&self.0.offers(metres, seconds))
    }

    pub fn options(&self, metres: u32, seconds: u32, offer_ids: Vec<String>) -> Result<JsValue, JsError> {
        to_js(&self.0.options(metres, seconds, &offer_ids))
    }
}

fn to_js(value: &impl Serialize) -> Result<JsValue, JsError> {
    serde_wasm_bindgen::to_value(value).map_err(|e| JsError::new(&e.to_string()))
}
```

- [ ] **Step 6: Run tests, lints and the wasm build**

Run: `cargo test -p whichride-wasm && cargo clippy --workspace --all-targets --locked && cargo fmt --all --check && cargo build --profile web --locked -p whichride-wasm --target wasm32-unknown-unknown`
Expected: 6 tests PASS, no warnings, `target/wasm32-unknown-unknown/web/whichride_wasm.wasm` exists. `Cargo.lock` keeps `wasm-bindgen` 0.2.129 (if cargo picked a newer one, run `cargo update -p wasm-bindgen --precise 0.2.129`). Run `cargo deny check` if installed (licences of the two new crates are MIT/Apache-2.0).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml crates/wasm
git commit -m "feat(wasm): carsharing-core for the browser through wasm-bindgen

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: `web/` scaffold, generate script, price engine

**Files:**
- Create: `web/package.json`, `web/package-lock.json` (via `npm install`), `web/.nvmrc`, `web/.gitignore`
- Create: `web/astro.config.mjs`, `web/tsconfig.json`, `web/vitest.config.ts`
- Create: `web/scripts/generate.mjs`
- Create: `web/src/lib/engine.ts`
- Create: `web/test/setup.ts`, `web/test/engine.test.ts`, `web/test/versions.test.ts`

**Interfaces:**
- Consumes: Task 2's `Prices` class and build path.
- Produces:
  - `web/src/generated/wasm/whichride_wasm.js` (+ `_bg.wasm`, `.d.ts`) and `web/src/generated/catalogue.json` from `npm run generate`.
  - `engine.ts`: `type Source = 'loading' | 'server' | 'built-in'`; `interface OfferInfo { id: string; name: string }`; `interface ProviderInfo { id: string; name: string; offers: OfferInfo[] }`; `interface Priced { providerId: string; offerId: string; offerName: string; cardLabel: string; totalCents: number }`; `interface Ranking { ranked: Priced[]; cheaperNotAdded: Priced[] }`; `interface Engine { providers(): ProviderInfo[]; offers(metres: number, seconds: number): Priced[]; options(metres: number, seconds: number, offerIds: string[]): Ranking }`; `seedEngine(): Promise<Engine>`; `serverEngine(fetchFn?: typeof fetch): Promise<Engine>`; `CATALOGUE_TIMEOUT_MS = 10_000`.
  - `scripts/generate.mjs`: exported `lockedVersion(lockText: string, name: string): string | null`.
  - `astro.config.mjs`: exported `DEFAULT_SITE`.

- [ ] **Step 1: Project files**

`web/.nvmrc`: `24`

`web/.gitignore`:

```
node_modules/
dist/
.astro/
src/generated/
```

`web/package.json`:

```json
{
  "name": "whichride-web",
  "private": true,
  "type": "module",
  "description": "whichride's website: Astro pages and one Svelte island over the Rust core in WebAssembly.",
  "engines": { "node": ">=24" },
  "scripts": {
    "generate": "node scripts/generate.mjs",
    "dev": "astro dev",
    "build": "astro build && node scripts/compress.mjs",
    "check": "astro check",
    "test": "vitest run",
    "test:dist": "vitest run --config vitest.dist.config.ts"
  }
}
```

Then install exact versions:

```bash
cd web
npm install --save-exact astro@7.3.6 svelte@5.57.2 @astrojs/svelte@9.0.1
npm install --save-exact --save-dev @astrojs/check@0.9.10 typescript@6.0.3 vitest@5.0.3 jsdom@30.1.2 \
  @testing-library/svelte@5.4.2 @testing-library/jest-dom@7.0.1 @testing-library/user-event@14.6.7 \
  fast-xml-parser@5.11.2
```

(`build` references `scripts/compress.mjs` and `test:dist` its config; both arrive in Task 7. Until then use `npx astro build`.)

`web/astro.config.mjs`:

```js
import { defineConfig } from 'astro/config';
import svelte from '@astrojs/svelte';

/** The Cloud Run URL; a domain later is one setting (WHICHRIDE_SITE_URL). */
export const DEFAULT_SITE = 'https://whichride-1034831276867.europe-west3.run.app';

export default defineConfig({
  site: process.env.WHICHRIDE_SITE_URL || DEFAULT_SITE,
  output: 'static',
  trailingSlash: 'always',
  build: { format: 'directory' },
  integrations: [svelte()],
});
```

`web/tsconfig.json`:

```json
{
  "extends": "astro/tsconfigs/strict",
  "include": [".astro/types.d.ts", "**/*"],
  "exclude": ["dist", "src/generated"],
  "compilerOptions": { "types": ["vitest/globals", "@testing-library/jest-dom/vitest"] }
}
```

`web/vitest.config.ts`:

```ts
/// <reference types="vitest/config" />
import { getViteConfig } from 'astro/config';
import { svelteTesting } from '@testing-library/svelte/vite';

export default getViteConfig({
  plugins: [svelteTesting()],
  test: {
    environment: 'jsdom',
    globals: true,
    include: ['test/**/*.test.ts'],
    setupFiles: ['./test/setup.ts'],
  },
});
```

- [ ] **Step 2: Generate script**

`web/scripts/generate.mjs`:

```js
// Builds the core for the browser and copies the seed catalogue into src/generated (git-ignored).
// Needs the wasm32-unknown-unknown target (rust-toolchain.toml) and the wasm-bindgen CLI at the
// version in Cargo.lock: cargo install wasm-bindgen-cli --version <it> --locked
import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { fileURLToPath, pathToFileURL } from 'node:url';

const ROOT = fileURLToPath(new URL('../../', import.meta.url));
const OUT = fileURLToPath(new URL('../src/generated/', import.meta.url));

/** The version of a crate in Cargo.lock, or null. */
export function lockedVersion(lockText, name) {
  const match = lockText.match(new RegExp(`\\[\\[package\\]\\]\\nname = "${name}"\\nversion = "([^"]+)"`));
  return match ? match[1] : null;
}

function run(command, args, cwd) {
  execFileSync(command, args, { cwd, stdio: 'inherit' });
}

export function generate() {
  const wanted = lockedVersion(readFileSync(`${ROOT}Cargo.lock`, 'utf8'), 'wasm-bindgen');
  let installed = '';
  try {
    installed = execFileSync('wasm-bindgen', ['--version'], { encoding: 'utf8' }).trim();
  } catch {
    // reported below
  }
  if (installed !== `wasm-bindgen ${wanted}`) {
    throw new Error(
      `wasm-bindgen CLI ${wanted} is needed (found: ${installed || 'none'}).\n` +
        `Run: cargo install wasm-bindgen-cli --version ${wanted} --locked`,
    );
  }
  run('cargo', ['build', '--profile', 'web', '--locked', '-p', 'whichride-wasm', '--target', 'wasm32-unknown-unknown'], ROOT);
  rmSync(OUT, { recursive: true, force: true });
  mkdirSync(`${OUT}wasm`, { recursive: true });
  run('wasm-bindgen', ['--target', 'web', '--out-dir', `${OUT}wasm`, `${ROOT}target/wasm32-unknown-unknown/web/whichride_wasm.wasm`], ROOT);
  copyFileSync(`${ROOT}crates/server/catalogue.json`, `${OUT}catalogue.json`);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) generate();
```

Run: `cargo install wasm-bindgen-cli --version 0.2.129 --locked && cd web && npm run generate`
Expected: `web/src/generated/wasm/whichride_wasm.js`, `whichride_wasm_bg.wasm`, `whichride_wasm.d.ts` and `web/src/generated/catalogue.json` exist.

- [ ] **Step 3: Write the failing tests**

`web/test/setup.ts`:

```ts
import '@testing-library/jest-dom/vitest';
import { readFileSync } from 'node:fs';
import { initSync } from '../src/generated/wasm/whichride_wasm.js';

// The page loads the WASM over fetch; tests load the same file from disk. init() is a no-op after this.
initSync({ module: readFileSync(new URL('../src/generated/wasm/whichride_wasm_bg.wasm', import.meta.url)) });

// jsdom may lack the modal API; the browser's behaviour that matters here is `open`.
if (typeof HTMLDialogElement !== 'undefined' && !HTMLDialogElement.prototype.showModal) {
  HTMLDialogElement.prototype.showModal = function (this: HTMLDialogElement) {
    this.open = true;
  };
  HTMLDialogElement.prototype.close = function (this: HTMLDialogElement) {
    this.open = false;
    this.dispatchEvent(new Event('close'));
  };
}
```

`web/test/engine.test.ts`:

```ts
import { readFileSync } from 'node:fs';
import { seedEngine, serverEngine } from '../src/lib/engine';

const SEED = readFileSync(new URL('../src/generated/catalogue.json', import.meta.url), 'utf8');
const TRIP = [7_400, 22 * 60] as const;

function respond(body: string, status = 200): typeof fetch {
  return vi.fn(async () => new Response(body, { status })) as unknown as typeof fetch;
}

test('the seed engine prices every offer, cheapest first', async () => {
  const engine = await seedEngine();
  const offers = engine.offers(...TRIP);
  const count = engine.providers().reduce((n, p) => n + p.offers.length, 0);
  expect(offers).toHaveLength(count);
  expect(offers.map((o) => o.totalCents)).toEqual([...offers.map((o) => o.totalCents)].sort((a, b) => a - b));
  expect(Number.isInteger(offers[0].totalCents)).toBe(true);
});

test('options rank only the ticked offers', async () => {
  const engine = await seedEngine();
  const ranking = engine.options(...TRIP, ['miles.s', 'bolt.vw-polo', 'gone.car']);
  expect(ranking.ranked.map((o) => o.offerId).sort()).toEqual(['bolt.vw-polo', 'miles.s']);
  expect(ranking.cheaperNotAdded.every((o) => o.totalCents < ranking.ranked[0].totalCents)).toBe(true);
});

test('the server engine reads /v1/catalogue', async () => {
  const fetchFn = respond(JSON.stringify({ catalogue: JSON.parse(SEED), offers: [], proposals: [] }));
  const engine = await serverEngine(fetchFn);
  expect(fetchFn).toHaveBeenCalledWith('/v1/catalogue', expect.objectContaining({ signal: expect.any(AbortSignal) }));
  expect(engine.offers(...TRIP)).toEqual((await seedEngine()).offers(...TRIP));
});

test('the server engine rejects errors and invalid bodies', async () => {
  await expect(serverEngine(respond('{}', 503))).rejects.toThrow();
  await expect(serverEngine(respond('<html>'))).rejects.toThrow();
  await expect(serverEngine(respond('{"catalogue": {"providers": 1}}'))).rejects.toThrow();
});
```

`web/test/versions.test.ts`:

```ts
import { readFileSync } from 'node:fs';
import { lockedVersion } from '../scripts/generate.mjs';

const root = (path: string) => readFileSync(new URL(`../../${path}`, import.meta.url), 'utf8');

test('lockedVersion reads Cargo.lock entries', () => {
  const lock = '[[package]]\nname = "wasm-bindgen"\nversion = "0.2.129"\nsource = "x"\n';
  expect(lockedVersion(lock, 'wasm-bindgen')).toBe('0.2.129');
  expect(lockedVersion(lock, 'wasm-bindgen-macro')).toBeNull();
  expect(lockedVersion(root('Cargo.lock'), 'wasm-bindgen')).toMatch(/^\d+\.\d+\.\d+$/);
});
```

(Task 9 adds a second test here that pins the Dockerfile's CLI version.)

- [ ] **Step 4: Run to see them fail**

Run: `cd web && npx vitest run test/engine.test.ts`
Expected: FAIL — `../src/lib/engine` does not exist.

- [ ] **Step 5: Implement `web/src/lib/engine.ts`**

```ts
// The Rust core in WebAssembly, with the seed catalogue built in and the server's on top.
import init, { Prices } from '../generated/wasm/whichride_wasm.js';
import wasmUrl from '../generated/wasm/whichride_wasm_bg.wasm?url';
import seed from '../generated/catalogue.json?raw';

export type Source = 'loading' | 'server' | 'built-in';
export interface OfferInfo { id: string; name: string }
export interface ProviderInfo { id: string; name: string; offers: OfferInfo[] }
export interface Priced { providerId: string; offerId: string; offerName: string; cardLabel: string; totalCents: number }
export interface Ranking { ranked: Priced[]; cheaperNotAdded: Priced[] }
export interface Engine {
  providers(): ProviderInfo[];
  offers(metres: number, seconds: number): Priced[];
  options(metres: number, seconds: number, offerIds: string[]): Ranking;
}

/** A slow server must never hold the page; the seed is already showing. */
export const CATALOGUE_TIMEOUT_MS = 10_000;

function wrap(prices: Prices): Engine {
  return {
    providers: () => prices.providers() as ProviderInfo[],
    offers: (metres, seconds) => prices.offers(metres, seconds) as Priced[],
    options: (metres, seconds, offerIds) => prices.options(metres, seconds, offerIds) as Ranking,
  };
}

export async function seedEngine(): Promise<Engine> {
  await init({ module_or_path: wasmUrl });
  return wrap(Prices.fromSeed(seed));
}

/** Prices from `GET /v1/catalogue` (same origin). Rejects on any failure. */
export async function serverEngine(fetchFn: typeof fetch = fetch): Promise<Engine> {
  await init({ module_or_path: wasmUrl });
  const response = await fetchFn('/v1/catalogue', {
    headers: { accept: 'application/json' },
    signal: AbortSignal.timeout(CATALOGUE_TIMEOUT_MS),
  });
  if (!response.ok) throw new Error(`catalogue: HTTP ${response.status}`);
  return wrap(Prices.fromServer(await response.text()));
}
```

- [ ] **Step 6: Run the tests**

Run: `cd web && npx vitest run`
Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add web/package.json web/package-lock.json web/.nvmrc web/.gitignore web/astro.config.mjs web/tsconfig.json web/vitest.config.ts web/scripts/generate.mjs web/src/lib/engine.ts web/test
git commit -m "feat(web): Astro project, WASM generate script and price engine

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Strings — the app's `strings.xml` plus web-only JSON

**Files:**
- Create: `web/src/i18n/format.ts` (client-safe)
- Create: `web/src/i18n/android.ts` (build-time, Node `fs`)
- Create: `web/src/i18n/web.ts`, `web/src/i18n/{en,it,de,tr}.json`
- Test: `web/test/strings.test.ts`

**Interfaces:**
- Produces (`format.ts`): `LANGS = ['en','it','de','tr'] as const`; `type Lang`; `LANGUAGE_NAMES: Record<Lang, string>`; `type Plural = { other: string } & Partial<Record<Intl.LDMLPluralRule, string>>`; `type Message = string | Plural`; `type Messages = Record<string, Message>`; `format(message: Message | undefined, lang: Lang, ...args: (string | number)[]): string`; `orList(names: string[], messages: Messages, lang: Lang): string`; `accented(title: string, accent: string): [before: string, accent: string, after: string]`.
- Produces (`android.ts`): `SHARED_KEYS` (readonly tuple); `unescapeAndroid(raw: string): string`; `toPositional(text: string): string`; `parseStrings(xml: string): Map<string, Message>`; `loadShared(lang: Lang, resDir?: string): Messages`; `RES_DIR`.
- Produces (`web.ts`): `type WebStrings`; `WEB: Record<Lang, WebStrings>`.

- [ ] **Step 1: Write the failing tests** (`web/test/strings.test.ts`)

```ts
import { readFileSync } from 'node:fs';
import { LANGS, accented, format, orList } from '../src/i18n/format';
import { RES_DIR, SHARED_KEYS, loadShared, parseStrings, toPositional, unescapeAndroid } from '../src/i18n/android';
import { WEB } from '../src/i18n/web';

const placeholders = (text: string) => [...text.matchAll(/\{\d+\}/g)].map((m) => m[0]).sort();
const forms = (message: unknown) => (typeof message === 'string' ? { other: message } : (message as Record<string, string>));

test('Android escapes are undone', () => {
  expect(unescapeAndroid(String.raw`We\'ll`)).toBe("We'll");
  expect(unescapeAndroid(String.raw`say \"hi\" \@home \?x`)).toBe('say "hi" @home ?x');
  expect(unescapeAndroid(String.raw`a\nb\tc\\d`)).toBe('a\nb\tc\\d');
  expect(unescapeAndroid(String.raw`è caro`)).toBe('è caro');
  expect(unescapeAndroid('"Don\'t quote me"')).toBe("Don't quote me");
});

test('placeholders become positional', () => {
  expect(toPositional('%1$d cars, from %2$s (%3$s)')).toBe('{1} cars, from {2} ({3})');
  expect(toPositional('Add %d cars')).toBe('Add {1} cars');
  expect(toPositional('%s or %s, 100%%')).toBe('{1} or {2}, 100%');
});

test('strings and plurals are parsed', () => {
  const xml = `<resources><string name="a">It\\'s %1$s</string>
    <plurals name="p"><item quantity="one">%d car</item><item quantity="other">%d cars</item></plurals></resources>`;
  const parsed = parseStrings(xml);
  expect(parsed.get('a')).toBe("It's {1}");
  expect(parsed.get('p')).toEqual({ one: '{1} car', other: '{1} cars' });
});

test('format fills placeholders and picks plurals by the first number', () => {
  expect(format('{2} then {1}', 'en', 'a', 'b')).toBe('b then a');
  const plural = { one: '{1} car', other: '{1} cars' };
  expect(format(plural, 'en', 1)).toBe('1 car');
  expect(format(plural, 'en', 3)).toBe('3 cars');
  expect(format({ other: '{1} araç' }, 'tr', 1)).toBe('1 araç');
  expect(() => format(undefined, 'en')).toThrow();
});

test('provider lists read like the app', () => {
  const en = loadShared('en');
  expect(orList([], en, 'en')).toBe('');
  expect(orList(['MILES'], en, 'en')).toBe('MILES');
  expect(orList(['MILES', 'Bolt Drive', 'SIXT share'], en, 'en')).toBe('MILES, Bolt Drive or SIXT share');
});

test('accented splits at the last occurrence', () => {
  expect(accented('Which cars are near you?', 'near you?')).toEqual(['Which cars are ', 'near you?', '']);
  expect(accented('No accent', 'zzz')).toEqual(['No accent', '', '']);
});

test.each(LANGS)('every shared key exists in %s with English placeholders', (lang) => {
  const en = loadShared('en');
  const messages = loadShared(lang);
  for (const key of SHARED_KEYS) {
    expect(messages[key], `${lang}:${key}`).toBeDefined();
    const english = placeholders(forms(en[key]).other);
    const translated = forms(messages[key]);
    expect(placeholders(translated.other), `${lang}:${key} other`).toEqual(english);
    for (const text of Object.values(translated)) {
      for (const p of placeholders(text)) expect(english, `${lang}:${key}`).toContain(p);
      expect(text, `${lang}:${key}`).not.toMatch(/%\d*\$?[sd]|\\['"@?n]/);
    }
  }
});

test.each(LANGS)('web-only strings in %s have every key, none empty', (lang) => {
  expect(Object.keys(WEB[lang]).sort()).toEqual(Object.keys(WEB.en).sort());
  for (const [key, text] of Object.entries(WEB[lang])) expect(text.trim(), `${lang}:${key}`).not.toBe('');
  expect(WEB[lang].heading).toContain(WEB[lang].heading_accent);
});

test('the km and minute limits match the app', () => {
  const units = readFileSync(new URL('../../android/app/src/main/kotlin/io/github/locadani/whichride/format/Units.kt', import.meta.url), 'utf8');
  expect(units).toContain('MAX_METRES = 2_000_000u');
  expect(units).toContain('MIN_METRES = 100u');
  expect(units).toContain('MAX_MINUTES = 1_440u');
  const en = loadShared('en');
  expect(en.manual_km_invalid).toBe('Between 0.1 and 2000 km');
  expect(en.manual_minutes_invalid).toBe('Between 1 and 1440 minutes');
  expect(RES_DIR).toMatch(/android\/app\/src\/main\/res$/);
});
```

- [ ] **Step 2: Run to see it fail**

Run: `cd web && npx vitest run test/strings.test.ts`
Expected: FAIL — modules missing.

- [ ] **Step 3: Implement `format.ts`**

```ts
// Message formatting shared by the build and the island; no Node APIs here.
export const LANGS = ['en', 'it', 'de', 'tr'] as const;
export type Lang = (typeof LANGS)[number];

/** Each language by its own name, the same on every page. */
export const LANGUAGE_NAMES: Record<Lang, string> = { en: 'English', it: 'Italiano', de: 'Deutsch', tr: 'Türkçe' };

export type Plural = { other: string } & Partial<Record<Intl.LDMLPluralRule, string>>;
export type Message = string | Plural;
export type Messages = Record<string, Message>;

/** Fills `{1}`, `{2}`… with `args`; a plural picks its form by the first argument. */
export function format(message: Message | undefined, lang: Lang, ...args: (string | number)[]): string {
  if (message === undefined) throw new Error('missing message');
  const text =
    typeof message === 'string' ? message : (message[new Intl.PluralRules(lang).select(Number(args[0]))] ?? message.other);
  return text.replace(/\{(\d+)\}/g, (whole, n: string) => {
    const value = args[Number(n) - 1];
    return value === undefined ? whole : String(value);
  });
}

/** "MILES, Bolt Drive or SIXT share", as the app's `orList`. */
export function orList(names: string[], messages: Messages, lang: Lang): string {
  if (names.length <= 1) return names[0] ?? '';
  return format(messages.list_or, lang, names.slice(0, -1).join(', '), names[names.length - 1]);
}

/** A title split around its accented part (the app's `accented`). */
export function accented(title: string, accent: string): [string, string, string] {
  const at = accent ? title.lastIndexOf(accent) : -1;
  if (at < 0) return [title, '', ''];
  return [title.slice(0, at), accent, title.slice(at + accent.length)];
}
```

- [ ] **Step 4: Implement `android.ts`**

```ts
// Build time only: reads the wording the site shares with the app from the app's strings.xml.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { XMLParser } from 'fast-xml-parser';
import type { Lang, Message, Messages, Plural } from './format';

/** Relative to web/ (the working directory of astro, vitest and npm scripts). */
export const RES_DIR = resolve(process.cwd(), '../android/app/src/main/res');

export const SHARED_KEYS = [
  'intro_title', 'intro_title_accent', 'intro_body', 'list_or', 'add_cars', 'add_more_cars',
  'cheaper_not_added', 'nothing_cheaper', 'which_cars_body', 'add_n_cars', 'done',
  'cheapest_of_options', 'package_meta', 'manual_km', 'manual_minutes', 'manual_km_invalid',
  'manual_minutes_invalid', 'loading_prices', 'built_in_prices',
] as const;

/** Android resource escapes: surrounding quotes, \' \" \@ \? \n \t \\ and \uXXXX. */
export function unescapeAndroid(raw: string): string {
  let text = raw.trim();
  if (text.length >= 2 && text.startsWith('"') && text.endsWith('"') && !text.endsWith('\\"')) text = text.slice(1, -1);
  return text.replace(/\\(u[0-9a-fA-F]{4}|.)/g, (_, c: string) => {
    if (c.length === 5) return String.fromCharCode(parseInt(c.slice(1), 16));
    if (c === 'n') return '\n';
    if (c === 't') return '\t';
    return c;
  });
}

/** `%1$s`, `%2$d` → `{1}`, `{2}`; bare `%s`/`%d` numbered in order; `%%` → `%`. */
export function toPositional(text: string): string {
  let next = 0;
  return text.replace(/%%|%(?:(\d+)\$)?[sd]/g, (whole, n?: string) => {
    if (whole === '%%') return '%';
    next += 1;
    return `{${n ?? next}}`;
  });
}

const parser = new XMLParser({
  ignoreAttributes: false,
  attributeNamePrefix: '',
  parseTagValue: false,
  trimValues: false,
  isArray: (name) => ['string', 'plurals', 'item'].includes(name),
});

type Node = { name: string; '#text'?: string; quantity?: string; item?: Node[] };

export function parseStrings(xml: string): Map<string, Message> {
  const resources = (parser.parse(xml) as { resources?: { string?: Node[]; plurals?: Node[] } }).resources ?? {};
  const out = new Map<string, Message>();
  const text = (node: Node) => toPositional(unescapeAndroid(String(node['#text'] ?? '')));
  for (const node of resources.string ?? []) out.set(node.name, text(node));
  for (const node of resources.plurals ?? []) {
    const forms: Record<string, string> = {};
    for (const item of node.item ?? []) if (item.quantity) forms[item.quantity] = text(item);
    if (forms.other !== undefined) out.set(node.name, forms as Plural);
  }
  return out;
}

const cache = new Map<Lang, Messages>();

/** The allowlisted keys for one language. A missing key is a build error. */
export function loadShared(lang: Lang, resDir: string = RES_DIR): Messages {
  const hit = cache.get(lang);
  if (hit && resDir === RES_DIR) return hit;
  const folder = lang === 'en' ? 'values' : `values-${lang}`;
  const all = parseStrings(readFileSync(`${resDir}/${folder}/strings.xml`, 'utf8'));
  const messages: Messages = {};
  for (const key of SHARED_KEYS) {
    const message = all.get(key);
    if (message === undefined) throw new Error(`${folder}/strings.xml has no "${key}"`);
    messages[key] = message;
  }
  if (resDir === RES_DIR) cache.set(lang, messages);
  return messages;
}
```

If a translated `strings.xml` holds a key with inline markup (`<b>`), fast-xml-parser returns an object instead of `#text`; none of the allowlisted keys do today — the "every shared key" test fails loudly if that changes.

- [ ] **Step 5: Web-only strings**

`web/src/i18n/en.json`:

```json
{
  "title": "whichride · Which carsharing is cheapest in Berlin?",
  "description": "Type a trip's kilometres and minutes, tick the carsharing cars near you and see which one is cheapest in Berlin. Free, no sign-up.",
  "heading": "Which carsharing is cheapest in Berlin?",
  "heading_accent": "in Berlin?",
  "close": "Close",
  "languages": "Language",
  "not_affiliated": "Not affiliated with any provider",
  "source_code": "Source code",
  "noscript": "Comparing prices needs JavaScript.",
  "not_found": "Page not found"
}
```

`web/src/i18n/it.json`:

```json
{
  "title": "whichride · Quale car sharing costa meno a Berlino?",
  "description": "Inserisci chilometri e minuti del viaggio, spunta le auto in car sharing vicino a te e scopri qual è la più economica a Berlino. Gratis, senza registrazione.",
  "heading": "Quale car sharing costa meno a Berlino?",
  "heading_accent": "a Berlino?",
  "close": "Chiudi",
  "languages": "Lingua",
  "not_affiliated": "Non affiliato ad alcun operatore",
  "source_code": "Codice sorgente",
  "noscript": "Per confrontare i prezzi serve JavaScript.",
  "not_found": "Pagina non trovata"
}
```

`web/src/i18n/de.json`:

```json
{
  "title": "whichride · Welches Carsharing ist in Berlin am günstigsten?",
  "description": "Gib Kilometer und Minuten der Fahrt ein, hake die Carsharing-Autos in deiner Nähe an und sieh, welches in Berlin am günstigsten ist. Kostenlos, ohne Anmeldung.",
  "heading": "Welches Carsharing ist in Berlin am günstigsten?",
  "heading_accent": "am günstigsten?",
  "close": "Schließen",
  "languages": "Sprache",
  "not_affiliated": "Mit keinem Anbieter verbunden",
  "source_code": "Quellcode",
  "noscript": "Für den Preisvergleich wird JavaScript benötigt.",
  "not_found": "Seite nicht gefunden"
}
```

`web/src/i18n/tr.json`:

```json
{
  "title": "whichride · Berlin'de hangi araç paylaşımı en ucuz?",
  "description": "Yolculuğun kilometresini ve dakikasını yazın, yakınınızdaki paylaşımlı araçları işaretleyin ve Berlin'de hangisinin en ucuz olduğunu görün. Ücretsiz, kayıt gerekmez.",
  "heading": "Berlin'de hangi araç paylaşımı en ucuz?",
  "heading_accent": "en ucuz?",
  "close": "Kapat",
  "languages": "Dil",
  "not_affiliated": "Hiçbir sağlayıcıyla bağlantılı değildir",
  "source_code": "Kaynak kodu",
  "noscript": "Fiyatları karşılaştırmak için JavaScript gerekir.",
  "not_found": "Sayfa bulunamadı"
}
```

`web/src/i18n/web.ts`:

```ts
// Wording only the website has (heading, meta, footer…); everything shared is in strings.xml.
import en from './en.json';
import it from './it.json';
import de from './de.json';
import tr from './tr.json';
import type { Lang } from './format';

export type WebStrings = typeof en;
export const WEB: Record<Lang, WebStrings> = { en, it, de, tr };
```

- [ ] **Step 6: Run the tests**

Run: `cd web && npx vitest run test/strings.test.ts`
Expected: PASS. A failure on a translated file is a real gap in the app's strings: report it, don't paper over it in the loader.

- [ ] **Step 7: Commit**

```bash
git add web/src/i18n web/test/strings.test.ts
git commit -m "feat(web): wording from the app's strings.xml plus web-only JSON

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Trip parsing, money, browser memory

**Files:**
- Create: `web/src/lib/trip.ts`, `web/src/lib/money.ts`, `web/src/lib/memory.ts`
- Test: `web/test/trip.test.ts`, `web/test/memory.test.ts`

**Interfaces:**
- Produces:
  - `trip.ts`: `MIN_METRES = 100`, `MAX_METRES = 2_000_000`, `MAX_MINUTES = 1_440`; `parseKilometres(text: string): number | null` (metres); `parseMinutes(text: string): number | null` (**seconds**).
  - `money.ts`: `formatMoney(cents: number, lang: Lang): string`.
  - `memory.ts`: `KEY = 'whichride.trip'`; `TTL_MS = 15 * 60 * 1000`; `interface Remembered { km: string; minutes: string; offers: string[] }`; `browserStorage(): Storage | null`; `remember(storage: Storage | null, value: Remembered, now: number): void`; `recall(storage: Storage | null, now: number): Remembered | null`.

- [ ] **Step 1: Write the failing tests**

`web/test/trip.test.ts` (vectors copied from the app's `UnitsTest.kt`):

```ts
import { parseKilometres, parseMinutes } from '../src/lib/trip';
import { formatMoney } from '../src/lib/money';

test('kilometres accept a point, a comma, whole numbers and metres', () => {
  expect(parseKilometres('7.4')).toBe(7_400);
  expect(parseKilometres(' 7,4 ')).toBe(7_400);
  expect(parseKilometres('12')).toBe(12_000);
  expect(parseKilometres('12.345')).toBe(12_345);
  expect(parseKilometres(',5')).toBe(500);
  expect(parseKilometres('2000')).toBe(2_000_000);
  expect(parseKilometres('0.1')).toBe(100);
});

test.each(['', '7.4.1', '7,4,1', 'abc', '-3', '0', '0.05', '2000.001', '12.3456', '1e3', '7 4', '١٢'])(
  'kilometres refuse %j',
  (text) => expect(parseKilometres(text)).toBeNull(),
);

test('minutes are whole, bounded, returned as seconds', () => {
  expect(parseMinutes(' 22 ')).toBe(1_320);
  expect(parseMinutes('1440')).toBe(86_400);
  for (const text of ['', '0', '1441', '2.5', '-1', 'x', '00000']) expect(parseMinutes(text)).toBeNull();
});

test('money is euros in the language style', () => {
  expect(formatMoney(969, 'en')).toBe('€9.69');
  expect(formatMoney(969, 'de')).toBe('9,69 €');
  expect(formatMoney(0, 'en')).toBe('€0.00');
});
```

`web/test/memory.test.ts`:

```ts
import { KEY, TTL_MS, recall, remember } from '../src/lib/memory';

const VALUE = { km: '7.4', minutes: '22', offers: ['miles.s'] };
const NOW = 1_800_000_000_000;

beforeEach(() => localStorage.clear());

test('remembers for 15 minutes from the last change', () => {
  remember(localStorage, VALUE, NOW);
  expect(recall(localStorage, NOW + TTL_MS - 1)).toEqual(VALUE);
  expect(recall(localStorage, NOW + TTL_MS + 1)).toBeNull();
  expect(localStorage.getItem(KEY)).toBeNull();
});

test.each([
  ['not JSON', '{'],
  ['wrong shape', JSON.stringify({ km: 7, minutes: '22', offers: 'x', savedAt: NOW })],
  ['non-string offers', JSON.stringify({ ...VALUE, offers: [1], savedAt: NOW })],
  ['from the future', JSON.stringify({ ...VALUE, savedAt: NOW + 60_000 })],
  ['too long', JSON.stringify({ ...VALUE, km: 'x'.repeat(100), savedAt: NOW })],
])('drops %s', (_, raw) => {
  localStorage.setItem(KEY, raw);
  expect(recall(localStorage, NOW)).toBeNull();
  expect(localStorage.getItem(KEY)).toBeNull();
});

test('storage that throws is ignored', () => {
  const broken = {
    getItem: () => { throw new Error('denied'); },
    setItem: () => { throw new Error('quota'); },
    removeItem: () => { throw new Error('denied'); },
  } as unknown as Storage;
  expect(() => remember(broken, VALUE, NOW)).not.toThrow();
  expect(recall(broken, NOW)).toBeNull();
  expect(recall(null, NOW)).toBeNull();
});
```

- [ ] **Step 2: Run to see them fail**

Run: `cd web && npx vitest run test/trip.test.ts test/memory.test.ts`
Expected: FAIL — modules missing.

- [ ] **Step 3: Implement**

`web/src/lib/trip.ts`:

```ts
// The app's km/min rules (format/Units.kt), with integer maths.
export const MIN_METRES = 100;
export const MAX_METRES = 2_000_000;
export const MAX_MINUTES = 1_440;

const KILOMETRES = /^(\d{0,4})(?:[.,](\d{1,3}))?$/;

/** "7.4", "7,4", "12" → metres, or null when not a distance between 0.1 and 2000 km. */
export function parseKilometres(text: string): number | null {
  const match = KILOMETRES.exec(text.trim());
  if (!match) return null;
  const [, whole, fraction = ''] = match;
  if (whole === '' && fraction === '') return null;
  const metres = Number(whole || '0') * 1_000 + Number(fraction.padEnd(3, '0') || '0');
  return metres >= MIN_METRES && metres <= MAX_METRES ? metres : null;
}

/** Whole minutes → seconds, or null when not between 1 and 1440. */
export function parseMinutes(text: string): number | null {
  const trimmed = text.trim();
  if (!/^\d{1,4}$/.test(trimmed)) return null;
  const minutes = Number(trimmed);
  return minutes >= 1 && minutes <= MAX_MINUTES ? minutes * 60 : null;
}
```

`web/src/lib/money.ts`:

```ts
import type { Lang } from '../i18n/format';

const formats = new Map<Lang, Intl.NumberFormat>();

export function formatMoney(cents: number, lang: Lang): string {
  let format = formats.get(lang);
  if (!format) {
    format = new Intl.NumberFormat(lang, { style: 'currency', currency: 'EUR' });
    formats.set(lang, format);
  }
  return format.format(cents / 100);
}
```

`web/src/lib/memory.ts`:

```ts
// The trip and ticked cars, in this browser only, for 15 minutes from the last change (cars move).
export const KEY = 'whichride.trip';
export const TTL_MS = 15 * 60 * 1000;
const MAX_FIELD = 16;
const MAX_OFFERS = 200;

export interface Remembered { km: string; minutes: string; offers: string[] }

export function browserStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

export function remember(storage: Storage | null, value: Remembered, now: number): void {
  try {
    storage?.setItem(KEY, JSON.stringify({ ...value, savedAt: now }));
  } catch {
    // private mode, quota, blocked: the page works without memory
  }
}

function forget(storage: Storage) {
  try {
    storage.removeItem(KEY);
  } catch {
    // nothing to do
  }
}

const isShortString = (v: unknown): v is string => typeof v === 'string' && v.length <= MAX_FIELD;

/** What was remembered, or null (and the entry removed) when missing, expired or malformed. */
export function recall(storage: Storage | null, now: number): Remembered | null {
  if (!storage) return null;
  let raw: string | null;
  try {
    raw = storage.getItem(KEY);
  } catch {
    return null;
  }
  if (raw === null) return null;
  try {
    const v = JSON.parse(raw) as Record<string, unknown>;
    const fresh = typeof v.savedAt === 'number' && v.savedAt <= now && now - v.savedAt <= TTL_MS;
    const offers = v.offers;
    if (
      fresh &&
      isShortString(v.km) &&
      isShortString(v.minutes) &&
      Array.isArray(offers) &&
      offers.length <= MAX_OFFERS &&
      offers.every((o) => typeof o === 'string' && o.length <= 100)
    ) {
      return { km: v.km, minutes: v.minutes, offers: offers as string[] };
    }
  } catch {
    // malformed: forgotten below
  }
  forget(storage);
  return null;
}
```

- [ ] **Step 4: Run the tests**

Run: `cd web && npx vitest run test/trip.test.ts test/memory.test.ts`
Expected: PASS. (If Node's ICU prints `€9.69` differently, check `node -p "Intl.NumberFormat('en',{style:'currency',currency:'EUR'}).format(9.69)"`; Node 24 ships full ICU.)

- [ ] **Step 5: Commit**

```bash
git add web/src/lib/trip.ts web/src/lib/money.ts web/src/lib/memory.ts web/test/trip.test.ts web/test/memory.test.ts
git commit -m "feat(web): trip limits as the app, euro formatting, 15-minute browser memory

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: The Svelte island — trip, car sheet, result

**Files:**
- Create: `web/src/components/Comparison.svelte`
- Create: `web/src/components/CarSheet.svelte`
- Test: `web/test/Comparison.test.ts`

**Interfaces:**
- Consumes: `engine.ts` (Task 3), `format.ts`/`android.ts` (Task 4), `trip.ts`/`money.ts`/`memory.ts` (Task 5).
- Produces: `Comparison.svelte` props `{ lang: Lang; messages: Messages; providerNames: string[] }` where `messages` = `loadShared(lang)` plus `close` (web-only). Used by Task 7 with `client:load`.
- `CarSheet.svelte` props `{ open: boolean; providers: ProviderInfo[]; prices: Map<string, Priced>; picked: string[]; lang: Lang; t: (key: string, ...args: (string | number)[]) => string; onCommit: (ids: string[]) => void; onCancel: () => void }`.

- [ ] **Step 1: Write the failing tests** (`web/test/Comparison.test.ts`)

```ts
import { fireEvent, render, screen, within } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { readFileSync } from 'node:fs';
import Comparison from '../src/components/Comparison.svelte';
import { loadShared } from '../src/i18n/android';
import { format, orList } from '../src/i18n/format';
import { seedEngine } from '../src/lib/engine';
import { formatMoney } from '../src/lib/money';
import { KEY, TTL_MS } from '../src/lib/memory';

const SEED = readFileSync(new URL('../src/generated/catalogue.json', import.meta.url), 'utf8');
const messages = { ...loadShared('en'), close: 'Close' };
const providerNames = (JSON.parse(SEED) as { providers: { name: string }[] }).providers.map((p) => p.name);
const TRIP = [7_400, 1_320] as const;

function offline() {
  vi.stubGlobal('fetch', vi.fn(async () => { throw new TypeError('offline'); }));
}

beforeEach(() => {
  localStorage.clear();
  offline();
});
afterEach(() => vi.unstubAllGlobals());

async function page() {
  const user = userEvent.setup();
  render(Comparison, { lang: 'en', messages, providerNames });
  await screen.findByText('built-in prices');
  return user;
}

async function typeTrip(user: ReturnType<typeof userEvent.setup>, km = '7.4', minutes = '22') {
  await user.type(screen.getByLabelText('Distance (km)'), km);
  await user.type(screen.getByLabelText('Time (min)'), minutes);
}

function tick(dialog: HTMLElement, offerId: string) {
  const box = dialog.querySelector<HTMLInputElement>(`input[value="${offerId}"]`);
  if (!box) throw new Error(`no checkbox for ${offerId}`);
  return box;
}

test('a first visit asks for the cars near you', async () => {
  await page();
  expect(screen.getByRole('heading', { name: 'Which cars are near you?' })).toBeInTheDocument();
  expect(screen.getByText(format(messages.intro_body, 'en', orList(providerNames, messages, 'en')))).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Add the cars near you' })).toBeInTheDocument();
  expect(screen.queryByRole('list')).not.toBeInTheDocument();
});

test('ticked cars are ranked cheapest first, as the core ranks them', async () => {
  const user = await page();
  await typeTrip(user);
  await user.click(screen.getByRole('button', { name: 'Add the cars near you' }));
  const dialog = screen.getByRole('dialog');
  await user.click(tick(dialog, 'miles.s'));
  await user.click(tick(dialog, 'bolt.vw-polo'));
  await user.click(within(dialog).getByRole('button', { name: 'Add 2 cars' }));

  const expected = (await seedEngine()).options(...TRIP, ['miles.s', 'bolt.vw-polo']);
  const rows = within(screen.getByRole('list')).getAllByRole('listitem');
  expect(rows).toHaveLength(2);
  expected.ranked.forEach((offer, i) => {
    expect(rows[i]).toHaveTextContent(offer.offerName);
    expect(rows[i]).toHaveTextContent(formatMoney(offer.totalCents, 'en'));
  });
  expect(rows[0]).toHaveTextContent('Cheapest of your options');
  expect(rows[1]).not.toHaveTextContent('Cheapest of your options');
  const cheaper = expected.cheaperNotAdded;
  const providers = (await seedEngine()).providers();
  const line = cheaper.length
    ? format(messages.cheaper_not_added, 'en', cheaper.length, formatMoney(cheaper[0].totalCents, 'en'),
        `${providers.find((p) => p.id === cheaper[0].providerId)!.name} ${cheaper[0].offerName}`)
    : 'No car you did not add is cheaper.';
  expect(screen.getByText(line)).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Add more cars' })).toHaveFocus();
});

test('the sheet button reads Done when nothing is ticked', async () => {
  const user = await page();
  await user.click(screen.getByRole('button', { name: 'Add the cars near you' }));
  const dialog = screen.getByRole('dialog');
  expect(within(dialog).getByRole('button', { name: 'Done' })).toBeInTheDocument();
  await user.click(tick(dialog, 'miles.s'));
  expect(within(dialog).getByRole('button', { name: 'Add 1 car' })).toBeInTheDocument();
});

test('closing the sheet drops its ticks and returns focus', async () => {
  const user = await page();
  const opener = screen.getByRole('button', { name: 'Add the cars near you' });
  await user.click(opener);
  const dialog = screen.getByRole('dialog');
  await user.click(tick(dialog, 'miles.s'));
  await fireEvent(dialog, new Event('cancel', { cancelable: true }));
  expect(dialog).not.toHaveAttribute('open');
  expect(screen.queryByRole('list')).not.toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Add the cars near you' })).toHaveFocus();

  await user.click(screen.getByRole('button', { name: 'Add the cars near you' }));
  expect(dialog).toHaveAttribute('open');
  expect(tick(dialog, 'miles.s')).not.toBeChecked();
  await user.click(within(dialog).getByRole('button', { name: 'Close' }));
  expect(dialog).not.toHaveAttribute('open');
});

test('an invalid trip lists the cars without prices', async () => {
  const user = await page();
  await typeTrip(user, 'abc', '22');
  const km = screen.getByLabelText('Distance (km)');
  expect(km).toHaveAttribute('aria-invalid', 'true');
  expect(screen.getByText('Between 0.1 and 2000 km')).toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Add the cars near you' }));
  const dialog = screen.getByRole('dialog');
  expect(dialog).not.toHaveTextContent('€');
  await user.click(tick(dialog, 'miles.s'));
  await user.click(within(dialog).getByRole('button', { name: 'Add 1 car' }));
  const rows = within(screen.getByRole('list')).getAllByRole('listitem');
  expect(rows).toHaveLength(1);
  expect(rows[0]).not.toHaveTextContent('€');
  expect(rows[0]).not.toHaveTextContent('Cheapest of your options');
  expect(screen.queryByText(/did not add/)).not.toBeInTheDocument();
});

test('an empty field shows no error', async () => {
  await page();
  expect(screen.getByLabelText('Distance (km)')).not.toHaveAttribute('aria-invalid', 'true');
  expect(screen.queryByText('Between 0.1 and 2000 km')).not.toBeInTheDocument();
});

test('server prices replace the built-in ones', async () => {
  const catalogue = JSON.parse(SEED);
  for (const card of catalogue.providers[0].offers[0].cards) card.per_km = 1;
  const body = JSON.stringify({ catalogue, offers: [], proposals: [] });
  vi.stubGlobal('fetch', vi.fn(async () => new Response(body)));
  localStorage.setItem(KEY, JSON.stringify({ km: '7.4', minutes: '22', offers: [catalogue.providers[0].offers[0].id], savedAt: Date.now() }));
  render(Comparison, { lang: 'en', messages, providerNames });
  const { Prices } = await import('../src/generated/wasm/whichride_wasm.js');
  const server = Prices.fromServer(body).options(...TRIP, [catalogue.providers[0].offers[0].id]);
  const row = await screen.findByText(formatMoney(server.ranked[0].totalCents, 'en'));
  expect(row).toBeInTheDocument();
  expect(screen.queryByText('built-in prices')).not.toBeInTheDocument();
  expect(screen.queryByText('Loading prices…')).not.toBeInTheDocument();
});

test('built-in prices when the server answers an error', async () => {
  vi.stubGlobal('fetch', vi.fn(async () => new Response('{}', { status: 503 })));
  render(Comparison, { lang: 'en', messages, providerNames });
  expect(await screen.findByText('built-in prices')).toBeInTheDocument();
});

test('the trip and cars come back within 15 minutes', async () => {
  localStorage.setItem(KEY, JSON.stringify({ km: '7.4', minutes: '22', offers: ['miles.s'], savedAt: Date.now() - 60_000 }));
  await page();
  expect(screen.getByLabelText('Distance (km)')).toHaveValue('7.4');
  expect(within(screen.getByRole('list')).getAllByRole('listitem')).toHaveLength(1);
});

test('remembered cars that no longer exist are ignored', async () => {
  localStorage.setItem(KEY, JSON.stringify({ km: '7.4', minutes: '22', offers: ['gone.car', 'miles.s'], savedAt: Date.now() }));
  const user = await page();
  expect(within(screen.getByRole('list')).getAllByRole('listitem')).toHaveLength(1);
  await user.click(screen.getByRole('button', { name: 'Add more cars' }));
  expect(within(screen.getByRole('dialog')).getByRole('button', { name: 'Add 1 car' })).toBeInTheDocument();
});

test('memory older than 15 minutes is not used', async () => {
  localStorage.setItem(KEY, JSON.stringify({ km: '7.4', minutes: '22', offers: ['miles.s'], savedAt: Date.now() - TTL_MS - 1 }));
  await page();
  expect(screen.getByLabelText('Distance (km)')).toHaveValue('');
  expect(screen.getByRole('button', { name: 'Add the cars near you' })).toBeInTheDocument();
});

test('changes are remembered', async () => {
  const user = await page();
  await typeTrip(user);
  const saved = JSON.parse(localStorage.getItem(KEY) ?? '{}');
  expect(saved).toMatchObject({ km: '7.4', minutes: '22', offers: [] });
});
```

jsdom may keep a closed `<dialog>` in the accessibility tree; the tests therefore check the `open` attribute, not whether the dialog role is found.

- [ ] **Step 2: Run to see them fail**

Run: `cd web && npx vitest run test/Comparison.test.ts`
Expected: FAIL — components missing.

- [ ] **Step 3: Implement `CarSheet.svelte`**

```svelte
<script lang="ts">
  import type { Lang } from '../i18n/format';
  import type { Priced, ProviderInfo } from '../lib/engine';
  import { formatMoney } from '../lib/money';

  let {
    open, providers, prices, picked, lang, t, onCommit, onCancel,
  }: {
    open: boolean;
    providers: ProviderInfo[];
    prices: Map<string, Priced>;
    picked: string[];
    lang: Lang;
    t: (key: string, ...args: (string | number)[]) => string;
    onCommit: (ids: string[]) => void;
    onCancel: () => void;
  } = $props();

  let dialog: HTMLDialogElement | undefined = $state();
  // The ticks are a draft until the button applies them (as the app's sheet).
  let draft: string[] = $state([]);

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      draft = [...picked];
      dialog.showModal();
    } else if (!open && dialog.open) {
      dialog.close();
    }
  });

  function toggle(id: string, on: boolean) {
    draft = on ? [...draft, id] : draft.filter((d) => d !== id);
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="sheet-title"
  aria-describedby="sheet-body"
  oncancel={(e) => { e.preventDefault(); onCancel(); }}
>
  <div class="head">
    <h2 id="sheet-title">{t('intro_title')}</h2>
    <button type="button" class="close" aria-label={t('close')} onclick={onCancel}>✕</button>
  </div>
  <p id="sheet-body" class="body">{t('which_cars_body')}</p>
  {#each providers as provider (provider.id)}
    <fieldset>
      <legend>{provider.name}</legend>
      {#each provider.offers as offer (offer.id)}
        {@const price = prices.get(offer.id)}
        <label class="car">
          <input
            type="checkbox"
            value={offer.id}
            checked={draft.includes(offer.id)}
            onchange={(e) => toggle(offer.id, e.currentTarget.checked)}
          />
          <span class="name">{offer.name}</span>
          {#if price}<span class="price">{formatMoney(price.totalCents, lang)}</span>{/if}
        </label>
      {/each}
    </fieldset>
  {/each}
  <button type="button" class="primary" onclick={() => onCommit(draft)}>
    {draft.length ? t('add_n_cars', draft.length) : t('done')}
  </button>
</dialog>

<style>
  dialog {
    inline-size: 100%;
    max-inline-size: var(--wr-size-content-max);
    max-block-size: 85dvh;
    margin-block-end: 0;
    padding: var(--wr-space-md);
    border: none;
    border-start-start-radius: var(--wr-radius-xxl);
    border-start-end-radius: var(--wr-radius-xxl);
    background: var(--wr-color-surface);
    color: var(--wr-color-on-surface);
  }
  dialog::backdrop { background: var(--wr-color-scrim); opacity: 0.7; }
  .head { display: flex; justify-content: space-between; align-items: center; gap: var(--wr-space-sm); }
  h2 { margin: 0; font: var(--wr-type-headline-small); letter-spacing: var(--wr-type-headline-small-tracking); }
  .body { margin-block: var(--wr-space-xs) var(--wr-space-md); font: var(--wr-type-body-medium); color: var(--wr-color-on-surface-muted); }
  .close {
    inline-size: var(--wr-size-touch-target);
    block-size: var(--wr-size-touch-target);
    border: none;
    border-radius: var(--wr-radius-full);
    background: var(--wr-color-surface-high);
    color: var(--wr-color-on-surface);
    font: var(--wr-type-title-medium);
    cursor: pointer;
  }
  fieldset { margin: 0 0 var(--wr-space-md); padding: 0; border: none; display: flex; flex-direction: column; gap: var(--wr-space-xxs); }
  legend { margin-block-end: var(--wr-space-xs); font: var(--wr-type-label-small); letter-spacing: var(--wr-type-label-small-tracking); text-transform: uppercase; color: var(--wr-color-on-surface-muted); }
  .car {
    display: flex; align-items: center; gap: var(--wr-space-sm);
    min-block-size: var(--wr-size-touch-target);
    padding-inline: var(--wr-space-sm);
    border-radius: var(--wr-radius-md);
    background: var(--wr-color-surface-high);
    cursor: pointer;
  }
  .car input { inline-size: var(--wr-space-lg); block-size: var(--wr-space-lg); margin: 0; accent-color: var(--wr-color-accent); }
  .name { flex: 1; font: var(--wr-type-title-small); }
  .price { font: var(--wr-type-number-medium); font-variant-numeric: tabular-nums; }
  .primary {
    position: sticky; inset-block-end: 0;
    inline-size: 100%; min-block-size: var(--wr-size-control-lg);
    border: none; border-radius: var(--wr-radius-md);
    background: var(--wr-color-accent); color: var(--wr-color-on-accent);
    font: var(--wr-type-label-large); cursor: pointer;
  }
</style>
```

- [ ] **Step 4: Implement `Comparison.svelte`**

```svelte
<script lang="ts">
  import { onMount, tick } from 'svelte';
  import CarSheet from './CarSheet.svelte';
  import { accented, format, orList, type Lang, type Messages } from '../i18n/format';
  import type { Engine, Priced, Ranking, Source } from '../lib/engine';
  import { browserStorage, recall, remember } from '../lib/memory';
  import { formatMoney } from '../lib/money';
  import { parseKilometres, parseMinutes } from '../lib/trip';

  let { lang, messages, providerNames }: { lang: Lang; messages: Messages; providerNames: string[] } = $props();

  /** The tariff name of plain per-km/per-minute pricing (the app's STANDARD_CARD). */
  const STANDARD_CARD = 'Standard';
  const t = (key: string, ...args: (string | number)[]) => format(messages[key], lang, ...args);

  let km = $state('');
  let minutes = $state('');
  let picked: string[] = $state([]);
  let engine: Engine | null = $state(null);
  let source: Source = $state('loading');
  let sheetOpen = $state(false);
  let addCars: HTMLButtonElement | undefined = $state();
  let addMore: HTMLButtonElement | undefined = $state();

  const metres = $derived(parseKilometres(km));
  const seconds = $derived(parseMinutes(minutes));
  const kmError = $derived(km.trim() !== '' && metres === null);
  const minutesError = $derived(minutes.trim() !== '' && seconds === null);
  const providers = $derived(engine?.providers() ?? []);
  const offerInfo = $derived(
    new Map(providers.flatMap((p) => p.offers.map((o) => [o.id, { provider: p.name, name: o.name }] as const))),
  );
  const providerName = (id: string) => providers.find((p) => p.id === id)?.name ?? id;
  /** Ticked offers the catalogue still has (memory may hold removed ones). */
  const chosen = $derived(picked.filter((id) => offerInfo.has(id)));
  const prices: Map<string, Priced> = $derived(
    engine && metres !== null && seconds !== null
      ? new Map(engine.offers(metres, seconds).map((o) => [o.offerId, o]))
      : new Map(),
  );
  const ranking: Ranking | null = $derived(
    engine && metres !== null && seconds !== null && chosen.length ? engine.options(metres, seconds, chosen) : null,
  );
  const rows = $derived(
    ranking
      ? ranking.ranked.map((o) => ({
          id: o.offerId,
          provider: providerName(o.providerId),
          offer: o.offerName,
          meta: o.cardLabel === STANDARD_CARD ? null : t('package_meta', o.cardLabel),
          cents: o.totalCents as number | null,
        }))
      : chosen.map((id) => ({ id, provider: offerInfo.get(id)!.provider, offer: offerInfo.get(id)!.name, meta: null, cents: null })),
  );
  const cheaperLine = $derived.by(() => {
    const cheaper = ranking?.cheaperNotAdded ?? [];
    if (cheaper.length === 0) return t('nothing_cheaper');
    const first = cheaper[0];
    return t('cheaper_not_added', cheaper.length, formatMoney(first.totalCents, lang), `${providerName(first.providerId)} ${first.offerName}`);
  });
  const [titleBefore, titleAccent, titleAfter] = $derived(accented(t('intro_title'), t('intro_title_accent')));

  function save() {
    remember(browserStorage(), { km, minutes, offers: [...picked] }, Date.now());
  }

  async function closeSheet(commit: string[] | null) {
    if (commit) {
      picked = commit;
      save();
    }
    sheetOpen = false;
    await tick();
    (chosen.length ? addMore : addCars)?.focus();
  }

  onMount(() => {
    const saved = recall(browserStorage(), Date.now());
    if (saved) {
      km = saved.km;
      minutes = saved.minutes;
      picked = saved.offers;
    }
    let cancelled = false;
    (async () => {
      const { seedEngine, serverEngine } = await import('../lib/engine');
      engine = await seedEngine();
      try {
        const server = await serverEngine();
        if (!cancelled) {
          engine = server;
          source = 'server';
        }
      } catch {
        if (!cancelled) source = 'built-in';
      }
    })();
    return () => {
      cancelled = true;
    };
  });
</script>

<div class="trip">
  <label class="field">
    <span>{t('manual_km')}</span>
    <input
      inputmode="decimal"
      autocomplete="off"
      value={km}
      aria-invalid={kmError}
      aria-describedby={kmError ? 'km-error' : undefined}
      oninput={(e) => { km = e.currentTarget.value; save(); }}
    />
    {#if kmError}<span id="km-error" class="error">{t('manual_km_invalid')}</span>{/if}
  </label>
  <label class="field">
    <span>{t('manual_minutes')}</span>
    <input
      inputmode="numeric"
      autocomplete="off"
      value={minutes}
      aria-invalid={minutesError}
      aria-describedby={minutesError ? 'minutes-error' : undefined}
      oninput={(e) => { minutes = e.currentTarget.value; save(); }}
    />
    {#if minutesError}<span id="minutes-error" class="error">{t('manual_minutes_invalid')}</span>{/if}
  </label>
</div>

<section class="result" aria-live="polite">
  {#if chosen.length === 0}
    <div class="intro">
      <h2>{titleBefore}<span class="accent">{titleAccent}</span>{titleAfter}</h2>
      <p>{t('intro_body', orList(providerNames, messages, lang))}</p>
      <button type="button" class="primary" bind:this={addCars} onclick={() => (sheetOpen = true)}>{t('add_cars')}</button>
    </div>
  {:else}
    <ol class="options">
      {#each rows as row, i (row.id)}
        <li class={['option', ranking && i === 0 && 'best']}>
          <div class="name">
            {#if ranking && i === 0}<span class="badge">{t('cheapest_of_options')}</span>{/if}
            <span class="provider">{row.provider}</span>
            <span class="meta">{row.offer}{row.meta ? ` · ${row.meta}` : ''}</span>
          </div>
          {#if row.cents !== null}<span class="price">{formatMoney(row.cents, lang)}</span>{/if}
        </li>
      {/each}
    </ol>
    {#if ranking}<p class="cheaper">{cheaperLine}</p>{/if}
    <button type="button" class="secondary" bind:this={addMore} onclick={() => (sheetOpen = true)}>{t('add_more_cars')}</button>
  {/if}
  {#if source !== 'server'}
    <p class="source">{t(source === 'loading' ? 'loading_prices' : 'built_in_prices')}</p>
  {/if}
</section>

<CarSheet
  open={sheetOpen}
  {providers}
  {prices}
  picked={chosen}
  {lang}
  {t}
  onCommit={(ids) => closeSheet(ids)}
  onCancel={() => closeSheet(null)}
/>

<style>
  .trip { display: grid; grid-template-columns: 1fr 1fr; gap: var(--wr-space-sm); margin-block-end: var(--wr-space-lg); }
  .field { display: flex; flex-direction: column; gap: var(--wr-space-xxs); font: var(--wr-type-label-medium); color: var(--wr-color-on-surface-muted); }
  .field input {
    min-inline-size: 0;
    min-block-size: var(--wr-size-control-md);
    padding-inline: var(--wr-space-sm);
    border: var(--wr-size-stroke) solid var(--wr-color-outline);
    border-radius: var(--wr-radius-md);
    background: var(--wr-color-surface);
    color: var(--wr-color-on-surface);
    font: var(--wr-type-number-medium);
  }
  .field input[aria-invalid='true'] { border-color: var(--wr-color-error); }
  .error { font: var(--wr-type-body-small); color: var(--wr-color-error); }
  .result { display: flex; flex-direction: column; gap: var(--wr-space-sm); }
  .intro { display: flex; flex-direction: column; gap: var(--wr-space-sm); padding: var(--wr-space-lg); border-radius: var(--wr-radius-lg); background: var(--wr-color-surface); }
  .intro h2 { margin: 0; font: var(--wr-type-headline-small); letter-spacing: var(--wr-type-headline-small-tracking); }
  .intro p { margin: 0; font: var(--wr-type-body-medium); color: var(--wr-color-on-surface-muted); }
  .options { margin: 0; padding: 0; list-style: none; display: flex; flex-direction: column; gap: var(--wr-space-xs); }
  .option { display: flex; justify-content: space-between; align-items: center; gap: var(--wr-space-sm); padding: var(--wr-space-sm) var(--wr-space-md); border-radius: var(--wr-radius-md); background: var(--wr-color-surface); }
  .option.best { padding-block: var(--wr-space-md); background: var(--wr-color-hero); color: var(--wr-color-on-hero); }
  .name { display: flex; flex-direction: column; gap: var(--wr-space-xxs); }
  .badge { font: var(--wr-type-label-small); letter-spacing: var(--wr-type-label-small-tracking); text-transform: uppercase; }
  .provider { font: var(--wr-type-title-medium); }
  .meta { font: var(--wr-type-body-small); }
  .price { font: var(--wr-type-number-medium); font-variant-numeric: tabular-nums; }
  .best .price { font: var(--wr-type-number-large); letter-spacing: var(--wr-type-number-large-tracking); }
  .cheaper, .source { margin: 0; font: var(--wr-type-body-small); color: var(--wr-color-on-surface-muted); }
  .primary, .secondary { min-block-size: var(--wr-size-control-md); padding-inline: var(--wr-space-lg); border-radius: var(--wr-radius-md); font: var(--wr-type-label-large); cursor: pointer; }
  .primary { border: none; background: var(--wr-color-accent); color: var(--wr-color-on-accent); }
  .secondary { align-self: flex-start; border: var(--wr-size-stroke) solid var(--wr-color-outline); background: none; color: var(--wr-color-on-surface); }
</style>
```

Notes for the implementer:
- `.accent` is a global class from `site.css` (Task 7); the island does not style it.
- Don't add `style=` attributes or `style:` directives: the CSP (Task 7) has no `'unsafe-inline'` for styles.
- `class={[…]}` arrays need Svelte ≥ 5.16 (pinned 5.57.2).

- [ ] **Step 5: Run the tests and the guard**

Run: `cd web && npx vitest run && cd ../design && npm test`
Expected: all web tests PASS; the design single-source guard PASSES (no literal lengths/colours in the new `.svelte` files).

- [ ] **Step 6: Commit**

```bash
git add web/src/components web/test/Comparison.test.ts
git commit -m "feat(web): comparison island with car sheet, ranking and browser memory

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Astro pages — layout, SEO, CSP, 404, robots, sitemap, compression

**Files:**
- Modify: `web/astro.config.mjs` (`security.csp`)
- Create: `web/src/styles/site.css`
- Create: `web/src/layouts/Page.astro`
- Create: `web/src/pages/[lang]/index.astro`, `web/src/pages/404.astro`, `web/src/pages/robots.txt.ts`, `web/src/pages/sitemap.xml.ts`
- Create: `web/scripts/compress.mjs`
- Create: `web/vitest.dist.config.ts`, `web/test-dist/dist.test.ts`

**Interfaces:**
- Consumes: `Comparison.svelte` (Task 6), `WEB`, `loadShared`, `LANGS`, `LANGUAGE_NAMES`, `accented` (Task 4), `DEFAULT_SITE` (Task 3), `--wr-size-content-max` (Task 1).
- Produces: `web/dist/{en,it,de,tr}/index.html`, `dist/404.html`, `dist/robots.txt`, `dist/sitemap.xml`, `dist/_astro/*` (hashed JS/CSS/WASM), `.br`/`.gz` siblings for text files ≥ 1 KiB, WASM and fonts. Task 8 serves this directory.

- [ ] **Step 1: Write the failing dist tests**

`web/vitest.dist.config.ts`:

```ts
import { defineConfig } from 'vitest/config';

// Checks the built site (npm run build first).
export default defineConfig({ test: { include: ['test-dist/**/*.test.ts'], environment: 'node' } });
```

`web/test-dist/dist.test.ts`:

```ts
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { DEFAULT_SITE } from '../astro.config.mjs';

const DIST = new URL('../dist/', import.meta.url);
const read = (path: string) => readFileSync(new URL(path, DIST), 'utf8');
const SITE = `${process.env.WHICHRIDE_SITE_URL || DEFAULT_SITE}/`;
const LANGS = ['en', 'it', 'de', 'tr'];
const visible = (html: string) => html.replace(/<script[\s\S]*?<\/script>/g, '').replace(/<style[\s\S]*?<\/style>/g, '');

describe.each(LANGS)('/%s/', (lang) => {
  const html = read(`${lang}/index.html`);

  test('declares its language, canonical URL and alternates', () => {
    expect(html).toContain(`<html lang="${lang}"`);
    expect(html).toContain(`<link rel="canonical" href="${SITE}${lang}/"`);
    for (const l of LANGS) expect(html).toContain(`<link rel="alternate" hreflang="${l}" href="${SITE}${l}/"`);
    expect(html).toContain(`<link rel="alternate" hreflang="x-default" href="${SITE}en/"`);
    expect(html).toMatch(/<meta name="description" content="[^"]{50,}"/);
  });

  test('has a CSP that allows the WebAssembly and only this origin', () => {
    const csp = html.match(/<meta http-equiv="content-security-policy" content="([^"]+)"/i)?.[1] ?? '';
    expect(csp).toContain("'wasm-unsafe-eval'");
    expect(csp).toContain("default-src 'self'");
    expect(csp).not.toContain('unsafe-inline');
    expect(html).not.toMatch(/<(script|link)[^>]+(src|href)="https?:/);
  });

  test('renders the first visit with no raw placeholders', () => {
    const text = visible(html);
    expect(text).not.toMatch(/%\d*\$?[sd]|\{\d+\}/);
    expect(text).toContain('<noscript>');
    expect(text).toMatch(/<h1>/);
    expect(text).toContain('astro-island');
  });
});

test('the 404 page exists and is not indexed', () => {
  const html = read('404.html');
  expect(html).toContain('<meta name="robots" content="noindex"');
  for (const l of LANGS) expect(html).toContain(`href="/${l}/"`);
});

test('robots.txt and the sitemap list the four pages', () => {
  expect(read('robots.txt')).toContain(`Sitemap: ${SITE}sitemap.xml`);
  const sitemap = read('sitemap.xml');
  for (const l of LANGS) expect(sitemap).toContain(`<loc>${SITE}${l}/</loc>`);
});

test('the WebAssembly, scripts and pages are precompressed', () => {
  const assets = readdirSync(new URL('_astro/', DIST));
  const wasm = assets.find((f) => f.endsWith('.wasm'));
  expect(wasm).toBeDefined();
  expect(existsSync(new URL(`_astro/${wasm}.br`, DIST))).toBe(true);
  expect(existsSync(new URL(`_astro/${wasm}.gz`, DIST))).toBe(true);
  expect(existsSync(new URL('en/index.html.br', DIST))).toBe(true);
});
```

Run: `cd web && npx vitest run --config vitest.dist.config.ts`
Expected: FAIL — no `dist/`.

- [ ] **Step 2: CSP in `astro.config.mjs`**

Add to the `defineConfig` object:

```js
  security: {
    csp: {
      directives: [
        "default-src 'self'",
        "img-src 'self'",
        "font-src 'self'",
        "connect-src 'self'",
        "object-src 'none'",
        "base-uri 'none'",
        "form-action 'none'",
      ],
      // The core runs as WebAssembly; hashes for Astro's own inline scripts are added by Astro.
      scriptDirective: { resources: ["'self'", "'wasm-unsafe-eval'"] },
    },
  },
```

(`frame-ancestors` cannot be set from a `<meta>`; the server sends `X-Frame-Options: DENY`, Task 8.)

- [ ] **Step 3: Global styles** (`web/src/styles/site.css`)

```css
/* Page shell; every value from tokens.css. No media queries: the column is max-inline-size. */
*, *::before, *::after { box-sizing: border-box; }
body { margin: 0; background: var(--wr-color-background); color: var(--wr-color-on-surface); font: var(--wr-type-body-large); }
.page { max-inline-size: var(--wr-size-content-max); margin-inline: auto; padding-inline: var(--wr-space-md); }
h1 { margin-block: var(--wr-space-md) var(--wr-space-lg); font: var(--wr-type-headline-large); letter-spacing: var(--wr-type-headline-large-tracking); text-wrap: balance; }
.accent { color: var(--wr-color-accent-text); }
.top { display: flex; justify-content: space-between; align-items: center; padding-block: var(--wr-space-sm); }
.brand { display: inline-flex; align-items: center; gap: var(--wr-space-xs); min-block-size: var(--wr-size-touch-target); color: inherit; text-decoration: none; font: var(--wr-type-title-large); }
.brand img { inline-size: var(--wr-size-monogram); block-size: var(--wr-size-monogram); }
.languages { position: relative; }
.languages summary {
  display: flex; align-items: center;
  min-block-size: var(--wr-size-touch-target); padding-inline: var(--wr-space-sm);
  border: var(--wr-size-stroke) solid var(--wr-color-outline-variant); border-radius: var(--wr-radius-sm);
  font: var(--wr-type-label-medium); cursor: pointer;
}
.languages ul {
  position: absolute; inset-inline-end: 0; z-index: 1;
  margin: var(--wr-space-xxs) 0 0; padding: var(--wr-space-xxs); list-style: none;
  border: var(--wr-size-stroke) solid var(--wr-color-outline-variant); border-radius: var(--wr-radius-md);
  background: var(--wr-color-surface);
}
.languages a { display: flex; align-items: center; min-block-size: var(--wr-size-touch-target); padding-inline: var(--wr-space-sm); color: inherit; }
.languages a[aria-current='page'] { font: var(--wr-type-label-large); }
.footer {
  display: flex; flex-wrap: wrap; justify-content: space-between; gap: var(--wr-space-xs);
  margin-block-start: var(--wr-space-xxl); padding-block: var(--wr-space-md);
  border-block-start: var(--wr-size-stroke) solid var(--wr-color-outline-variant);
  font: var(--wr-type-body-small); color: var(--wr-color-on-surface-muted);
}
.footer p { margin: 0; }
.footer a { color: inherit; }
:focus-visible { outline: var(--wr-size-stroke-strong) solid var(--wr-color-accent); outline-offset: var(--wr-size-stroke-strong); }
```

- [ ] **Step 4: Layout** (`web/src/layouts/Page.astro`)

```astro
---
import '../styles/tokens.css';
import '../styles/site.css';
import { LANGS, LANGUAGE_NAMES, type Lang } from '../i18n/format';
import type { WebStrings } from '../i18n/web';

interface Props {
  lang: Lang;
  web: WebStrings;
  /** A language page: canonical + hreflang alternates. The 404 page has none and is noindex. */
  indexable: boolean;
}
const { lang, web, indexable } = Astro.props;
const url = (l: Lang) => new URL(`${l}/`, Astro.site).href;
const SOURCE = 'https://github.com/locadani/whichride';
---
<!doctype html>
<html lang={lang}>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{web.title}</title>
    <meta name="description" content={web.description} />
    <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
    {indexable && <link rel="canonical" href={url(lang)} />}
    {indexable && LANGS.map((l) => <link rel="alternate" hreflang={l} href={url(l)} />)}
    {indexable && <link rel="alternate" hreflang="x-default" href={url('en')} />}
    {!indexable && <meta name="robots" content="noindex" />}
  </head>
  <body>
    <header class="top page">
      <a class="brand" href={`/${lang}/`}><img src="/favicon.svg" alt="" />whichride</a>
      <details class="languages">
        <summary aria-label={`${web.languages}: ${LANGUAGE_NAMES[lang]}`}>{lang.toUpperCase()}</summary>
        <ul>
          {LANGS.map((l) => (
            <li>
              <a href={`/${l}/`} hreflang={l} lang={l} aria-current={l === lang ? 'page' : undefined}>{LANGUAGE_NAMES[l]}</a>
            </li>
          ))}
        </ul>
      </details>
    </header>
    <main class="page"><slot /></main>
    <footer class="footer page">
      <p>{web.not_affiliated}</p>
      <a href={SOURCE}>{web.source_code}</a>
    </footer>
  </body>
</html>
```

- [ ] **Step 5: Pages**

`web/src/pages/[lang]/index.astro`:

```astro
---
import Page from '../../layouts/Page.astro';
import Comparison from '../../components/Comparison.svelte';
import { LANGS, accented, type Lang } from '../../i18n/format';
import { loadShared } from '../../i18n/android';
import { WEB } from '../../i18n/web';
import seed from '../../generated/catalogue.json';

export function getStaticPaths() {
  return LANGS.map((lang) => ({ params: { lang } }));
}

const lang = Astro.params.lang as Lang;
const web = WEB[lang];
const messages = { ...loadShared(lang), close: web.close };
// Server and seed share the providers (the server takes them from the seed), so the intro can be
// rendered before any price loads.
const providerNames = seed.providers.map((p) => p.name);
const [before, accent, after] = accented(web.heading, web.heading_accent);
---
<Page lang={lang} web={web} indexable={true}>
  <h1>{before}<span class="accent">{accent}</span>{after}</h1>
  <noscript><p>{web.noscript}</p></noscript>
  <Comparison client:load lang={lang} messages={messages} providerNames={providerNames} />
</Page>
```

`web/src/pages/404.astro`:

```astro
---
import Page from '../layouts/Page.astro';
import { LANGS, LANGUAGE_NAMES } from '../i18n/format';
import { WEB } from '../i18n/web';
// One page for every unknown path: the server can't know the language, so it says it in all four.
---
<Page lang="en" web={WEB.en} indexable={false}>
  <h1>{WEB.en.not_found}</h1>
  {LANGS.filter((l) => l !== 'en').map((l) => <p lang={l}>{WEB[l].not_found}</p>)}
  <ul>
    {LANGS.map((l) => <li><a href={`/${l}/`} hreflang={l} lang={l}>{LANGUAGE_NAMES[l]}</a></li>)}
  </ul>
</Page>
```

`web/src/pages/robots.txt.ts`:

```ts
import type { APIRoute } from 'astro';

export const GET: APIRoute = ({ site }) =>
  new Response(`User-agent: *\nAllow: /\nSitemap: ${new URL('sitemap.xml', site).href}\n`, {
    headers: { 'content-type': 'text/plain; charset=utf-8' },
  });
```

`web/src/pages/sitemap.xml.ts`:

```ts
import type { APIRoute } from 'astro';
import { LANGS } from '../i18n/format';

export const GET: APIRoute = ({ site }) => {
  const urls = LANGS.map((l) => `  <url><loc>${new URL(`${l}/`, site).href}</loc></url>`).join('\n');
  return new Response(
    `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}\n</urlset>\n`,
    { headers: { 'content-type': 'application/xml; charset=utf-8' } },
  );
};
```

- [ ] **Step 6: Compression script** (`web/scripts/compress.mjs`)

```js
// Cloud Run sends files as they are: write .br and .gz next to each compressible file in dist/,
// which the server's ServeDir picks by Accept-Encoding.
import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { extname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { brotliCompressSync, constants, gzipSync } from 'node:zlib';

export const COMPRESSIBLE = new Set(['.html', '.css', '.js', '.mjs', '.json', '.svg', '.xml', '.txt', '.wasm', '.ttf']);
export const MIN_BYTES = 1024;

export function compressDir(dir) {
  let count = 0;
  for (const entry of readdirSync(dir, { withFileTypes: true, recursive: true })) {
    if (!entry.isFile() || !COMPRESSIBLE.has(extname(entry.name))) continue;
    const path = join(entry.parentPath, entry.name);
    const data = readFileSync(path);
    if (data.length < MIN_BYTES) continue;
    writeFileSync(`${path}.br`, brotliCompressSync(data, {
      params: { [constants.BROTLI_PARAM_QUALITY]: 11, [constants.BROTLI_PARAM_SIZE_HINT]: data.length },
    }));
    writeFileSync(`${path}.gz`, gzipSync(data, { level: 9 }));
    count += 1;
  }
  return count;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const dist = fileURLToPath(new URL('../dist/', import.meta.url));
  console.log(`compressed ${compressDir(dist)} files`);
}
```

- [ ] **Step 7: Build and run every check**

Run: `cd web && npm run check && npm test && npm run build && npm run test:dist && cd ../design && npm test`
Expected: `astro check` 0 errors; unit tests PASS; build writes `dist/`; dist tests PASS; design guard PASS.

Then look at it: `npx astro preview` and open `http://localhost:4321/en/` — type 7.4 km / 22 min, add cars, check the sheet, result, language menu and dark mode (system setting). The page shows "built-in prices" (no API under preview). Check the browser console for CSP violations — there must be none.

- [ ] **Step 8: Commit**

```bash
git add web/astro.config.mjs web/src/styles/site.css web/src/layouts web/src/pages web/scripts/compress.mjs web/vitest.dist.config.ts web/test-dist
git commit -m "feat(web): language pages, SEO, CSP, 404, robots, sitemap, precompression

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Server — serve the site next to the API

**Files:**
- Modify: `Cargo.toml` (`tower-http` features `fs`, `set-status`)
- Create: `crates/server/src/site.rs`
- Modify: `crates/server/src/lib.rs` (`pub mod site;`)
- Modify: `crates/server/src/app.rs` (`router` signature)
- Modify: `crates/server/src/config.rs` (`ServeConfig.web_dir`, tests)
- Modify: `crates/server/src/commands.rs` (`serve`)
- Modify: `crates/server/tests/common/mod.rs` (`Options.site`, `fake_site()`)
- Modify: `.env.example` (`WEB_DIR`)
- Test: `crates/server/tests/site.rs`

**Interfaces:**
- Consumes: Task 7's `dist/` layout (`{lang}/index.html`, `404.html`, `_astro/…`, precompressed siblings).
- Produces:
  - `whichride_server::site::{LANGUAGES, best_language(header: Option<&str>) -> &'static str, check(dir: &Path) -> Result<(), String>, router(dir: &Path) -> axum::Router}`
  - `whichride_server::app::router(state: AppState, site: Option<Router>) -> Router` (was `router(state)`)
  - `ServeConfig.web_dir: Option<PathBuf>` from `WEB_DIR`.

- [ ] **Step 1: Dependencies**

Workspace `Cargo.toml`:

```toml
tower-http = { version = "0.7.1", features = ["catch-panic", "fs", "limit", "request-id", "set-status", "trace"] }
```

(If tower-http 0.7 names a feature differently, `cargo check` says so; use its name.)

- [ ] **Step 2: Write the failing tests**

In `crates/server/tests/common/mod.rs`: add `use std::path::PathBuf;`, a field `pub site: Option<PathBuf>` on `Options` (default `None` in its `Default` impl), build the router with `router(state, options.site.as_deref().map(whichride_server::site::router))`, and add:

```rust
/// The smallest built site: two language pages, the 404 page and a hashed asset with its gzip.
pub fn fake_site() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("whichride-site-{}", Uuid::new_v4()));
    for (path, body) in [
        ("en/index.html", "<h1>en</h1>"),
        ("it/index.html", "<h1>it</h1>"),
        ("404.html", "<h1>missing</h1>"),
        ("_astro/app.1a2b3c.js", "console.log(1)"),
        ("_astro/app.1a2b3c.js.gz", "gzipped"),
    ] {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, body).unwrap();
    }
    dir
}
```

`crates/server/tests/site.rs`:

```rust
mod common;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use common::{Options, TestApp, fake_site};
use tower::ServiceExt;
use whichride_server::site;

async fn get(router: &Router, path: &str, headers: &[(&str, &str)]) -> (StatusCode, HeaderMap, String) {
    let mut request = Request::get(path);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let response = router.clone().oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 1 << 20).await.unwrap();
    (status, headers, String::from_utf8_lossy(&body).into_owned())
}

#[tokio::test]
async fn the_root_redirects_to_the_visitors_language() {
    let router = site::router(&fake_site());
    for (accept, expected) in [
        (Some("it-IT,it;q=0.9,en;q=0.8"), "/it/"),
        (Some("fr-FR, de;q=0.5, tr;q=0.9"), "/tr/"),
        (Some("fr"), "/en/"),
        (None, "/en/"),
    ] {
        let headers: Vec<(&str, &str)> = accept.map(|a| ("accept-language", a)).into_iter().collect();
        let (status, headers, _) = get(&router, "/", &headers).await;
        assert_eq!(status, StatusCode::FOUND, "{accept:?}");
        assert_eq!(headers[header::LOCATION], expected, "{accept:?}");
        assert_eq!(headers[header::VARY], "Accept-Language");
    }
}

#[tokio::test]
async fn a_language_page_is_served_revalidated_and_hardened() {
    let (status, headers, body) = get(&site::router(&fake_site()), "/it/", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "<h1>it</h1>");
    assert!(headers[header::CONTENT_TYPE].to_str().unwrap().starts_with("text/html"));
    assert_eq!(headers[header::CACHE_CONTROL], "no-cache");
    assert_eq!(headers[header::X_CONTENT_TYPE_OPTIONS], "nosniff");
    assert_eq!(headers[header::REFERRER_POLICY], "no-referrer");
    assert_eq!(headers[header::X_FRAME_OPTIONS], "DENY");
}

#[tokio::test]
async fn a_language_without_its_slash_is_redirected() {
    let (status, headers, _) = get(&site::router(&fake_site()), "/en", &[]).await;
    assert!(status.is_redirection(), "{status}");
    assert_eq!(headers[header::LOCATION], "/en/");
}

#[tokio::test]
async fn hashed_assets_are_immutable_and_precompressed() {
    let (status, headers, body) =
        get(&site::router(&fake_site()), "/_astro/app.1a2b3c.js", &[("accept-encoding", "gzip")]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CONTENT_ENCODING], "gzip");
    assert_eq!(body, "gzipped");
    assert_eq!(headers[header::CACHE_CONTROL], "public, max-age=31536000, immutable");
}

#[tokio::test]
async fn unknown_paths_get_the_404_page_never_cached_as_immutable() {
    let router = site::router(&fake_site());
    for path in ["/nope/", "/_astro/gone.js"] {
        let (status, headers, body) = get(&router, path, &[]).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert_eq!(body, "<h1>missing</h1>", "{path}");
        assert_eq!(headers[header::CACHE_CONTROL], "no-cache", "{path}");
    }
}

#[tokio::test]
async fn unknown_api_paths_keep_the_json_404() {
    let (status, headers, body) = get(&site::router(&fake_site()), "/v1/nope", &[]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(headers[header::CONTENT_TYPE].to_str().unwrap().starts_with("application/json"));
    assert_eq!(body, r#"{"error":"not_found"}"#);
}

#[test]
fn a_directory_without_a_built_site_is_refused() {
    let empty = std::env::temp_dir().join(format!("whichride-empty-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&empty).unwrap();
    assert!(site::check(&empty).unwrap_err().contains("en/index.html"));
    assert!(site::check(&fake_site()).is_ok());
}

#[tokio::test]
async fn the_api_and_the_site_share_one_router() {
    let Some(app) = TestApp::start_with(Options {
        site: Some(fake_site()),
        ..Options::default()
    })
    .await
    else {
        return;
    };
    let (status, body) = app.get("/v1/catalogue").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, _, page) = get(&app.router, "/en/", &[]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(page, "<h1>en</h1>");
    let (status, _, error) = get(&app.router, "/v1/nope", &[]).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(error, r#"{"error":"not_found"}"#);
}
```

(Adjust the `ApiError` JSON literal to the actual body if `error.rs` serialises differently — check the existing `a_panic_is_a_json_internal_error` test for the shape.)

Also in `crates/server/src/site.rs`'s own test module (written with the implementation, Step 4):

```rust
#[cfg(test)]
mod tests {
    use super::best_language;

    #[test]
    fn the_best_supported_language_wins() {
        assert_eq!(best_language(Some("de-DE,de;q=0.9")), "de");
        assert_eq!(best_language(Some("en;q=0.2, it;q=0.8")), "it");
        assert_eq!(best_language(Some("TR")), "tr");
        assert_eq!(best_language(Some("it;q=0, de;q=0.1")), "de", "q=0 means not acceptable");
        assert_eq!(best_language(Some("de;q=0.5, it;q=0.5")), "de", "ties go to the earlier entry");
        assert_eq!(best_language(Some("*")), "en");
        assert_eq!(best_language(Some("it;q=abc")), "en", "a broken q is ignored");
        assert_eq!(best_language(Some("")), "en");
        assert_eq!(best_language(None), "en");
    }
}
```

And in `config.rs` tests:

```rust
    #[test]
    fn web_dir_is_optional() {
        assert_eq!(ServeConfig::from_lookup(lookup(MINIMAL)).unwrap().web_dir, None);
        let mut pairs = MINIMAL.to_vec();
        pairs.push(("WEB_DIR", " /srv/web "));
        let config = ServeConfig::from_lookup(lookup(&pairs)).unwrap();
        assert_eq!(config.web_dir, Some(std::path::PathBuf::from("/srv/web")));
    }
```

- [ ] **Step 3: Run to see them fail**

Run: `cargo test -p whichride-server --test site && cargo test -p whichride-server config`
Expected: compile errors (`site` module, `web_dir`, `Options.site` missing).

- [ ] **Step 4: Implement `crates/server/src/site.rs`**

```rust
//! The website (milestone 7): the static files Astro built, served after the API routes.
//! Only the route template ("unmatched") is logged, as for the API.

use std::path::Path;

use axum::Router;
use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_status::SetStatus;

use crate::error::ApiError;

/// The site's languages; the first is the default.
pub const LANGUAGES: [&str; 4] = ["en", "it", "de", "tr"];

/// Files a built site must have; `serve` refuses a `WEB_DIR` without them.
const REQUIRED: [&str; 2] = ["en/index.html", "404.html"];

pub fn check(dir: &Path) -> Result<(), String> {
    for file in REQUIRED {
        if !dir.join(file).is_file() {
            return Err(format!("{} has no {file}: build the site (web/: npm run build)", dir.display()));
        }
    }
    Ok(())
}

/// `/` → the visitor's language, unknown `/v1/…` → the API's JSON 404, everything else from `dir`.
pub fn router(dir: &Path) -> Router {
    let not_found = SetStatus::new(ServeFile::new(dir.join("404.html")), StatusCode::NOT_FOUND);
    let files = ServeDir::new(dir)
        .precompressed_br()
        .precompressed_gzip()
        .not_found_service(not_found);
    Router::new()
        .route("/", get(to_language))
        .route("/v1/{*rest}", any(|| async { ApiError::NotFound }))
        .fallback_service(files)
        .layer(middleware::from_fn(site_headers))
}

async fn to_language(headers: HeaderMap) -> Response {
    let accept = headers.get(header::ACCEPT_LANGUAGE).and_then(|v| v.to_str().ok());
    (
        StatusCode::FOUND,
        [
            (header::LOCATION, format!("/{}/", best_language(accept))),
            (header::VARY, "Accept-Language".to_owned()),
        ],
    )
        .into_response()
}

/// The best of `LANGUAGES` for an `Accept-Language` header: highest q wins, ties go to the earlier
/// entry, q=0 and broken entries are skipped; `en` when nothing matches.
pub fn best_language(header: Option<&str>) -> &'static str {
    let mut best: Option<(&'static str, u16)> = None;
    for item in header.unwrap_or_default().split(',') {
        let mut parts = item.split(';');
        let primary = parts.next().unwrap_or_default().trim().split('-').next().unwrap_or_default().to_ascii_lowercase();
        let quality = match parts.find_map(|p| p.trim().strip_prefix("q=")) {
            None => Some(1000),
            Some(q) => parse_quality(q),
        };
        let (Some(quality), Some(language)) = (quality, LANGUAGES.iter().find(|l| **l == primary)) else {
            continue;
        };
        if quality > 0 && best.is_none_or(|(_, b)| quality > b) {
            best = Some((language, quality));
        }
    }
    best.map_or(LANGUAGES[0], |(language, _)| language)
}

/// "0.8" → 800; outside 0..=1 or not a number → None.
fn parse_quality(text: &str) -> Option<u16> {
    let value: f32 = text.trim().parse().ok()?;
    (0.0..=1.0).contains(&value).then(|| (value * 1000.0).round() as u16)
}

/// Hashed assets are cached for a year; everything else revalidates. Hardening headers for all.
async fn site_headers(request: Request, next: Next) -> Response {
    let hashed = request.uri().path().starts_with("/_astro/");
    let mut response = next.run(request).await;
    let immutable = hashed && response.status().is_success();
    let headers = response.headers_mut();
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(if immutable { "public, max-age=31536000, immutable" } else { "no-cache" }),
    );
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    response
}
```

(If clippy flags the `as u16` cast, keep it with `#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]` on `parse_quality` and a comment: the value is checked to be in 0..=1000.)

`crates/server/src/lib.rs`: add `pub mod site;` with the other modules.

- [ ] **Step 5: Router, config, serve**

`crates/server/src/app.rs`:

```rust
/// The API, and the website behind it when `site` is given (`site::router`).
pub fn router(state: AppState, site: Option<Router>) -> Router {
    let api = Router::new()
        .route("/health", get(|| async { "ok" }))
        // … every existing .route(…) unchanged …
        .route("/v1/expand", post(crate::expand::post_expand))
        .method_not_allowed_fallback(|| async { ApiError::MethodNotAllowed });
    let api = match site {
        Some(site) => api.fallback_service(site),
        None => api.fallback(|| async { ApiError::NotFound }),
    };
    api.with_state(state)
        .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
        // … the existing layers unchanged …
}
```

`crates/server/src/config.rs`: add `pub web_dir: Option<PathBuf>` to `ServeConfig` (and to its `Debug`: `.field("web_dir", &self.web_dir)`), `use std::path::PathBuf;`, and in `from_lookup`:

```rust
        let web_dir = get("WEB_DIR").map(|dir| PathBuf::from(dir.trim()));
```

…included in the returned struct.

`crates/server/src/commands.rs` `serve()`, before binding the listener:

```rust
    let site = match &config.web_dir {
        Some(dir) => {
            crate::site::check(dir).map_err(anyhow::Error::msg)?;
            info!(dir = %dir.display(), "serving the website");
            Some(crate::site::router(dir))
        }
        None => None,
    };
```

and `axum::serve(listener, router(state, site))`.

`.env.example`, after `ANDROID_PACKAGE=`:

```
# The built website (web/dist after npm run build); unset → API only
# WEB_DIR=../web/dist
```

- [ ] **Step 6: Run the tests**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test -p whichride-server && cargo clippy --workspace --all-targets --locked && cargo fmt --all --check`
Expected: all PASS (start the smoke-pg container first if it is stopped: `docker start smoke-pg`). If tower-http's `ServeDir` answers `/en` with a different redirect status than expected, the test only requires a redirection to `/en/`.

- [ ] **Step 7: Try it locally**

```bash
cd web && npm run build && cd ..
WEB_DIR=web/dist PLAY_INTEGRITY=off ORS_API_KEY=x DATABASE_URL=postgres://postgres@localhost:55432/postgres cargo run -p whichride-server -- serve
```

(Requires a migrated database there; if not, `cargo run -p whichride-server -- migrate` first with the same `DATABASE_URL`.) Open `http://127.0.0.1:8080/` → redirected to your language; the page loads prices from `/v1/catalogue` (no "built-in prices" line). Stop the server.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock crates/server .env.example
git commit -m "feat(server): serve the website next to the API (WEB_DIR)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Docker image, CI, docs

**Files:**
- Modify: `Dockerfile`, `.dockerignore`
- Modify: `web/test/versions.test.ts` (Dockerfile pin)
- Modify: `.github/workflows/ci.yml` (new `web` job, docker job check)
- Modify: `.github/dependabot.yml` (if present: npm `/web`)
- Modify: `docs/DECISIONS.md` (decisions 84–91)
- Modify: `docs/plans/ROADMAP.md` (row 7)
- Modify: `README.md` (website section)
- Create: `web/README.md`

**Interfaces:**
- Consumes: everything above. The image serves the site at `/` with `WEB_DIR=/srv/web`; the deploy workflow needs no change (it builds the same Dockerfile).

- [ ] **Step 1: Dockerfile**

```dockerfile
# syntax=docker/dockerfile:1
# A static musl binary and the website on a distroless base: no shell, no package manager.

FROM rust:1-alpine AS build
RUN apk add --no-cache gcc musl-dev
# Embeds the dependency list in the binary, so Trivy can scan the Rust crates inside it.
RUN cargo install cargo-auditable --version 0.7.7 --locked
# Must equal wasm-bindgen in Cargo.lock (web/test/versions.test.ts checks this line).
RUN cargo install wasm-bindgen-cli --version 0.2.129 --locked
WORKDIR /src
COPY . .
# rust-toolchain.toml adds the wasm32-unknown-unknown target.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo auditable build --release --locked --bin whichride-server \
    && cp target/release/whichride-server /whichride-server \
    && cargo build --profile web --locked -p whichride-wasm --target wasm32-unknown-unknown \
    && wasm-bindgen --target web --out-dir /wasm target/wasm32-unknown-unknown/web/whichride_wasm.wasm

FROM node:24-alpine AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
# The site reads the app's wording and the seed prices at build time.
COPY android/app/src/main/res/ /src/android/app/src/main/res/
COPY crates/server/catalogue.json src/generated/catalogue.json
COPY --from=build /wasm src/generated/wasm
RUN npm run build

FROM gcr.io/distroless/static-debian13:nonroot
COPY --from=build /whichride-server /usr/local/bin/whichride-server
COPY --from=web /src/web/dist /srv/web
ENV BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info \
    WEB_DIR=/srv/web
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/whichride-server"]
CMD ["serve"]
```

If `cargo install wasm-bindgen-cli` fails on musl for a missing system library, add it to the `apk add` line (e.g. `openssl-dev pkgconfig`) and note it in the commit.

`.dockerignore` — append:

```
**/node_modules
web/dist
web/.astro
web/src/generated
android/**/build
android/.gradle
```

- [ ] **Step 2: Build the image and check the site is in it**

```bash
docker build -t whichride:local .
id=$(docker create whichride:local) && docker cp "$id:/srv/web/en/index.html" - | tar -xO | grep -c '<html lang="en"' && docker rm "$id"
cd web && npx vitest run test/versions.test.ts
```

Expected: `1`.

Add to `web/test/versions.test.ts`:

```ts
test('the Dockerfile installs the wasm-bindgen CLI that matches Cargo.lock', () => {
  const version = lockedVersion(root('Cargo.lock'), 'wasm-bindgen');
  expect(root('Dockerfile')).toContain(`cargo install wasm-bindgen-cli --version ${version} --locked`);
});
```

Run: `cd web && npx vitest run test/versions.test.ts` — PASS; changing the version in the Dockerfile makes it FAIL.

- [ ] **Step 3: CI**

In `.github/workflows/ci.yml`, docker job, after `docker run --rm whichride-server:ci --help`:

```yaml
      - name: the image carries the website
        run: |
          id=$(docker create whichride-server:ci)
          docker cp "$id:/srv/web/en/index.html" - | tar -xO | grep -q '<html lang="en"'
          docker rm "$id"
```

New job (mirror the existing jobs' action pins and `persist-credentials: false`):

```yaml
  web:
    name: website (WASM core, Astro, Vitest)
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: web
    steps:
      - uses: actions/checkout@v7
        with:
          persist-credentials: false
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - name: wasm-bindgen CLI at the Cargo.lock version
        working-directory: .
        run: |
          version=$(awk '/^name = "wasm-bindgen"$/ { getline; gsub(/version = |"/, ""); print }' Cargo.lock)
          cargo install wasm-bindgen-cli --version "$version" --locked
      - uses: actions/setup-node@v7
        with:
          node-version-file: web/.nvmrc
          cache: npm
          cache-dependency-path: web/package-lock.json
      - run: npm ci
      - run: npm run generate
      - run: npm run check
      - run: npm test
      - run: npm run build
      - run: npm run test:dist
```

If `.github/dependabot.yml` exists, add an `npm` entry for `/web` with the same schedule and grouping as the `/design` entry.

- [ ] **Step 4: Decisions** — append to the table in `docs/DECISIONS.md` (same columns as rows 78–83):

| # | Date | Area | Decision | Alternatives | Why |
|---|---|---|---|---|---|
| 84 | 2026-10-21 | Web | The **website is served by the Cloud Run server** (`WEB_DIR`, `ServeDir` after the API routes) at its `run.app` URL; no domain for now (`WHICHRIDE_SITE_URL` changes it). | Firebase Hosting / Cloudflare Pages; a custom domain | One deploy, one origin (no CORS), nothing new to run or pay |
| 85 | 2026-10-21 | Web | **Web input is km and minutes only**: no routing, address search or location from the web, so **no Turnstile** and no new endpoint (amends spec §8.1). | Routing behind Turnstile and web quotas | Owner: "nothing else. it must be simple"; nothing to abuse |
| 86 | 2026-10-21 | Web | **Layout C**: the page starts with no car; cars are added through a sheet whose ticks are a draft (button applies, ✕/Esc drops); trip and cars are kept in `localStorage` for 15 minutes. | Chips above the result (A); one list with ticks (B) | Same model and wording as the app's result screen (mockups 2026-10-21) |
| 87 | 2026-10-21 | Web | **Astro static pages with one Svelte 5 island**. | Plain TypeScript DOM code; a SPA framework | Indexable HTML without a JS framework on every page; no hand-written DOM updates |
| 88 | 2026-10-21 | Web | **`crates/wasm`** wraps the core with wasm-bindgen (logic tested natively); the output is a build artefact; the wasm-bindgen CLI version is the one in `Cargo.lock`, checked by a test against the Dockerfile. | Committing the WASM; wasm-pack | Same pattern as `crates/ffi` (decision 58); no drift between glue and crate |
| 89 | 2026-10-21 | Web | **Shared wording is read from the app's `strings.xml`** (allowlist, Android escapes undone, `%1$s` → `{1}`, plurals by `Intl.PluralRules`, lists through `list_or`); web-only wording in `web/src/i18n/*.json`. | A neutral generated source for both | Native files stay canonical for translation tools; one place per sentence |
| 90 | 2026-10-21 | Web | **No per-provider SEO pages** (amends spec §10.4); four language pages with hreflang, canonical and a sitemap. | A page per provider | Owner: one simple page |
| 91 | 2026-10-21 | Web | Site delivery: **`.br`/`.gz` written at build** and served by `ServeDir`; `/_astro/` immutable for a year, the rest `no-cache`; **CSP** from Astro's `security.csp` meta (`'wasm-unsafe-eval'`, no `'unsafe-inline'`); `nosniff`, `no-referrer`, `X-Frame-Options: DENY`; no media queries (`size.content-max`). | On-the-fly compression; no CSP | Cloud Run doesn't compress; the WASM is the largest file; tokens can't be used in media queries |

- [ ] **Step 5: Roadmap and READMEs**

`docs/plans/ROADMAP.md` row 7:

```
| 7 | **Website** | Astro page per language with one Svelte island; core via WebAssembly (`crates/wasm`); km + minutes and a car picker only (no Turnstile, no SEO provider pages: decisions 85, 90); served by the server | §8.1, §10.4, website spec | `07-website.md` |
```

`web/README.md`:

```markdown
# whichride website

Astro pages (`src/pages`) with one Svelte island (`src/components/Comparison.svelte`) over the
Rust core in WebAssembly (`crates/wasm`). Shared wording comes from the app's `strings.xml`
(`src/i18n/android.ts`); web-only wording from `src/i18n/*.json`. Colours and sizes only from
`src/styles/tokens.css` (generated by `design/`).

    cargo install wasm-bindgen-cli --version <wasm-bindgen in Cargo.lock> --locked
    npm ci
    npm run generate   # WASM + seed catalogue into src/generated (git-ignored)
    npm run dev        # http://localhost:4321/en/ — built-in prices (no API in dev)
    npm test           # unit + island tests
    npm run build      # dist/, precompressed
    npm run test:dist  # checks dist/

The server serves `dist/` when `WEB_DIR` points to it.
```

Root `README.md`: one line under the repository layout listing `web/` ("the website, served by the server") and `crates/wasm` ("the core for the browser"), matching the existing style.

- [ ] **Step 6: Whole-branch verification**

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets --locked
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace --locked
cd design && npm test && npm run contrast && npm run build && git status --porcelain -- ../android ../web/src/styles ../web/public && cd ..
cd web && npm run generate && npm run check && npm test && npm run build && npm run test:dist && cd ..
docker build -t whichride:local .
```

Expected: everything green; the design `git status` prints nothing (generated files committed).

- [ ] **Step 7: Commit and push**

```bash
git add Dockerfile .dockerignore .github docs README.md web/README.md web/test/versions.test.ts
git commit -m "build: website in the server image; CI web job; decisions 84-91

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git push origin feat/m7-website   # origin = git@github-locadani:locadani/whichride.git
```
 The owner opens the PR and squash-merges; after deploy, `https://whichride-1034831276867.europe-west3.run.app/` redirects to a language page.
