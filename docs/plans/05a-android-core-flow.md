# Milestone 5a: Android app v1, core flow — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** An installable Android app that takes a destination (Google Maps share, `geo:` link,
pasted link, or typed address with suggestions), finds the user's location, gets km and minutes
from our server (or from the user when the route service is busy), prices every offer with the
Rust core, lets the user add the cars near them (§4.1), and opens the chosen provider's app or
Google Maps.

**Architecture:** A new crate `crates/ffi` wraps `carsharing-core` for UniFFI; the core itself
gains the §4.1 option ranking and stays free of UniFFI. Gradle builds the Rust library for three
Android ABIs with cargo-ndk, generates the Kotlin bindings from it, and builds a host copy so JVM
unit tests run the real Rust. The Gradle project gets `:core` (the bindings) and `:app` (the
screens) next to `:designsystem`. The app is one activity, one ViewModel holding its own back
stack, small interfaces at every Android or network edge (`Routes`, `LocationSource`,
`Launcher`, `IntegrityTokens`, `TokenStore`) and fakes or MockWebServer in tests.

**Tech Stack:** Rust (workspace toolchain), UniFFI 0.32.2, cargo-ndk 4.1.2, NDK 30.0.16248370;
Gradle 9.8.1, AGP 9.4.1 (built-in Kotlin), Kotlin 2.4.21 (+ serialization plugin), Compose BOM
2026.09.00 (Material 3, material-icons-core 1.7.8, ui-test-junit4), activity-compose 1.13.0,
lifecycle 2.11.0, core-ktx 1.19.1, OkHttp 5.5.0 (+ okhttp-coroutines, mockwebserver3),
kotlinx-serialization-json 1.12.0, kotlinx-coroutines 1.11.0 (+ play-services, test), Play
Integrity 1.6.0, JNA 5.19.1, JUnit 4.13.2, Robolectric 4.17, androidx.test core 1.7.0 /
ext-junit 1.3.0; compileSdk 37, targetSdk 36, minSdk 26.

**Spec:** `docs/specs/2026-10-19-carsharing-compare-design.md` §4 (core flow), §4.1 (cars near
you), §7.5 (freshness), §8 (routing, manual fallback), §9 (privacy), §10.3 (Android), §11
(touch targets). Decisions `docs/DECISIONS.md` 1–57; this plan adds 58–64. **Design:** canvas
https://claude.ai/artifact/P2A5aUCR45ZBKhnRRqCW2b, artboards Home (where to, shared from Maps,
route service busy) and Main (result, step 1 provider, step 2 cars). PriceCheck and Discounts are
5b.

## Scope

**5a (this plan):** UniFFI bindings built from Gradle; `:app` next to `:designsystem`; share
sheet and `geo:` entry; paste a link or type an address (autocomplete through our server);
location; result screen; adding the cars near you (§4.1, decision 50); **Open <provider>**
(SIXT opens BVG Jelbi, decision 48) and **Open in Google Maps**; the manual km/min fallback.
Install registration (Play Integrity) is here because routing needs a registered install
(decision 33, ruling R9).

**5b (later):** personal discounts and passes, price reports and confirmations with Play
Integrity, the per-user choice of which app opens a provider, EN/IT/DE/TR, accessibility pass
(TalkBack walk-through, font scale 200 %).

## Rulings ledger

Calls made without the owner, each defensible on its merits. DECISIONS.md rows are marked.

| # | Ruling | Why |
|---|---|---|
| R1 | Branch `feat/m5a-android-core-flow` from `origin/main` `de0f359` (milestone 4 merged). `fix/pin-gcp-auth-action` is **not** in main yet; untouched here. | Owner merged M4; the pin PR is independent. |
| R2 | Bindings live in a new crate **`crates/ffi`** (`whichride-ffi`); the bindgen CLI in **`crates/uniffi-bindgen`**. `carsharing-core` gets no UniFFI dependency. **Decision 58.** | The server and the future WASM build keep a pure core; the shipped `.so` doesn't link clap (the CLI feature). |
| R3 | UniFFI **0.32.2**, the spike's version; `MPL-2.0` joins `deny.toml`'s allow-list. **Decision 64.** | Proven on the owner's phone; MPL is file-level copyleft and we don't modify UniFFI. |
| R4 | Gradle builds Rust with **own tasks in `android/buildSrc`** calling cargo-ndk and library-mode bindgen, not Gobley. Outputs stay in `build/` and are added through AGP's variant API; nothing generated is committed. **Decision 58.** | The spike proved these tools with AGP 9.4; Gobley's support for AGP 9's built-in Kotlin is unproven. Rust is needed to build the `.so` anyway, so committing bindings saves nothing (unlike decision 55). |
| R5 | Bindings are generated from the **arm64 Android `.so`** (exported `UNIFFI_META_*` symbols survive `strip`); fallback: the host build. | Same command on macOS and Linux CI. |
| R6 | Cargo profile **`android`** (inherits release; `opt-level = "z"`, `lto = true`, `codegen-units = 1`); panics stay `unwind` (UniFFI turns them into Kotlin exceptions). cargo-ndk `-P 26` (= minSdk). | Spike: ~380 KB per ABI with these settings; `abort` would kill the app on a core bug. |
| R7 | JVM unit tests load a **host debug build** of `whichride-ffi` through JNA (`jna.library.path`), so Kotlin tests exercise the real Rust. | One implementation of prices everywhere is the point of decision 16; tests must not fake it. |
| R8 | The core gains `options` (rank added cars, "any <provider> car" = its cheapest, cheaper offers not added) and `link::shared_label` (place name in a share). | Spec §4.1 requires ranking to be a core function; the label is link-text parsing, which lives in the core (decision 31) and the website will need it. |
| R9 | **Registration with Play Integrity is in 5a**; reports, votes and confirmations stay 5b. Server URL and Cloud project number are Gradle properties `whichride.serverUrl` and `whichride.cloudProjectNumber` (unset → no server: manual km/min only). **Decision 59.** | Route, geocode, autocomplete and expand need a registered install (decision 33). The server isn't deployed yet, so the app must work without it. |
| R10 | The bearer token is encrypted with an **AndroidKeyStore AES-256-GCM key** and stored in app-private preferences; backup is off, so a restored phone re-registers. | Decision 32. |
| R11 | Prices come from the server's catalogue, else the **last one it sent** (cached in app files), else the **seed `crates/server/catalogue.json` copied into the APK at build time**. **Decision 60.** | Comparison is on-device (§8), so it must work offline and before the server exists; copying at build time keeps one source of prices. |
| R12 | Freshness labels (§7.5) are in 5a, from the catalogue's `confirmed_on`; built-in prices say "built-in prices" instead. | They're part of the result screen in §4 step 6 and cost one core call. |
| R13 | **No "Recent" list** and **no automatic clipboard reading** (canvas Home shows both). A link pasted into the destination field is recognised and offered as "Compare this trip". **Decision 61.** | §9 stores no locations; Android 12+ shows a "pasted from clipboard" toast on every silent read. |
| R14 | **Open <provider>** starts an implicit `MAIN`/`LAUNCHER` intent restricted to the catalogue's package; if nothing answers, Play Store (`market://`, then https). No `<queries>`. **Decision 62.** | Package visibility doesn't filter implicit `startActivity`, so a provider added to the catalogue later works without an app update. SIXT's package in the catalogue is already Jelbi (decision 48). |
| R15 | The per-user choice of which app opens a provider moves to **5b**. | It's a setting like discounts; 5a opens the catalogue's package. |
| R16 | **Open in Google Maps** uses Maps URLs `https://www.google.com/maps/dir/?api=1&origin=…&destination=…&travelmode=driving` (Google Maps app first, then any handler); coordinates are written from e7 integers without floats; an unresolved short link opens the link itself. | Official, keyless, cross-app URL format; works even when our server is down. |
| R17 | **Navigation is a back stack owned by the ViewModel** (a list of screens, `BackHandler` pops), no navigation library. Trip, destination and location live in memory only; after process death the app starts at Home. **Decision 63.** | Six screens in a line; a plain list is testable without Android. Not persisting locations follows §9. |
| R18 | Added cars ("options") are stored in **no-backup preferences with their time** and dropped 15 minutes after the last change (`OptionsStore`). | §4.1: 15 minutes on the device; surviving process death matters when the user checks the provider apps. They're offer IDs, not locations. |
| R19 | Location: `LocationManagerCompat.getCurrentLocation` (fused provider on API 31+, else network, else GPS), 15 s timeout, coarse accepted; **asked when the first trip needs a start**, not at launch; kept in memory. | No Play services location library; approximate is enough to route; asking in context. |
| R20 | Start is changeable (§4 step 3): **Change** opens a search with the same autocomplete; **Use my location** resets it. | Spec §4. |
| R21 | Icons from **material-icons-core** (in the BOM); the logo from `:designsystem`'s generated drawable. | Vector XML would need `dp` literals (single-source guard); hand-built `ImageVector`s would need colours. |
| R22 | Cards are `Surface(color = surface)`, not Material `Card()`. | Resolves the M4 deferred minor (`Card()` defaults to surface-highest). |
| R23 | All text in `strings.xml` (English only now); prices with `NumberFormat` currency for the device locale (EUR); km with the locale's decimal separator. | 5b only adds translations. |
| R24 | Manual entry: km `0.1–2000` with dot or comma and up to 3 decimals, minutes `1–1440`, parsed with integer maths. Reachable from the result header (**Edit**) at any time. | §8 "manual entry is always available"; a German keyboard types `7,4`. |
| R25 | Server errors: `503`/`429`/`5xx` → "Route service is busy"; no connection, no server URL or registration impossible → "Can't reach the route service"; `no_route` → "No route by car"; all three go to manual entry. | §8 wording; the user can always finish the comparison. |
| R26 | Autocomplete waits **300 ms** after typing, needs **3 characters**, sends `near` = the start when known, cancels the previous request. | Server limit 600/h (decision 45); fewer wasted calls. |
| R27 | HTTP: OkHttp 5 + kotlinx.serialization; tests use **MockWebServer** as the fake server. | OkHttp is the Android standard; the fake server speaks real HTTP. |
| R28 | UI tests are **Robolectric Compose tests** on the JVM. | CI has no emulator; the behaviours (Open disabled, search box over 6 offers, any-car row) are UI logic worth pinning. |
| R29 | A **Licences** screen shows the two OFL font licences (copied from `design/fonts` at build time) and the app's MIT licence; the full dependency list is milestone 6. | OFL requires the licence to travel with the fonts (M4 note). |
| R30 | Release builds use R8 with keep rules for JNA and the bindings; CI assembles debug and release (unsigned). | R8 breaks JNA without keep rules; catching it in CI. |
| R31 | `launchMode="singleTop"`; a new share while the app is open replaces the trip (added cars are kept). | The user may share a second place from Maps. |
| R32 | Debug builds allow cleartext only to `localhost` and `10.0.2.2` (local server through `adb reverse` or the emulator); release is HTTPS only. | Lets the owner try a local server without weakening release. |
| R33 | DI by hand (`AppContainer`). | Five collaborators; a framework adds build time and nothing testable. |
| R35 | "Car not listed? Add it" (canvas, §5.1) moves to **5b**. | Proposing an offer is a crowd write that needs Play Integrity, like price reports. |
| R36 | A quote's card label is shown as "<label> package" unless it is the catalogue's `"Standard"` card. | The catalogue names pay-as-you-go cards "Standard"; packages are worth naming (spec §5). |
| R34 | The ffi crate doesn't use the workspace `unsafe_code = "forbid"` lint (UniFFI's macros emit `unsafe` and `#[unsafe(no_mangle)]`); its own code has no `unsafe` block. | `forbid` cannot be relaxed per item. |

## Global Constraints

- Work on branch `feat/m5a-android-core-flow` (cut from `main` at `de0f359`; this plan is its
  first commit). Never commit to `main`. Push only to `git@github-locadani:locadani/whichride.git`.
  Never use `gh` for this repo; the owner opens PRs and squash-merges.
- Before every commit `git config user.name` prints `locadani` and `git config user.email` prints
  `61902944+locadani@users.noreply.github.com`. No email address in a tracked file.
- gitleaks pre-commit hook active (`git config core.hooksPath .githooks`). The OpenRouteService key
  is never handled; no secret is needed in this milestone. The server URL and Cloud project number
  are not secrets but are not committed either (Gradle properties, R9).
- Every commit message ends with a blank line and
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Conventional Commits:
  `feat(core): …`, `feat(ffi): …`, `feat(android): …`, `build(android): …`, `ci: …`, `docs: …`.
- **Every graphic value comes from `design/tokens.json`** through `WhichrideTheme` /
  `WhichrideTokens`: no literal colour, `dp`, `sp`, `em`, font weight, `Color.X`,
  `RoundedCornerShape(<number>)` or `alpha = <number>` in hand-written Kotlin or XML under
  `android/*/src/` (tests excepted). `design/test/single-source.test.mjs` must stay green
  (`cd design && npm test`). A new token needs `npm run build` and the regenerated files committed.
- New foreground/background colour pairs go into `design/contrast.json` first.
- Touch targets ≥ 48 dp (`WhichrideTokens.Size.touchTarget`, already the Material minimum via
  `WhichrideTheme`).
- **Privacy (§9):** coordinates only in request bodies; no location, destination or route
  written to disk or logs; no `Log.*` of user data.
- Rust: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked` with
  `RUSTFLAGS=-D warnings`, `cargo test --workspace --locked` stay green; no floats in the core.
- Exact versions in `android/gradle/libs.versions.toml`; no `+` versions.
- JDK 21: `export JAVA_HOME=/opt/homebrew/opt/openjdk@21`. Android SDK at
  `~/Library/Android/sdk` (in `android/local.properties`, git-ignored).
- Disk: never `cargo clean`, `docker rmi`, `rm -rf` of build folders.
- Doc comments say what and why; no tutorial comments.

## Review Focus

1. **A shared short link while the server is unreachable.** The user shares
   "Markthalle Neun https://maps.app.goo.gl/x" with no server: the app must land on manual
   entry titled "Markthalle Neun", and **Open in Google Maps** must open the short link, not
   end on an error (Task 9: `short link without server goes to manual entry with the shared name`).
2. **Location refused or never found.** Permission denied or a 15 s timeout must lead to
   manual entry ("We couldn't find your location"), never an endless spinner (Task 9:
   `no location goes to manual entry`).
3. **A German keyboard.** `7,4` km must be 7 400 m, not 74 km or an error; `7.4.1` is
   refused (Task 5: `kilometresAcceptADecimalComma`, `kilometresRefuseNonsenseAndOutOfRange`).
4. **Coming back after the provider app.** After "Check in Bolt" the process may be killed;
   added cars must come back if under 15 minutes, be gone after, and an added offer that left the
   catalogue must be skipped, not crash (Task 1: `unknown_offers_are_skipped`; Task 5:
   `optionsExpire15MinutesAfterTheLastChange`; Task 9: `cars expire while the app is in the background`).
5. **A second share while the app is open.** `onNewIntent` must replace the trip, keep the added
   cars and cancel the previous route request (Task 9:
   `a new share replaces the trip and keeps the cars`).

---

## File map

```
Cargo.toml                         + crates/ffi, crates/uniffi-bindgen members (glob), uniffi dep, profile.android
deny.toml                          + MPL-2.0
crates/core/src/options.rs         NEW  rank added cars, cheaper offers not added
crates/core/src/link.rs            + shared_label
crates/core/src/lib.rs             + exports
crates/ffi/                        NEW  Cargo.toml, uniffi.toml, src/lib.rs (+ tests)
crates/uniffi-bindgen/             NEW  Cargo.toml, src/main.rs
android/
  settings.gradle.kts              + :core, :app
  build.gradle.kts                 + application, serialization plugins
  gradle/libs.versions.toml        + versions and libraries
  buildSrc/                        NEW  Rust and asset tasks (CargoNdkBuild, CargoHostBuild, UniffiBindgen, CopyIntoDirectory)
  core/                            NEW  Android library: Rust .so + generated bindings; BindingsTest
  app/                             NEW  the app
    build.gradle.kts, proguard-rules.pro
    src/main/AndroidManifest.xml
    src/main/res/values/{strings,themes}.xml, values-night/themes.xml, xml/data_extraction_rules.xml
    src/debug/res/xml/network_security_config.xml, src/debug/AndroidManifest.xml
    src/main/kotlin/io/github/locadani/whichride/
      WhichrideApplication.kt, AppContainer.kt, MainActivity.kt
      entry/SharedText.kt
      format/Units.kt
      launch/MapsUrl.kt, Launcher.kt
      location/LocationSource.kt
      options/Choices.kt, OptionsStore.kt
      prices/PriceBook.kt
      server/ApiResult.kt, ServerApi.kt, Installs.kt, IntegrityTokens.kt, TokenStore.kt, Routes.kt
      compare/Screen.kt, UiState.kt, CompareViewModel.kt
      ui/WhichrideNav.kt, Components.kt, HomeScreen.kt, ResolvingScreen.kt, ManualTripScreen.kt,
         ResultScreen.kt, ProvidersScreen.kt, CarsScreen.kt, StartScreen.kt, LicencesScreen.kt
    src/test/kotlin/io/github/locadani/whichride/…   unit + Robolectric tests
    src/test/resources/robolectric.properties
.github/workflows/ci.yml           android job builds Rust, tests and assembles :app
docs/DECISIONS.md                  + 58–64
docs/plans/ROADMAP.md              row 5 split into 5a / 5b
android/README.md                  NEW  how to build, run, point at a server
```

---
### Task 1: Core — rank the added cars, label of a share

**Files:**
- Create: `crates/core/src/options.rs`
- Modify: `crates/core/src/link.rs` (add `shared_label` + tests), `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `compare::Quote` (cheapest first, as `compare()` returns them), `OfferId`, `ProviderId`.
- Produces:
  - `pub enum Choice { Offer { offer: OfferId }, AnyOf { provider: ProviderId } }` (serde tag `type`, snake_case)
  - `pub struct RankedOption { pub choice: Choice, pub quote: Quote }`
  - `pub struct Options { pub ranked: Vec<RankedOption>, pub cheaper_not_added: Vec<Quote> }`
  - `pub fn options(quotes: &[Quote], choices: &[Choice]) -> Options`
  - `pub fn shared_label(input: &str) -> Option<String>` (in `link`)

- [ ] **Step 1: Write the failing tests** (`crates/core/src/options.rs`, test module)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::money::Cents;

    fn quote(provider: &str, offer: &str, total: i64) -> Quote {
        Quote {
            provider: ProviderId(provider.into()),
            offer: OfferId(offer.into()),
            offer_name: offer.into(),
            card_label: "Standard".into(),
            total: Cents(total),
            overridden: false,
        }
    }

    /// Cheapest first, as `compare()` returns them.
    fn quotes() -> Vec<Quote> {
        vec![
            quote("bolt", "bolt.picanto", 790),
            quote("miles", "miles.s", 849),
            quote("bolt", "bolt.polo", 912),
            quote("miles", "miles.premium", 1180),
            quote("sixt", "sixt.troc", 1300),
        ]
    }

    fn offer(id: &str) -> Choice {
        Choice::Offer { offer: OfferId(id.into()) }
    }

    fn any(id: &str) -> Choice {
        Choice::AnyOf { provider: ProviderId(id.into()) }
    }

    fn ranked(options: &Options) -> Vec<(&str, i64)> {
        options.ranked.iter().map(|o| (o.quote.offer.0.as_str(), o.quote.total.0)).collect()
    }

    #[test]
    fn nothing_added_ranks_nothing_and_reports_nothing() {
        let options = options(&quotes(), &[]);
        assert!(options.ranked.is_empty());
        assert!(options.cheaper_not_added.is_empty());
    }

    #[test]
    fn added_offers_are_ranked_cheapest_first() {
        let options = options(&quotes(), &[offer("sixt.troc"), offer("bolt.polo"), offer("miles.s")]);
        assert_eq!(ranked(&options), [("miles.s", 849), ("bolt.polo", 912), ("sixt.troc", 1300)]);
    }

    #[test]
    fn any_car_is_the_providers_cheapest_offer() {
        let options = options(&quotes(), &[any("miles"), offer("sixt.troc")]);
        assert_eq!(ranked(&options), [("miles.s", 849), ("sixt.troc", 1300)]);
        assert_eq!(options.ranked[0].choice, any("miles"));
    }

    #[test]
    fn any_car_covers_that_providers_specific_offers() {
        let options = options(&quotes(), &[offer("miles.premium"), any("miles")]);
        assert_eq!(ranked(&options), [("miles.s", 849)]);
    }

    #[test]
    fn unknown_offers_are_skipped() {
        let options = options(&quotes(), &[offer("bolt.gone"), any("free2move"), offer("bolt.polo")]);
        assert_eq!(ranked(&options), [("bolt.polo", 912)]);
    }

    #[test]
    fn a_choice_added_twice_appears_once() {
        let options = options(&quotes(), &[offer("bolt.polo"), offer("bolt.polo")]);
        assert_eq!(ranked(&options), [("bolt.polo", 912)]);
    }

    #[test]
    fn cheaper_offers_not_added_are_listed_cheapest_first() {
        let options = options(&quotes(), &[offer("miles.premium")]);
        let cheaper: Vec<_> = options.cheaper_not_added.iter().map(|q| q.offer.0.as_str()).collect();
        assert_eq!(cheaper, ["bolt.picanto", "miles.s", "bolt.polo"]);
    }

    #[test]
    fn an_offer_with_the_same_price_is_not_cheaper() {
        let mut quotes = quotes();
        quotes.insert(2, quote("sixt", "sixt.polo", 849));
        let options = options(&quotes, &[offer("miles.s")]);
        let cheaper: Vec<_> = options.cheaper_not_added.iter().map(|q| q.offer.0.as_str()).collect();
        assert_eq!(cheaper, ["bolt.picanto"]);
    }

    #[test]
    fn nothing_is_cheaper_than_the_cheapest_offer() {
        let options = options(&quotes(), &[any("bolt")]);
        assert!(options.cheaper_not_added.is_empty());
    }

    #[test]
    fn choices_round_trip_through_json() {
        let json = serde_json::to_string(&[offer("bolt.polo"), any("miles")]).unwrap();
        assert_eq!(
            json,
            r#"[{"type":"offer","offer":"bolt.polo"},{"type":"any_of","provider":"miles"}]"#
        );
        let back: Vec<Choice> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, [offer("bolt.polo"), any("miles")]);
    }
}
```

- [ ] **Step 2: Run to verify they fail**

Run: `cargo test -p carsharing-core options` — Expected: compile error (`options` module missing).

- [ ] **Step 3: Implement** (top of `crates/core/src/options.rs`)

```rust
//! The cars near the user that they added as options (spec §4.1): ranked by price, plus the
//! offers they did not add that would be cheaper.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::catalogue::{OfferId, ProviderId};
use crate::compare::Quote;

/// One car the user added: a specific offer, or "any car" of a provider when they don't know
/// the model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Choice {
    Offer { offer: OfferId },
    AnyOf { provider: ProviderId },
}

/// An added car with its price; "any car" is priced as the provider's cheapest offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedOption {
    pub choice: Choice,
    pub quote: Quote,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Cheapest first; empty when nothing was added.
    pub ranked: Vec<RankedOption>,
    /// Offers not added that cost strictly less than the cheapest option, cheapest first.
    pub cheaper_not_added: Vec<Quote>,
}

/// Ranks the user's choices. `quotes` must be in `compare()`'s order (cheapest first, ties by
/// provider and offer), so walking them yields the ranking and each provider's first quote is
/// its cheapest. "Any car" covers every offer of its provider; choices for offers or providers
/// missing from the quotes (removed from the catalogue since) are skipped.
pub fn options(quotes: &[Quote], choices: &[Choice]) -> Options {
    let any_of: HashSet<&ProviderId> = choices
        .iter()
        .filter_map(|choice| match choice {
            Choice::AnyOf { provider } => Some(provider),
            Choice::Offer { .. } => None,
        })
        .collect();
    let offers: HashSet<&OfferId> = choices
        .iter()
        .filter_map(|choice| match choice {
            Choice::Offer { offer } => Some(offer),
            Choice::AnyOf { .. } => None,
        })
        .collect();

    let mut ranked = Vec::new();
    let mut providers_seen = HashSet::new();
    for quote in quotes {
        let choice = if any_of.contains(&quote.provider) {
            if !providers_seen.insert(&quote.provider) {
                continue;
            }
            Choice::AnyOf { provider: quote.provider.clone() }
        } else if offers.contains(&quote.offer) {
            Choice::Offer { offer: quote.offer.clone() }
        } else {
            continue;
        };
        ranked.push(RankedOption { choice, quote: quote.clone() });
    }

    // The cheapest option is the cheapest added offer, so everything strictly cheaper was not added.
    let cheaper_not_added = match ranked.first() {
        Some(best) => quotes
            .iter()
            .take_while(|quote| quote.total < best.quote.total)
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    Options { ranked, cheaper_not_added }
}
```

Add to `lib.rs`: `pub mod options;` and `pub use options::{Choice, Options, RankedOption, options};`
(keep modules alphabetical; `link::shared_label` joins the existing `pub use link::{…}`).

- [ ] **Step 4: Write the failing `shared_label` tests** (in `link.rs`'s test module)

```rust
#[test]
fn shared_label_is_the_first_line_without_a_link() {
    assert_eq!(
        shared_label("Markthalle Neun\nEisenbahnstraße 42, Berlin\n\nhttps://maps.app.goo.gl/x7Qa2Lm").as_deref(),
        Some("Markthalle Neun")
    );
}

#[test]
fn shared_label_skips_blank_lines_and_lines_with_links() {
    assert_eq!(shared_label("https://maps.app.goo.gl/x\n\n  Mauerpark  ").as_deref(), Some("Mauerpark"));
}

#[test]
fn shared_label_is_none_for_a_bare_link() {
    assert_eq!(shared_label("https://maps.app.goo.gl/x7Qa2Lm"), None);
    assert_eq!(shared_label("   "), None);
}

#[test]
fn shared_label_drops_control_and_bidi_characters_and_is_bounded() {
    assert_eq!(shared_label("Mauer\u{202E}park\u{0007}").as_deref(), Some("Mauerpark"));
    let long = "a".repeat(500);
    assert_eq!(shared_label(&long).map(|label| label.chars().count()), Some(MAX_LABEL_CHARS));
}
```

- [ ] **Step 5: Run to verify they fail** — `cargo test -p carsharing-core shared_label` (missing function).

- [ ] **Step 6: Implement** (in `link.rs`, after `parse_destination`)

```rust
/// Longest place name kept from a share.
pub const MAX_LABEL_CHARS: usize = 120;

/// The place name a share carries next to its link: Google Maps shares
/// "Markthalle Neun\n…\nhttps://maps.app.goo.gl/…". The first non-blank line that holds no
/// link, without control or bidirectional-formatting characters (they could reorder what the
/// screen shows), at most [`MAX_LABEL_CHARS`] characters.
pub fn shared_label(input: &str) -> Option<String> {
    if input.len() > MAX_INPUT_BYTES {
        return None;
    }
    let line = input
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && find_links(line).next().is_none())?;
    let label: String = line
        .chars()
        .filter(|c| !c.is_control() && !is_bidi_format(*c))
        .take(MAX_LABEL_CHARS)
        .collect();
    let label = label.trim();
    (!label.is_empty()).then(|| label.to_owned())
}

fn is_bidi_format(c: char) -> bool {
    matches!(c, '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}
```

(If `link.rs` already has an equivalent bidi helper for labels, reuse it instead of adding one.)

- [ ] **Step 7: Run all core tests, fmt, clippy**

Run: `cargo test -p carsharing-core && cargo fmt --all --check && RUSTFLAGS=-D\ warnings cargo clippy -p carsharing-core --all-targets --locked`
Expected: all pass.

- [ ] **Step 8: Commit** — `feat(core): rank the cars the user added and read a share's place name`

---

### Task 2: `whichride-ffi` crate and the bindgen CLI

**Files:**
- Create: `crates/ffi/Cargo.toml`, `crates/ffi/uniffi.toml`, `crates/ffi/src/lib.rs`,
  `crates/uniffi-bindgen/Cargo.toml`, `crates/uniffi-bindgen/src/main.rs`
- Modify: `Cargo.toml` (workspace deps + profile), `deny.toml` (MPL-2.0), `Cargo.lock`

**Interfaces:**
- Consumes: Task 1's `options`, `shared_label`; core `parse_destination`, `compare`,
  `Catalogue`, `Day`, `confirmed_days_ago`.
- Produces (Rust names; Kotlin names in brackets, package `io.github.locadani.whichride.core`):
  - `Coordinate { lat_e7: i32, lon_e7: i32 }` [`Coordinate(latE7, lonE7)`]
  - `Destination::{Coordinates{at, label}, Place{query, near}, ShortLink{url}}` [`Destination.Coordinates` …]
  - `LinkError::{NoLink, NotGoogleMaps, NoDestination, InvalidCoordinates, TooLong}` [`LinkException.NoLink` …]
  - `DataError::Invalid { message }` [`DataException.Invalid`]
  - `fn parse_destination(text: String) -> Result<Destination, LinkError>` [`parseDestination`]
  - `fn shared_label(text: String) -> Option<String>` [`sharedLabel`]
  - `fn destination_from_json(json: String) -> Result<Destination, DataError>` [`destinationFromJson`] — the server's `/v1/expand` body
  - `Trip { metres: u32, seconds: u32 }` [`Trip(metres: UInt, seconds: UInt)`]
  - `ProviderInfo { id, name, android_package, offers: Vec<OfferInfo> }`, `OfferInfo { id, name }`
  - `Quote { provider_id, offer_id, offer_name, card_label, total_cents: i64, overridden: bool, confirmed_days_ago: Option<u32> }`
  - `Choice::{Offer{offer_id}, AnyOf{provider_id}}`, `RankedOption { choice, quote }`, `Options { ranked, cheaper_not_added }`
  - `object Prices` with constructors `from_server_response(json)` / `from_seed(json)`
    [`Prices.fromServerResponse`, `Prices.fromSeed`] and methods `providers()`,
    `quotes(trip, today_unix_seconds: i64)`, `options(trip, choices, today_unix_seconds)`.

- [ ] **Step 1: Workspace changes** — in root `Cargo.toml` add to `[workspace.dependencies]`
  `uniffi = "0.32.2"` and `whichride-ffi = { path = "crates/ffi" }`, and at the end:

```toml
# The Android library (cargo ndk … --profile android): smallest code, still unwinding so a
# core bug becomes a Kotlin exception instead of killing the app.
[profile.android]
inherits = "release"
opt-level = "z"
lto = true
codegen-units = 1
```

`deny.toml` `allow` gains `"MPL-2.0",  # UniFFI (file-level copyleft, used unmodified)`.

- [ ] **Step 2: Crate manifests**

`crates/ffi/Cargo.toml`:

```toml
[package]
name = "whichride-ffi"
version = "0.1.0"
description = "UniFFI bindings of carsharing-core for the Android app"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
repository.workspace = true

[lib]
name = "whichride_ffi"
crate-type = ["cdylib", "lib"]

[dependencies]
carsharing-core.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
uniffi.workspace = true

# Not the workspace lints: UniFFI's macros expand to `unsafe` FFI glue, and the workspace
# forbids `unsafe_code` (which can't be relaxed per item). This crate's own code has none.
[lints.clippy]
all = { level = "warn", priority = -1 }
```

`crates/ffi/uniffi.toml`:

```toml
[bindings.kotlin]
package_name = "io.github.locadani.whichride.core"
cdylib_name = "whichride_ffi"
```

`crates/uniffi-bindgen/Cargo.toml`:

```toml
[package]
name = "uniffi-bindgen"
version = "0.1.0"
description = "UniFFI's binding generator, run by the Android build"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
repository.workspace = true

[dependencies]
uniffi = { workspace = true, features = ["cli"] }

[lints]
workspace = true
```

`crates/uniffi-bindgen/src/main.rs`:

```rust
//! `cargo run -p uniffi-bindgen -- generate --library <lib> --language kotlin …`

fn main() {
    uniffi::uniffi_bindgen_main()
}
```

- [ ] **Step 3: Write the failing tests** (`crates/ffi/src/lib.rs` test module)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const SEED: &str = include_str!("../../server/catalogue.json");
    /// 2026-10-20 12:00 UTC = day 20_746.
    const TODAY: i64 = 1_792_497_600;

    fn server_response() -> String {
        let catalogue: serde_json::Value = serde_json::from_str(SEED).unwrap();
        serde_json::json!({
            "catalogue": catalogue,
            "offers": [
                {"offer": "miles.s", "confirmed_on": 20_744, "locked": false, "pending": null},
                {"offer": "bolt.vw-polo", "confirmed_on": null, "locked": false, "pending": null}
            ],
            "proposals": []
        })
        .to_string()
    }

    const TRIP: Trip = Trip { metres: 7_400, seconds: 22 * 60 };

    #[test]
    fn parses_a_coordinate_link() {
        let destination = parse_destination("https://www.google.com/maps/place/@52.5200,13.4050,15z".into());
        assert!(matches!(destination, Ok(Destination::Coordinates { .. }) | Ok(Destination::Place { .. })));
    }

    #[test]
    fn link_errors_cross_the_boundary() {
        assert_eq!(parse_destination("hello".into()), Err(LinkError::NoLink));
        assert_eq!(parse_destination("https://example.com".into()), Err(LinkError::NotGoogleMaps));
    }

    #[test]
    fn short_links_stay_short_links() {
        assert_eq!(
            parse_destination("Markthalle Neun https://maps.app.goo.gl/x7Qa2Lm".into()),
            Ok(Destination::ShortLink { url: "https://maps.app.goo.gl/x7Qa2Lm".into() })
        );
        assert_eq!(shared_label("Markthalle Neun\nhttps://maps.app.goo.gl/x7Qa2Lm".into()).as_deref(), Some("Markthalle Neun"));
    }

    #[test]
    fn reads_the_servers_expanded_destination() {
        let json = r#"{"type":"coordinates","at":{"lat_e7":525000000,"lon_e7":134000000},"label":"Mauerpark"}"#;
        assert_eq!(
            destination_from_json(json.into()),
            Ok(Destination::Coordinates {
                at: Coordinate { lat_e7: 525_000_000, lon_e7: 134_000_000 },
                label: Some("Mauerpark".into())
            })
        );
        assert!(destination_from_json("{}".into()).is_err());
    }

    #[test]
    fn the_seed_catalogue_loads_with_every_v1_provider() {
        let prices = Prices::from_seed(SEED.into()).unwrap();
        let ids: Vec<_> = prices.providers().into_iter().map(|p| p.id).collect();
        assert_eq!(ids, ["miles", "bolt", "sixt"]);
        assert_eq!(prices.providers()[2].android_package, "com.trafi.whitelabel.bvg");
    }

    #[test]
    fn seed_quotes_have_no_confirmation_date() {
        let prices = Prices::from_seed(SEED.into()).unwrap();
        let quotes = prices.quotes(TRIP, TODAY);
        assert_eq!(quotes.len(), 9);
        assert!(quotes.windows(2).all(|w| w[0].total_cents <= w[1].total_cents));
        assert!(quotes.iter().all(|q| q.confirmed_days_ago.is_none()));
    }

    #[test]
    fn server_quotes_say_how_long_ago_they_were_confirmed() {
        let prices = Prices::from_server_response(server_response()).unwrap();
        let quotes = prices.quotes(TRIP, TODAY);
        let days = |id: &str| quotes.iter().find(|q| q.offer_id == id).unwrap().confirmed_days_ago;
        assert_eq!(days("miles.s"), Some(2));
        assert_eq!(days("bolt.vw-polo"), None);
    }

    #[test]
    fn options_cross_the_boundary() {
        let prices = Prices::from_seed(SEED.into()).unwrap();
        let options = prices.options(
            TRIP,
            vec![Choice::AnyOf { provider_id: "miles".into() }, Choice::Offer { offer_id: "sixt.volkswagen-t-roc".into() }],
            TODAY,
        );
        assert_eq!(options.ranked.len(), 2);
        assert_eq!(options.ranked[0].choice, Choice::AnyOf { provider_id: "miles".into() });
        assert!(options.ranked[0].quote.total_cents <= options.ranked[1].quote.total_cents);
    }

    #[test]
    fn invalid_catalogues_are_errors_not_panics() {
        assert!(Prices::from_seed("{".into()).is_err());
        assert!(Prices::from_seed(r#"{"providers":[{"id":"x","name":"X","android_package":"x","offers":[]}]}"#.into()).is_err());
        assert!(Prices::from_server_response(SEED.into()).is_err());
    }
}
```

(The exact day for `TODAY` must be computed when writing the test: `TODAY / 86_400`; set the
`confirmed_on` in `server_response()` to that day minus 2.)

- [ ] **Step 4: Run to verify they fail** — `cargo test -p whichride-ffi` (types missing).

- [ ] **Step 5: Implement** `crates/ffi/src/lib.rs`

```rust
//! The core's functions for the Android app, through UniFFI. The types are flat (strings,
//! integers) so the generated Kotlin stays simple; every rule lives in `carsharing-core`.

use std::collections::HashMap;
use std::sync::Arc;

use carsharing_core as cc;
use serde::Deserialize;

uniffi::setup_scaffolding!();

/// A point in integer 1e-7 degrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct Coordinate {
    pub lat_e7: i32,
    pub lon_e7: i32,
}

impl From<cc::Coordinate> for Coordinate {
    fn from(c: cc::Coordinate) -> Self {
        Coordinate { lat_e7: c.lat_e7, lon_e7: c.lon_e7 }
    }
}

/// Where the user wants to go, as far as a link tells (see `carsharing_core::Destination`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum Destination {
    Coordinates { at: Coordinate, label: Option<String> },
    Place { query: String, near: Option<Coordinate> },
    ShortLink { url: String },
}

impl From<cc::Destination> for Destination {
    fn from(d: cc::Destination) -> Self {
        match d {
            cc::Destination::Coordinates { at, label } => Destination::Coordinates { at: at.into(), label },
            cc::Destination::Place { query, near } => Destination::Place { query, near: near.map(Into::into) },
            cc::Destination::ShortLink { url } => Destination::ShortLink { url },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum LinkError {
    #[error("the text contains no link")]
    NoLink,
    #[error("the link is not a Google Maps link")]
    NotGoogleMaps,
    #[error("the link has no destination")]
    NoDestination,
    #[error("the link's coordinates are out of range")]
    InvalidCoordinates,
    #[error("the text is too long")]
    TooLong,
}

impl From<cc::LinkError> for LinkError {
    fn from(e: cc::LinkError) -> Self {
        match e {
            cc::LinkError::NoLink => LinkError::NoLink,
            cc::LinkError::NotGoogleMaps => LinkError::NotGoogleMaps,
            cc::LinkError::NoDestination => LinkError::NoDestination,
            cc::LinkError::InvalidCoordinates => LinkError::InvalidCoordinates,
            cc::LinkError::TooLong => LinkError::TooLong,
        }
    }
}

/// Data from the server or the APK that the core refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum DataError {
    #[error("{message}")]
    Invalid { message: String },
}

fn invalid(error: impl std::fmt::Display) -> DataError {
    DataError::Invalid { message: error.to_string() }
}

#[uniffi::export]
pub fn parse_destination(text: String) -> Result<Destination, LinkError> {
    Ok(cc::parse_destination(&text)?.into())
}

#[uniffi::export]
pub fn shared_label(text: String) -> Option<String> {
    cc::link::shared_label(&text)
}

/// The body of the server's `POST /v1/expand`.
#[uniffi::export]
pub fn destination_from_json(json: String) -> Result<Destination, DataError> {
    let destination: cc::Destination = serde_json::from_str(&json).map_err(invalid)?;
    Ok(destination.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct Trip {
    pub metres: u32,
    pub seconds: u32,
}

impl From<Trip> for cc::Trip {
    fn from(t: Trip) -> Self {
        cc::Trip { metres: t.metres, seconds: t.seconds }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OfferInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub android_package: String,
    pub offers: Vec<OfferInfo>,
}

/// One offer's best price for the trip.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Quote {
    pub provider_id: String,
    pub offer_id: String,
    pub offer_name: String,
    pub card_label: String,
    pub total_cents: i64,
    pub overridden: bool,
    /// Whole days since the crowd last confirmed the price; `None` if never (or built-in prices).
    pub confirmed_days_ago: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum Choice {
    Offer { offer_id: String },
    AnyOf { provider_id: String },
}

impl From<&Choice> for cc::Choice {
    fn from(c: &Choice) -> Self {
        match c {
            Choice::Offer { offer_id } => cc::Choice::Offer { offer: cc::OfferId(offer_id.clone()) },
            Choice::AnyOf { provider_id } => cc::Choice::AnyOf { provider: cc::ProviderId(provider_id.clone()) },
        }
    }
}

impl From<cc::Choice> for Choice {
    fn from(c: cc::Choice) -> Self {
        match c {
            cc::Choice::Offer { offer } => Choice::Offer { offer_id: offer.0 },
            cc::Choice::AnyOf { provider } => Choice::AnyOf { provider_id: provider.0 },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RankedOption {
    pub choice: Choice,
    pub quote: Quote,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Options {
    pub ranked: Vec<RankedOption>,
    pub cheaper_not_added: Vec<Quote>,
}

/// The fields of `GET /v1/catalogue` the app uses in 5a (pending reports and proposals are 5b).
#[derive(Deserialize)]
struct ServerCatalogue {
    catalogue: cc::Catalogue,
    offers: Vec<ServerOfferStatus>,
}

#[derive(Deserialize)]
struct ServerOfferStatus {
    offer: cc::OfferId,
    confirmed_on: Option<cc::Day>,
}

/// A validated catalogue and when each price was last confirmed.
#[derive(uniffi::Object)]
pub struct Prices {
    catalogue: cc::Catalogue,
    confirmed_on: HashMap<cc::OfferId, cc::Day>,
}

#[uniffi::export]
impl Prices {
    /// The body of the server's `GET /v1/catalogue`.
    #[uniffi::constructor]
    pub fn from_server_response(json: String) -> Result<Arc<Prices>, DataError> {
        let response: ServerCatalogue = serde_json::from_str(&json).map_err(invalid)?;
        response.catalogue.validate().map_err(invalid)?;
        let confirmed_on = response
            .offers
            .into_iter()
            .filter_map(|status| Some((status.offer, status.confirmed_on?)))
            .collect();
        Ok(Arc::new(Prices { catalogue: response.catalogue, confirmed_on }))
    }

    /// The seed catalogue shipped in the APK (`crates/server/catalogue.json`); nothing is confirmed.
    #[uniffi::constructor]
    pub fn from_seed(json: String) -> Result<Arc<Prices>, DataError> {
        let catalogue = cc::Catalogue::from_json(&json).map_err(invalid)?;
        Ok(Arc::new(Prices { catalogue, confirmed_on: HashMap::new() }))
    }

    pub fn providers(&self) -> Vec<ProviderInfo> {
        self.catalogue
            .providers
            .iter()
            .map(|p| ProviderInfo {
                id: p.id.0.clone(),
                name: p.name.clone(),
                android_package: p.android_package.clone(),
                offers: p.offers.iter().map(|o| OfferInfo { id: o.id.0.clone(), name: o.name.clone() }).collect(),
            })
            .collect()
    }

    /// Every offer's best price, cheapest first. Personal adjustments arrive in milestone 5b.
    pub fn quotes(&self, trip: Trip, today_unix_seconds: i64) -> Vec<Quote> {
        let today = cc::Day::from_unix_seconds(today_unix_seconds);
        self.core_quotes(trip).iter().map(|q| self.quote(q, today)).collect()
    }

    /// The user's added cars ranked, and the cheaper ones they did not add (spec §4.1).
    pub fn options(&self, trip: Trip, choices: Vec<Choice>, today_unix_seconds: i64) -> Options {
        let today = cc::Day::from_unix_seconds(today_unix_seconds);
        let quotes = self.core_quotes(trip);
        let choices: Vec<cc::Choice> = choices.iter().map(Into::into).collect();
        let options = cc::options(&quotes, &choices);
        Options {
            ranked: options
                .ranked
                .into_iter()
                .map(|o| RankedOption { choice: o.choice.into(), quote: self.quote(&o.quote, today) })
                .collect(),
            cheaper_not_added: options.cheaper_not_added.iter().map(|q| self.quote(q, today)).collect(),
        }
    }
}

impl Prices {
    fn core_quotes(&self, trip: Trip) -> Vec<cc::Quote> {
        cc::compare(&self.catalogue, &trip.into(), &cc::Adjustments::default())
    }

    fn quote(&self, q: &cc::Quote, today: cc::Day) -> Quote {
        Quote {
            provider_id: q.provider.0.clone(),
            offer_id: q.offer.0.clone(),
            offer_name: q.offer_name.clone(),
            card_label: q.card_label.clone(),
            total_cents: q.total.0,
            overridden: q.overridden,
            confirmed_days_ago: cc::confirmed_days_ago(today, self.confirmed_on.get(&q.offer).copied()),
        }
    }
}
```

- [ ] **Step 6: Run** `cargo test -p whichride-ffi`, `cargo fmt --all --check`,
  `RUSTFLAGS=-D\ warnings cargo clippy --workspace --all-targets --locked`, `cargo deny check`
  (if installed; else the CI job checks it). Expected: pass. If clippy flags macro output, allow
  that lint at crate level with a comment naming UniFFI.

- [ ] **Step 7: Build the Android library once by hand** (proves R5/R6 before Gradle wraps it):

```sh
export ANDROID_NDK_HOME=~/Library/Android/sdk/ndk/30.0.16248370
cargo ndk -t arm64-v8a -P 26 -o /tmp/whichride-jni build -p whichride-ffi --profile android
cargo run -p uniffi-bindgen -- generate --library /tmp/whichride-jni/arm64-v8a/libwhichride_ffi.so \
  --language kotlin --no-format --out-dir /tmp/whichride-kt
ls -l /tmp/whichride-jni/arm64-v8a/ && find /tmp/whichride-kt -name '*.kt'
```

Expected: one `.so` (~0.5 MB) and `io/github/locadani/whichride/core/whichride_ffi.kt`. (Use the
scratchpad instead of `/tmp` when running as an agent.) If bindgen can't read the stripped `.so`,
switch Task 3's bindgen input to a host build and update R5.

- [ ] **Step 8: Commit** — `feat(ffi): UniFFI bindings of the core for Android`

---

### Task 3: `:core` Android module — Gradle builds the Rust

**Files:**
- Create: `android/buildSrc/build.gradle.kts`, `android/buildSrc/src/main/kotlin/RustTasks.kt`,
  `android/buildSrc/src/main/kotlin/CopyIntoDirectory.kt`, `android/core/build.gradle.kts`,
  `android/core/consumer-rules.pro`,
  `android/core/src/test/kotlin/io/github/locadani/whichride/core/BindingsTest.kt`
- Modify: `android/settings.gradle.kts`, `android/gradle/libs.versions.toml`, `.github/workflows/ci.yml`

**Interfaces:**
- Produces: Gradle project `:core` exposing the generated Kotlin API of Task 2 and JNA (`api`);
  task `:core:cargoHostBuild` and the directory `<repo>/target/debug` for `jna.library.path`
  (used by `:app`'s tests in later tasks); helper `fun Project.useHostRustInTests()` in buildSrc.

- [ ] **Step 1: Version catalog** — add to `android/gradle/libs.versions.toml`:

```toml
[versions]
# … existing …
ndk = "30.0.16248370"
jna = "5.19.1"
activity-compose = "1.13.0"
lifecycle = "2.11.0"
core-ktx = "1.19.1"
okhttp = "5.5.0"
kotlinx-serialization = "1.12.0"
kotlinx-coroutines = "1.11.0"
play-integrity = "1.6.0"
robolectric = "4.17"
androidx-test-core = "1.7.0"
androidx-test-ext-junit = "1.3.0"

[libraries]
# … existing …
jna = { group = "net.java.dev.jna", name = "jna", version.ref = "jna" }
activity-compose = { group = "androidx.activity", name = "activity-compose", version.ref = "activity-compose" }
lifecycle-viewmodel-compose = { group = "androidx.lifecycle", name = "lifecycle-viewmodel-compose", version.ref = "lifecycle" }
lifecycle-runtime-compose = { group = "androidx.lifecycle", name = "lifecycle-runtime-compose", version.ref = "lifecycle" }
core-ktx = { group = "androidx.core", name = "core-ktx", version.ref = "core-ktx" }
compose-material-icons-core = { group = "androidx.compose.material", name = "material-icons-core" }
compose-ui-test-junit4 = { group = "androidx.compose.ui", name = "ui-test-junit4" }
compose-ui-test-manifest = { group = "androidx.compose.ui", name = "ui-test-manifest" }
okhttp = { group = "com.squareup.okhttp3", name = "okhttp", version.ref = "okhttp" }
okhttp-coroutines = { group = "com.squareup.okhttp3", name = "okhttp-coroutines", version.ref = "okhttp" }
okhttp-mockwebserver = { group = "com.squareup.okhttp3", name = "mockwebserver3", version.ref = "okhttp" }
kotlinx-serialization-json = { group = "org.jetbrains.kotlinx", name = "kotlinx-serialization-json", version.ref = "kotlinx-serialization" }
kotlinx-coroutines-android = { group = "org.jetbrains.kotlinx", name = "kotlinx-coroutines-android", version.ref = "kotlinx-coroutines" }
kotlinx-coroutines-play-services = { group = "org.jetbrains.kotlinx", name = "kotlinx-coroutines-play-services", version.ref = "kotlinx-coroutines" }
kotlinx-coroutines-test = { group = "org.jetbrains.kotlinx", name = "kotlinx-coroutines-test", version.ref = "kotlinx-coroutines" }
play-integrity = { group = "com.google.android.play", name = "integrity", version.ref = "play-integrity" }
robolectric = { group = "org.robolectric", name = "robolectric", version.ref = "robolectric" }
androidx-test-core = { group = "androidx.test", name = "core", version.ref = "androidx-test-core" }
androidx-test-ext-junit = { group = "androidx.test.ext", name = "junit", version.ref = "androidx-test-ext-junit" }

[plugins]
# … existing …
android-application = { id = "com.android.application", version.ref = "agp" }
kotlin-serialization = { id = "org.jetbrains.kotlin.plugin.serialization", version.ref = "kotlin" }
```

`settings.gradle.kts`: `include(":designsystem", ":core", ":app")`. Root `build.gradle.kts`
adds `alias(libs.plugins.android.application) apply false` and
`alias(libs.plugins.kotlin.serialization) apply false`. (`:app` itself arrives in Task 4;
include `":core"` only in this task.)

- [ ] **Step 2: buildSrc tasks**

`android/buildSrc/build.gradle.kts`:

```kotlin
plugins { `kotlin-dsl` }
repositories { mavenCentral() }
```

`android/buildSrc/src/main/kotlin/RustTasks.kt`:

```kotlin
import javax.inject.Inject
import org.gradle.api.DefaultTask
import org.gradle.api.Project
import org.gradle.api.file.ConfigurableFileCollection
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.provider.ListProperty
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction
import org.gradle.api.tasks.testing.Test
import org.gradle.process.ExecOperations

/** The repository root: the Cargo workspace that holds `android/`. */
val Project.repoRoot get() = rootProject.layout.projectDirectory.dir("..")

/** Everything that changes the Rust library: the inputs of every cargo task. */
fun Project.rustSources(): ConfigurableFileCollection = files(
    repoRoot.file("Cargo.toml"),
    repoRoot.file("Cargo.lock"),
    repoRoot.file("rust-toolchain.toml"),
    repoRoot.dir("crates/core"),
    repoRoot.dir("crates/ffi"),
    repoRoot.dir("crates/uniffi-bindgen"),
)

/** Runs cargo in the workspace root. */
abstract class CargoTask @Inject constructor(private val exec: ExecOperations) : DefaultTask() {
    @get:InputFiles @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sources: ConfigurableFileCollection

    @get:Internal
    abstract val workspace: DirectoryProperty

    protected fun cargo(arguments: List<String>, environment: Map<String, String> = emptyMap()) {
        exec.exec {
            workingDir = workspace.get().asFile
            environment(environment)
            commandLine(listOf("cargo") + arguments)
        }
    }
}

/** `cargo ndk`: the release library for each Android ABI, laid out as `jniLibs/<abi>/`. */
abstract class CargoNdkBuild @Inject constructor(exec: ExecOperations) : CargoTask(exec) {
    @get:Input abstract val abis: ListProperty<String>
    @get:Input abstract val minSdk: org.gradle.api.provider.Property<Int>
    @get:Internal abstract val ndkDirectory: DirectoryProperty
    @get:OutputDirectory abstract val outputDir: DirectoryProperty

    @TaskAction
    fun build() {
        val targets = abis.get().flatMap { listOf("-t", it) }
        cargo(
            listOf("ndk") + targets + listOf(
                "-P", minSdk.get().toString(),
                "-o", outputDir.get().asFile.absolutePath,
                "build", "--locked", "-p", "whichride-ffi", "--profile", "android",
            ),
            mapOf("ANDROID_NDK_HOME" to ndkDirectory.get().asFile.absolutePath),
        )
    }
}

/** Kotlin bindings read from a built library (UniFFI library mode). */
abstract class UniffiBindgen @Inject constructor(exec: ExecOperations) : CargoTask(exec) {
    @get:InputFile @get:PathSensitive(PathSensitivity.NONE)
    abstract val library: RegularFileProperty
    @get:OutputDirectory abstract val outputDir: DirectoryProperty

    @TaskAction
    fun generate() {
        outputDir.get().asFile.deleteRecursively()
        cargo(
            listOf(
                "run", "--locked", "-p", "uniffi-bindgen", "--", "generate",
                "--library", library.get().asFile.absolutePath,
                "--language", "kotlin", "--no-format",
                "--out-dir", outputDir.get().asFile.absolutePath,
            ),
        )
    }
}

/** A debug build for the machine running Gradle, so JVM unit tests call the real Rust. */
abstract class CargoHostBuild @Inject constructor(exec: ExecOperations) : CargoTask(exec) {
    @get:OutputDirectory abstract val targetDir: DirectoryProperty

    @TaskAction
    fun build() = cargo(listOf("build", "--locked", "-p", "whichride-ffi"))
}

/** JVM unit tests of this module load the host build through JNA. */
fun Project.useHostRustInTests() {
    tasks.withType(Test::class.java).configureEach {
        dependsOn(":core:cargoHostBuild")
        systemProperty("jna.library.path", repoRoot.dir("target/debug").asFile.absolutePath)
    }
}
```

`android/buildSrc/src/main/kotlin/CopyIntoDirectory.kt`:

```kotlin
import org.gradle.api.DefaultTask
import org.gradle.api.file.ConfigurableFileCollection
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.provider.MapProperty
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction

/**
 * Copies files from elsewhere in the repository into a generated source directory, under the
 * names in [targets] (source file name → path inside [outputDir]), so the APK carries them
 * without a committed copy.
 */
abstract class CopyIntoDirectory : DefaultTask() {
    @get:InputFiles @get:PathSensitive(PathSensitivity.NAME_ONLY)
    abstract val sources: ConfigurableFileCollection
    @get:Input abstract val targets: MapProperty<String, String>
    @get:OutputDirectory abstract val outputDir: DirectoryProperty

    @TaskAction
    fun copy() {
        val out = outputDir.get().asFile
        out.deleteRecursively()
        val names = targets.get()
        sources.files.forEach { file ->
            val target = out.resolve(names[file.name] ?: error("no target for ${file.name}"))
            target.parentFile.mkdirs()
            file.copyTo(target, overwrite = true)
        }
    }
}
```

- [ ] **Step 3: `android/core/build.gradle.kts`**

```kotlin
plugins {
    alias(libs.plugins.android.library)
}

android {
    namespace = "io.github.locadani.whichride.core"
    compileSdk = 37
    ndkVersion = libs.versions.ndk.get()
    defaultConfig {
        minSdk = 26
        consumerProguardFiles("consumer-rules.pro")
    }
}

val abis = listOf("arm64-v8a", "armeabi-v7a", "x86_64")

val cargoNdkBuild = tasks.register<CargoNdkBuild>("cargoNdkBuild") {
    sources.from(rustSources())
    workspace.set(repoRoot)
    this.abis.set(abis)
    minSdk.set(26)
    ndkDirectory.set(androidComponents.sdkComponents.ndkDirectory)
    outputDir.set(layout.buildDirectory.dir("rust/jniLibs"))
}

val uniffiBindgen = tasks.register<UniffiBindgen>("uniffiBindgen") {
    sources.from(rustSources())
    workspace.set(repoRoot)
    library.set(cargoNdkBuild.flatMap { it.outputDir.file("arm64-v8a/libwhichride_ffi.so") })
    outputDir.set(layout.buildDirectory.dir("generated/uniffi"))
}

tasks.register<CargoHostBuild>("cargoHostBuild") {
    sources.from(rustSources())
    workspace.set(repoRoot)
    targetDir.set(repoRoot.dir("target/debug"))
}

androidComponents.onVariants { variant ->
    variant.sources.jniLibs?.addGeneratedSourceDirectory(cargoNdkBuild, CargoNdkBuild::outputDir)
    variant.sources.kotlin?.addGeneratedSourceDirectory(uniffiBindgen, UniffiBindgen::outputDir)
}

useHostRustInTests()

dependencies {
    // UniFFI's generated Kotlin calls the Rust library through JNA; the AAR carries JNA's
    // Android native dispatcher, the JAR the desktop one for JVM unit tests.
    api("${libs.jna.get()}@aar")
    testImplementation(libs.jna)
    testImplementation(libs.junit)
}
```

(`targetDir` on `CargoHostBuild` is `target/debug`, shared with other cargo builds; Gradle may
warn about overlapping outputs — if so, mark the property `@Internal` and declare
`outputs.upToDateWhen { false }` instead, since cargo is incremental itself.)

`android/core/consumer-rules.pro`:

```
# JNA reflects over its own classes and over the generated binding interfaces.
-keep class com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
-keep class io.github.locadani.whichride.core.** { *; }
-dontwarn java.awt.**
```

- [ ] **Step 4: Write the binding test** (`BindingsTest.kt`)

```kotlin
package io.github.locadani.whichride.core

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/** Calls the real Rust library (host build) through the generated bindings. */
class BindingsTest {
    private val seed = File("../../crates/server/catalogue.json").readText()
    private val trip = Trip(metres = 7_400u, seconds = 1_320u)

    @Test
    fun parsesAShortLinkAndItsLabel() {
        val text = "Markthalle Neun\nhttps://maps.app.goo.gl/x7Qa2Lm"
        assertEquals(Destination.ShortLink("https://maps.app.goo.gl/x7Qa2Lm"), parseDestination(text))
        assertEquals("Markthalle Neun", sharedLabel(text))
    }

    @Test(expected = LinkException.NotGoogleMaps::class)
    fun linkErrorsAreExceptions() {
        parseDestination("https://example.com")
    }

    @Test
    fun pricesTheSeedCatalogue() {
        val prices = Prices.fromSeed(seed)
        assertEquals(listOf("miles", "bolt", "sixt"), prices.providers().map { it.id })
        val quotes = prices.quotes(trip, 1_792_497_600L)
        assertEquals(9, quotes.size)
        assertTrue(quotes.zipWithNext().all { (a, b) -> a.totalCents <= b.totalCents })
        assertNull(quotes.first().confirmedDaysAgo)
    }

    @Test
    fun ranksOptions() {
        val prices = Prices.fromSeed(seed)
        val options = prices.options(trip, listOf(Choice.AnyOf("bolt")), 1_792_497_600L)
        assertEquals("bolt", options.ranked.single().quote.providerId)
    }

    @Test(expected = DataException::class)
    fun refusesABrokenCatalogue() {
        Prices.fromSeed("{")
    }
}
```

(Unit tests run with the module directory as working directory, so `../../crates` is the repo's.)

- [ ] **Step 5: Run**

```sh
cd android && export JAVA_HOME=/opt/homebrew/opt/openjdk@21
./gradlew :core:testDebugUnitTest :core:assembleRelease
unzip -l core/build/outputs/aar/core-release.aar | grep -E 'jni/|classes.jar'
```

Expected: tests pass; the AAR holds `jni/{arm64-v8a,armeabi-v7a,x86_64}/libwhichride_ffi.so`.
If the AAR also carries `libjnidispatch.so` for obsolete ABIs, that is filtered in `:app`
(`abiFilters`, Task 4).

- [ ] **Step 6: CI** — replace the `android` job in `.github/workflows/ci.yml`:

```yaml
  android:
    name: android (Rust core, designsystem, app)
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
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android,armv7-linux-androideabi,x86_64-linux-android
      - uses: Swatinem/rust-cache@v2
      - name: cargo-ndk
        run: cargo install cargo-ndk --version 4.1.2 --locked
      - name: NDK
        run: yes | "$ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager" "ndk;30.0.16248370" > /dev/null
      - uses: gradle/actions/setup-gradle@3f5f9adaf7d9fecd50b5935e54106014257a94e6 # v6.4.0
      - run: ./gradlew --no-daemon :designsystem:testDebugUnitTest :designsystem:lintDebug :designsystem:assembleRelease :core:testDebugUnitTest
```

(Task 11 extends the last line with `:app` tasks.)

- [ ] **Step 7: Commit** — `build(android): build the Rust core and its bindings from Gradle`

---
### Task 4: `:app` skeleton — manifest, entry intents, theme, build config

**Files:**
- Create: `android/app/build.gradle.kts`, `android/app/proguard-rules.pro`,
  `android/app/src/main/AndroidManifest.xml`, `android/app/src/debug/AndroidManifest.xml`,
  `android/app/src/debug/res/xml/network_security_config.xml`,
  `android/app/src/main/res/values/themes.xml`, `android/app/src/main/res/values-night/themes.xml`,
  `android/app/src/main/res/values/strings.xml`, `android/app/src/main/res/xml/data_extraction_rules.xml`,
  `…/whichride/WhichrideApplication.kt`, `…/whichride/MainActivity.kt`, `…/whichride/entry/SharedText.kt`,
  `android/app/src/test/resources/robolectric.properties`,
  `android/app/src/test/kotlin/io/github/locadani/whichride/entry/SharedTextTest.kt`
- Modify: `android/settings.gradle.kts` (`:app`)

**Interfaces:**
- Produces: `fun sharedText(intent: Intent?): String?`; `BuildConfig.SERVER_URL: String`
  (empty = no server), `BuildConfig.CLOUD_PROJECT_NUMBER: Long` (0 = none); assets
  `seed-catalogue.json`, `licences/Sora-OFL.txt`, `licences/JetBrainsMono-OFL.txt`;
  `MainActivity` calls `viewModel.onShared(text)` (Task 9) — in this task it only shows a
  placeholder `Text` inside `WhichrideTheme`.

- [ ] **Step 1: Write the failing test** (`SharedTextTest.kt`, Robolectric)

```kotlin
package io.github.locadani.whichride.entry

import android.content.Intent
import android.net.Uri
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SharedTextTest {
    @Test
    fun aShareCarriesItsText() {
        val intent = Intent(Intent.ACTION_SEND).setType("text/plain")
            .putExtra(Intent.EXTRA_TEXT, "Markthalle Neun\nhttps://maps.app.goo.gl/x7Qa2Lm")
        assertEquals("Markthalle Neun\nhttps://maps.app.goo.gl/x7Qa2Lm", sharedText(intent))
    }

    @Test
    fun aGeoLinkIsItsUri() {
        val intent = Intent(Intent.ACTION_VIEW, Uri.parse("geo:52.52,13.405?q=Alexanderplatz"))
        assertEquals("geo:52.52,13.405?q=Alexanderplatz", sharedText(intent))
    }

    @Test
    fun launchesAndBlankSharesCarryNothing() {
        assertNull(sharedText(Intent(Intent.ACTION_MAIN)))
        assertNull(sharedText(Intent(Intent.ACTION_SEND).putExtra(Intent.EXTRA_TEXT, "  ")))
        assertNull(sharedText(null))
    }
}
```

`src/test/resources/robolectric.properties`: `sdk=35` (Robolectric 4.17's newest bundled
SDK at or below targetSdk; raise when Robolectric supports 36).

- [ ] **Step 2: `android/app/build.gradle.kts`**

```kotlin
plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
}

/** Not secrets, but deployment-specific: set them in ~/.gradle/gradle.properties (R9). */
val serverUrl = providers.gradleProperty("whichride.serverUrl").getOrElse("")
val cloudProjectNumber = providers.gradleProperty("whichride.cloudProjectNumber").getOrElse("0")
require(serverUrl.isEmpty() || serverUrl.matches(Regex("""https://[^\s"]+|http://(localhost|10\.0\.2\.2)(:\d+)?"""))) {
    "whichride.serverUrl must be https:// (or http://localhost / http://10.0.2.2 for a local server)"
}
require(cloudProjectNumber.matches(Regex("""\d{1,20}"""))) { "whichride.cloudProjectNumber must be digits" }

android {
    namespace = "io.github.locadani.whichride"
    compileSdk = 37
    defaultConfig {
        applicationId = "io.github.locadani.whichride" // decision 41; never changes
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        buildConfigField("String", "SERVER_URL", "\"$serverUrl\"")
        buildConfigField("long", "CLOUD_PROJECT_NUMBER", "${cloudProjectNumber}L")
        // JNA also ships libraries for ABIs no phone uses any more (spike finding).
        ndk { abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64") }
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }
    testOptions {
        unitTests.isIncludeAndroidResources = true
    }
}

val bundledFiles = tasks.register<CopyIntoDirectory>("bundledFiles") {
    sources.from(
        repoRoot.file("crates/server/catalogue.json"),
        repoRoot.file("design/fonts/Sora-OFL.txt"),
        repoRoot.file("design/fonts/JetBrainsMono-OFL.txt"),
        repoRoot.file("LICENSE"),
    )
    targets.set(
        mapOf(
            "catalogue.json" to "seed-catalogue.json",
            "Sora-OFL.txt" to "licences/Sora-OFL.txt",
            "JetBrainsMono-OFL.txt" to "licences/JetBrainsMono-OFL.txt",
            "LICENSE" to "licences/whichride-LICENSE.txt",
        ),
    )
    outputDir.set(layout.buildDirectory.dir("generated/bundled"))
}

androidComponents.onVariants { variant ->
    variant.sources.assets?.addGeneratedSourceDirectory(bundledFiles, CopyIntoDirectory::outputDir)
}

useHostRustInTests()

dependencies {
    implementation(project(":designsystem"))
    implementation(project(":core"))
    implementation(platform(libs.compose.bom))
    implementation(libs.compose.material3)
    implementation(libs.compose.material.icons.core)
    implementation(libs.compose.ui.tooling.preview)
    debugImplementation(libs.compose.ui.tooling)
    implementation(libs.activity.compose)
    implementation(libs.lifecycle.viewmodel.compose)
    implementation(libs.lifecycle.runtime.compose)
    implementation(libs.core.ktx)
    implementation(libs.okhttp)
    implementation(libs.okhttp.coroutines)
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.kotlinx.coroutines.android)
    implementation(libs.kotlinx.coroutines.play.services)
    implementation(libs.play.integrity)

    testImplementation(libs.junit)
    testImplementation(libs.jna)
    testImplementation(libs.kotlinx.coroutines.test)
    testImplementation(libs.okhttp.mockwebserver)
    testImplementation(libs.robolectric)
    testImplementation(libs.androidx.test.core)
    testImplementation(libs.androidx.test.ext.junit)
    testImplementation(platform(libs.compose.bom))
    testImplementation(libs.compose.ui.test.junit4)
    debugImplementation(libs.compose.ui.test.manifest)
}
```

`android/app/proguard-rules.pro`:

```
# kotlinx.serialization: keep generated serializers of the server DTOs.
-keepattributes *Annotation*, InnerClasses
-keepclassmembers @kotlinx.serialization.Serializable class io.github.locadani.whichride.** {
    *** Companion;
    kotlinx.serialization.KSerializer serializer(...);
}
```

- [ ] **Step 3: Manifest, themes, backup rules, debug network config**

`src/main/AndroidManifest.xml`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">

    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_COARSE_LOCATION" />
    <uses-permission android:name="android.permission.ACCESS_FINE_LOCATION" />

    <!-- No backup or device transfer: nothing on the phone is worth restoring (the install
         token is tied to a Keystore key that doesn't travel, added cars expire in minutes). -->
    <application
        android:name=".WhichrideApplication"
        android:allowBackup="false"
        android:fullBackupContent="false"
        android:dataExtractionRules="@xml/data_extraction_rules"
        android:icon="@mipmap/ic_launcher"
        android:label="@string/app_name"
        android:supportsRtl="true"
        android:theme="@style/Theme.Whichride">

        <activity
            android:name=".MainActivity"
            android:exported="true"
            android:launchMode="singleTop"
            android:windowSoftInputMode="adjustResize">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
            <!-- Google Maps → Share → whichride -->
            <intent-filter>
                <action android:name="android.intent.action.SEND" />
                <category android:name="android.intent.category.DEFAULT" />
                <data android:mimeType="text/plain" />
            </intent-filter>
            <!-- geo: links from any app -->
            <intent-filter>
                <action android:name="android.intent.action.VIEW" />
                <category android:name="android.intent.category.DEFAULT" />
                <category android:name="android.intent.category.BROWSABLE" />
                <data android:scheme="geo" />
            </intent-filter>
        </activity>
    </application>
</manifest>
```

`src/debug/AndroidManifest.xml`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <application android:networkSecurityConfig="@xml/network_security_config" />
</manifest>
```

`src/debug/res/xml/network_security_config.xml`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<!-- Debug only: a local server through `adb reverse tcp:8080 tcp:8080` or the emulator's host alias. -->
<network-security-config>
    <domain-config cleartextTrafficPermitted="true">
        <domain includeSubdomains="false">localhost</domain>
        <domain includeSubdomains="false">10.0.2.2</domain>
    </domain-config>
</network-security-config>
```

`src/main/res/xml/data_extraction_rules.xml`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<data-extraction-rules>
    <cloud-backup>
        <exclude domain="root" />
        <exclude domain="file" />
        <exclude domain="database" />
        <exclude domain="sharedpref" />
        <exclude domain="external" />
    </cloud-backup>
    <device-transfer>
        <exclude domain="root" />
        <exclude domain="file" />
        <exclude domain="database" />
        <exclude domain="sharedpref" />
        <exclude domain="external" />
    </device-transfer>
</data-extraction-rules>
```

`values/themes.xml` (window only; Compose draws everything, colours come from `WhichrideTheme`):

```xml
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <style name="Theme.Whichride" parent="android:Theme.Material.Light.NoActionBar" />
</resources>
```

`values-night/themes.xml`: same with parent `android:Theme.Material.NoActionBar`.

`values/strings.xml` starts with `<string name="app_name">whichride</string>`; Task 10 adds
the rest.

- [ ] **Step 4: Implement** `entry/SharedText.kt`, `WhichrideApplication.kt`, `MainActivity.kt`

```kotlin
package io.github.locadani.whichride.entry

import android.content.Intent

/**
 * The text a Google Maps share or a `geo:` link brought, or null for a plain launch. Shares
 * carry "Place name\nhttps://maps.app.goo.gl/…"; the core finds the link in it.
 */
fun sharedText(intent: Intent?): String? = when (intent?.action) {
    Intent.ACTION_SEND -> intent.getCharSequenceExtra(Intent.EXTRA_TEXT)?.toString()
    Intent.ACTION_VIEW -> intent.dataString
    else -> null
}?.takeIf { it.isNotBlank() }
```

`WhichrideApplication.kt` (container filled in Task 9):

```kotlin
package io.github.locadani.whichride

import android.app.Application

class WhichrideApplication : Application() {
    val container: AppContainer by lazy { AppContainer(this) }
}
```

In this task `AppContainer` is `class AppContainer(val app: Application)`; `MainActivity`:

```kotlin
package io.github.locadani.whichride

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.material3.Text
import io.github.locadani.whichride.designsystem.WhichrideTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        setContent { WhichrideTheme { Text(getString(R.string.app_name)) } }
    }
}
```

- [ ] **Step 5: Run** — `./gradlew :app:testDebugUnitTest :app:assembleDebug`; then
  `unzip -l app/build/outputs/apk/debug/app-debug.apk | grep -E 'assets/|lib/'` shows the seed,
  the three licences and `libwhichride_ffi.so` for three ABIs only. `cd ../design && npm test`
  stays green.

- [ ] **Step 6: Commit** — `feat(android): app module with share-sheet and geo: entry`

---

### Task 5: Pure helpers — units, Maps URLs, added cars

**Files:**
- Create: `…/format/Units.kt`, `…/launch/MapsUrl.kt`, `…/options/Choices.kt`, `…/options/OptionsStore.kt`
- Test: `…/format/UnitsTest.kt`, `…/launch/MapsUrlTest.kt`, `…/options/ChoicesTest.kt`, `…/options/OptionsStoreTest.kt`

**Interfaces:**
- Consumes: generated `Coordinate`, `Choice`.
- Produces:
  - `fun parseKilometres(text: String): UInt?` (metres), `fun parseMinutes(text: String): UInt?` (seconds)
  - `fun formatKilometres(metres: UInt, locale: Locale): String` ("7.4"), `fun formatMinutes(seconds: UInt): String` ("22"), `fun formatMoney(cents: Long, locale: Locale): String` ("€9.69"), `fun kilometresInput(metres: UInt): String` ("7.4", for prefilling)
  - `sealed interface MapsTarget { data class At(val at: Coordinate); data class Query(val text: String); data class Link(val url: String) }`
  - `fun mapsUrl(from: Coordinate?, to: MapsTarget): String`, `fun degrees(e7: Int): String`
  - `fun Set<Choice>.forProvider(providerId: String, offerIds: Set<String>): Set<Choice>`, `fun toggle(draft: Set<Choice>, choice: Choice): Set<Choice>`,
    `fun Set<Choice>.replacingProvider(providerId: String, offerIds: Set<String>, draft: Set<Choice>): Set<Choice>`
  - `interface ChoiceStore { fun load(): Set<Choice>; fun save(choices: Set<Choice>) }`; `class OptionsStore(prefs: SharedPreferences, now: () -> Long) : ChoiceStore`

- [ ] **Step 1: Failing tests**

`UnitsTest.kt`:

```kotlin
package io.github.locadani.whichride.format

import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class UnitsTest {
    @Test fun kilometresAcceptADecimalPoint() = assertEquals(7_400u, parseKilometres("7.4"))
    @Test fun kilometresAcceptADecimalComma() = assertEquals(7_400u, parseKilometres(" 7,4 "))
    @Test fun kilometresAcceptWholeNumbersAndMetres() {
        assertEquals(12_000u, parseKilometres("12"))
        assertEquals(12_345u, parseKilometres("12.345"))
        assertEquals(500u, parseKilometres(",5"))
    }
    @Test fun kilometresRefuseNonsenseAndOutOfRange() {
        listOf("", "7.4.1", "7,4,1", "abc", "-3", "0", "0.05", "2000.001", "12.3456", "1e3", "7 4").forEach {
            assertNull(it, parseKilometres(it))
        }
        assertEquals(2_000_000u, parseKilometres("2000"))
        assertEquals(100u, parseKilometres("0.1"))
    }
    @Test fun minutesAreWholeAndBounded() {
        assertEquals(1_320u, parseMinutes(" 22 "))
        listOf("", "0", "1441", "2.5", "-1", "x").forEach { assertNull(it, parseMinutes(it)) }
        assertEquals(86_400u, parseMinutes("1440"))
    }
    @Test fun kilometresAreShownWithOneDecimalInTheLocalesStyle() {
        assertEquals("7.4", formatKilometres(7_449u, Locale.UK))
        assertEquals("7.5", formatKilometres(7_450u, Locale.UK))
        assertEquals("7,4", formatKilometres(7_400u, Locale.GERMANY))
        assertEquals("0.0", formatKilometres(0u, Locale.UK))
    }
    @Test fun prefilledKilometresKeepEveryMetre() {
        assertEquals("7.4", kilometresInput(7_400u))
        assertEquals("12.345", kilometresInput(12_345u))
        assertEquals("12", kilometresInput(12_000u))
    }
    @Test fun minutesAreRoundedUp() {
        assertEquals("22", formatMinutes(1_320u))
        assertEquals("23", formatMinutes(1_321u))
        assertEquals("0", formatMinutes(0u))
    }
    @Test fun moneyIsEurosInTheLocalesStyle() {
        assertEquals("€9.69", formatMoney(969, Locale.UK))
        assertEquals("9,69 €", formatMoney(969, Locale.GERMANY))
        assertEquals("€0.00", formatMoney(0, Locale.UK))
    }
}
```

`MapsUrlTest.kt`:

```kotlin
package io.github.locadani.whichride.launch

import io.github.locadani.whichride.core.Coordinate
import org.junit.Assert.assertEquals
import org.junit.Test

class MapsUrlTest {
    private val alex = Coordinate(latE7 = 525_219_000, lonE7 = 134_132_000)
    private val neun = Coordinate(latE7 = 525_022_870, lonE7 = 134_316_050)

    @Test fun degreesAreExactWithoutTrailingZeros() {
        assertEquals("52.5219", degrees(525_219_000))
        assertEquals("-0.5", degrees(-5_000_000))
        assertEquals("-0.0000001", degrees(-1))
        assertEquals("0", degrees(0))
        assertEquals("-180", degrees(-1_800_000_000))
    }

    @Test fun directionsBetweenTwoPoints() = assertEquals(
        "https://www.google.com/maps/dir/?api=1&origin=52.5219,13.4132&destination=52.502287,13.431605&travelmode=driving",
        mapsUrl(alex, MapsTarget.At(neun)),
    )

    @Test fun withoutAStartMapsUsesTheCurrentLocation() = assertEquals(
        "https://www.google.com/maps/dir/?api=1&destination=52.502287,13.431605&travelmode=driving",
        mapsUrl(null, MapsTarget.At(neun)),
    )

    @Test fun aPlaceNameIsEncoded() = assertEquals(
        "https://www.google.com/maps/dir/?api=1&origin=52.5219,13.4132&destination=Markthalle%20Neun%2C%20Berlin&travelmode=driving",
        mapsUrl(alex, MapsTarget.Query("Markthalle Neun, Berlin")),
    )

    @Test fun anUnresolvedShortLinkOpensAsItIs() = assertEquals(
        "https://maps.app.goo.gl/x7Qa2Lm",
        mapsUrl(alex, MapsTarget.Link("https://maps.app.goo.gl/x7Qa2Lm")),
    )
}
```

`ChoicesTest.kt`:

```kotlin
package io.github.locadani.whichride.options

import io.github.locadani.whichride.core.Choice
import org.junit.Assert.assertEquals
import org.junit.Test

class ChoicesTest {
    private val polo = Choice.Offer("bolt.vw-polo")
    private val yaris = Choice.Offer("bolt.toyota-yaris")
    private val anyBolt = Choice.AnyOf("bolt")
    private val milesS = Choice.Offer("miles.s")
    private val boltOffers = setOf("bolt.vw-polo", "bolt.toyota-yaris", "bolt.toyota-yaris-cross")

    @Test fun tickingACarAddsIt() = assertEquals(setOf(polo, yaris), toggle(setOf(polo), yaris))
    @Test fun tickingItAgainRemovesIt() = assertEquals(setOf(polo), toggle(setOf(polo, yaris), yaris))
    @Test fun anyCarReplacesTheModels() = assertEquals(setOf(anyBolt), toggle(setOf(polo, yaris), anyBolt))
    @Test fun aModelReplacesAnyCar() = assertEquals(setOf(polo), toggle(setOf(anyBolt), polo))
    @Test fun untickingAnyCarLeavesNothing() = assertEquals(emptySet<Choice>(), toggle(setOf(anyBolt), anyBolt))

    @Test fun aProvidersChoicesAreItsOffersAndItsAnyCar() =
        assertEquals(setOf(polo, anyBolt), setOf(polo, anyBolt, milesS).forProvider("bolt", boltOffers))

    @Test fun committingReplacesOnlyThatProvider() = assertEquals(
        setOf(milesS, anyBolt),
        setOf(milesS, polo, yaris).replacingProvider("bolt", boltOffers, setOf(anyBolt)),
    )
}
```

(`forProvider` takes the provider's offer IDs too, since an offer ID alone doesn't say its
provider; the Interfaces line above reads `forProvider(providerId, offerIds)`.)

`OptionsStoreTest.kt` (Robolectric for real `SharedPreferences`):

```kotlin
package io.github.locadani.whichride.options

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.core.Choice
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class OptionsStoreTest {
    private var now = 1_000_000L
    private val prefs = ApplicationProvider.getApplicationContext<Context>()
        .getSharedPreferences("options-test", Context.MODE_PRIVATE)
    private val store = OptionsStore(prefs) { now }
    private val choices = setOf(Choice.Offer("bolt.vw-polo"), Choice.AnyOf("miles"))

    @Test fun keepsTheCarsForFifteenMinutes() {
        store.save(choices)
        now += 15 * 60_000L - 1
        assertEquals(choices, store.load())
    }

    @Test fun optionsExpire15MinutesAfterTheLastChange() {
        store.save(choices)
        now += 15 * 60_000L
        assertEquals(emptySet<Choice>(), store.load())
        assertEquals(emptySet<Choice>(), OptionsStore(prefs) { now - 60_000L }.load())
    }

    @Test fun aWrongClockGoingBackwardsExpiresThem() {
        store.save(choices)
        now -= 1
        assertEquals(emptySet<Choice>(), store.load())
    }

    @Test fun survivesANewStoreInstance() {
        store.save(choices)
        assertEquals(choices, OptionsStore(prefs) { now }.load())
    }

    @Test fun unreadableEntriesAreDropped() {
        prefs.edit().putStringSet("choices", setOf("offer:bolt.vw-polo", "garbage")).putLong("changed_at", now).commit()
        assertEquals(setOf(Choice.Offer("bolt.vw-polo")), store.load())
    }
}
```

- [ ] **Step 2: Run, verify failures** — `./gradlew :app:testDebugUnitTest` (unresolved references).

- [ ] **Step 3: Implement**

`format/Units.kt`:

```kotlin
package io.github.locadani.whichride.format

import java.math.BigDecimal
import java.text.DecimalFormatSymbols
import java.text.NumberFormat
import java.util.Currency
import java.util.Locale

private val KILOMETRES = Regex("""(\d{0,4})(?:[.,](\d{1,3}))?""")
private const val MAX_METRES = 2_000_000u
private const val MIN_METRES = 100u
private const val MAX_MINUTES = 1_440u

/** Kilometres the user typed ("7.4", "7,4", "12") as metres, with integer maths (R24). */
fun parseKilometres(text: String): UInt? {
    val match = KILOMETRES.matchEntire(text.trim()) ?: return null
    val (whole, fraction) = match.destructured
    if (whole.isEmpty() && fraction.isEmpty()) return null
    val metres = (whole.ifEmpty { "0" }.toUInt() * 1_000u) + fraction.padEnd(3, '0').ifEmpty { "0" }.toUInt()
    return metres.takeIf { it in MIN_METRES..MAX_METRES }
}

/** Whole minutes the user typed, as seconds. */
fun parseMinutes(text: String): UInt? =
    text.trim().takeIf { it.all(Char::isDigit) && it.length in 1..4 }?.toUInt()
        ?.takeIf { it in 1u..MAX_MINUTES }?.let { it * 60u }

/** "7.4" (one decimal, half up, the locale's separator); the unit comes from strings.xml. */
fun formatKilometres(metres: UInt, locale: Locale): String {
    val tenths = (metres + 50u) / 100u
    return "${tenths / 10u}${DecimalFormatSymbols.getInstance(locale).decimalSeparator}${tenths % 10u}"
}

/** The value to prefill the km field with: every metre, no trailing zeros, a dot. */
fun kilometresInput(metres: UInt): String {
    val fraction = (metres % 1_000u).toString().padStart(3, '0').trimEnd('0')
    return if (fraction.isEmpty()) "${metres / 1_000u}" else "${metres / 1_000u}.$fraction"
}

/** Started minutes, as providers bill them. */
fun formatMinutes(seconds: UInt): String = ((seconds + 59u) / 60u).toString()

fun formatMoney(cents: Long, locale: Locale): String =
    NumberFormat.getCurrencyInstance(locale).apply { currency = Currency.getInstance("EUR") }
        .format(BigDecimal.valueOf(cents, 2))
```

`launch/MapsUrl.kt`:

```kotlin
package io.github.locadani.whichride.launch

import io.github.locadani.whichride.core.Coordinate
import java.net.URLEncoder
import kotlin.math.absoluteValue

/** What Google Maps should show as the destination. */
sealed interface MapsTarget {
    data class At(val at: Coordinate) : MapsTarget
    /** A place name we couldn't turn into coordinates; Maps searches it. */
    data class Query(val text: String) : MapsTarget
    /** A short link we couldn't expand; Maps opens it itself. */
    data class Link(val url: String) : MapsTarget
}

/** Maps URLs (R16): driving directions, from the start when we know it. */
fun mapsUrl(from: Coordinate?, to: MapsTarget): String {
    val destination = when (to) {
        is MapsTarget.Link -> return to.url
        is MapsTarget.At -> pair(to.at)
        is MapsTarget.Query -> URLEncoder.encode(to.text, Charsets.UTF_8).replace("+", "%20")
    }
    val origin = from?.let { "&origin=${pair(it)}" }.orEmpty()
    return "https://www.google.com/maps/dir/?api=1$origin&destination=$destination&travelmode=driving"
}

private fun pair(c: Coordinate) = "${degrees(c.latE7)},${degrees(c.lonE7)}"

/** 1e-7 degrees as a decimal string, exactly (no floating point), without trailing zeros. */
fun degrees(e7: Int): String {
    val magnitude = e7.toLong().absoluteValue
    val fraction = (magnitude % 10_000_000).toString().padStart(7, '0').trimEnd('0')
    val sign = if (e7 < 0) "-" else ""
    return sign + (magnitude / 10_000_000) + if (fraction.isEmpty()) "" else ".$fraction"
}
```

`options/Choices.kt`:

```kotlin
package io.github.locadani.whichride.options

import io.github.locadani.whichride.core.Choice

/** The choices that belong to one provider: its offers and its "any car". */
fun Set<Choice>.forProvider(providerId: String, offerIds: Set<String>): Set<Choice> = filterTo(mutableSetOf()) {
    when (it) {
        is Choice.AnyOf -> it.providerId == providerId
        is Choice.Offer -> it.offerId in offerIds
    }
}

/**
 * One tick in step 2 (§4.1), within one provider's draft: "any car" and specific models
 * exclude each other, as in the prototype.
 */
fun toggle(draft: Set<Choice>, choice: Choice): Set<Choice> = when {
    choice in draft -> draft - choice
    choice is Choice.AnyOf -> setOf(choice)
    else -> draft.filterNotTo(mutableSetOf()) { it is Choice.AnyOf } + choice
}

/** "Add N cars": the provider's choices become the draft; other providers are untouched. */
fun Set<Choice>.replacingProvider(providerId: String, offerIds: Set<String>, draft: Set<Choice>): Set<Choice> =
    this - forProvider(providerId, offerIds) + draft
```

`options/OptionsStore.kt`:

```kotlin
package io.github.locadani.whichride.options

import android.content.SharedPreferences
import io.github.locadani.whichride.core.Choice

/**
 * The cars the user added (spec §4.1, R18): offer and provider IDs only, kept on the device and
 * dropped 15 minutes after the last change (cars move). Never sent anywhere.
 */
/** The added cars between launches; the ViewModel's tests fake it. */
interface ChoiceStore {
    fun load(): Set<Choice>
    fun save(choices: Set<Choice>)
}

class OptionsStore(private val prefs: SharedPreferences, private val now: () -> Long) : ChoiceStore {
    override fun load(): Set<Choice> {
        val changedAt = prefs.getLong(CHANGED_AT, Long.MIN_VALUE)
        val age = now() - changedAt
        if (changedAt == Long.MIN_VALUE || age < 0 || age >= LIFETIME_MILLIS) {
            clear()
            return emptySet()
        }
        return prefs.getStringSet(CHOICES, emptySet()).orEmpty().mapNotNullTo(mutableSetOf(), ::decode)
    }

    override fun save(choices: Set<Choice>) {
        if (choices.isEmpty()) return clear()
        prefs.edit().putStringSet(CHOICES, choices.mapTo(mutableSetOf(), ::encode)).putLong(CHANGED_AT, now()).apply()
    }

    private fun clear() = prefs.edit().remove(CHOICES).remove(CHANGED_AT).apply()

    private fun encode(choice: Choice) = when (choice) {
        is Choice.Offer -> "$OFFER${choice.offerId}"
        is Choice.AnyOf -> "$ANY${choice.providerId}"
    }

    private fun decode(text: String): Choice? = when {
        text.startsWith(OFFER) -> Choice.Offer(text.removePrefix(OFFER))
        text.startsWith(ANY) -> Choice.AnyOf(text.removePrefix(ANY))
        else -> null
    }

    companion object {
        const val LIFETIME_MILLIS = 15 * 60_000L
        private const val CHOICES = "choices"
        private const val CHANGED_AT = "changed_at"
        private const val OFFER = "offer:"
        private const val ANY = "any:"
    }
}
```

- [ ] **Step 4: Run** `./gradlew :app:testDebugUnitTest` — all pass. (If the JDK's CLDR data
  formats `de` currency differently, adjust only the expected string, after printing it.)

- [ ] **Step 5: Commit** — `feat(android): units, Maps links and the added cars store`

---

### Task 6: Server client and install registration

**Files:**
- Create: `…/server/ApiResult.kt`, `…/server/ServerApi.kt`, `…/server/Installs.kt`,
  `…/server/IntegrityTokens.kt`, `…/server/TokenStore.kt`, `…/server/Routes.kt`
- Test: `…/server/ServerApiTest.kt`, `…/server/InstallsTest.kt`, `…/server/ServerRoutesTest.kt`

**Interfaces:**
- Produces:
  - `sealed interface ApiResult<out T> { Ok(value); Unavailable(why: Why); Refused(status: Int, code: String) }`, `enum class Why { NOT_CONFIGURED, OFFLINE, BUSY, NOT_REGISTERED }`
  - `data class Place(val label: String, val at: Coordinate)`
  - `class ServerApi(baseUrl: String, http: OkHttpClient)`: `catalogue(): ApiResult<String>`,
    `register(body: String, integrityToken: String): ApiResult<String>` (the token),
    `route(bearer, from, to): ApiResult<Trip>`, `geocode(bearer, text, near): ApiResult<List<Place>>`,
    `autocomplete(bearer, text, near): ApiResult<List<Place>>`, `expand(bearer, url): ApiResult<String>` (Destination JSON)
  - `fun interface IntegrityTokens { suspend fun token(requestHash: String): String? }`; `PlayIntegrityTokens(context, cloudProjectNumber)`
  - `interface TokenStore { fun load(): String?; fun save(token: String); fun clear() }`; `KeystoreTokenStore(prefs)`
  - `class Installs(api, integrity, store, nonce: () -> String)`: `suspend fun <T> authorized(call: suspend (String) -> ApiResult<T>): ApiResult<T>`
  - `interface Routes` (used by the ViewModel): `route(from, to): ApiResult<Trip>`, `geocode(text, near): ApiResult<List<Place>>`,
    `autocomplete(text, near): ApiResult<List<Place>>`, `expand(url): ApiResult<Destination>`; `class ServerRoutes(api, installs) : Routes`
  - `fun registrationHash(body: String): String` (lowercase hex SHA-256 of `"POST /v1/installs\n" + body`)

- [ ] **Step 1: Failing tests** — `ServerApiTest.kt` (MockWebServer is the fake server; plain JVM)

```kotlin
package io.github.locadani.whichride.server

import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.Trip
import kotlinx.coroutines.test.runTest
import mockwebserver3.MockResponse
import mockwebserver3.MockWebServer
import okhttp3.OkHttpClient
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Before
import org.junit.Test

class ServerApiTest {
    private val server = MockWebServer()
    private lateinit var api: ServerApi
    private val alex = Coordinate(525_219_000, 134_132_000)
    private val neun = Coordinate(525_022_870, 134_316_050)

    @Before fun start() {
        server.start()
        api = ServerApi(server.url("/").toString().trimEnd('/'), OkHttpClient())
    }

    @After fun stop() = server.close()

    private fun reply(status: Int, body: String) =
        server.enqueue(MockResponse.Builder().code(status).body(body).build())

    @Test fun routeSendsCoordinatesInTheBodyWithTheBearer() = runTest {
        reply(200, """{"metres":7400,"seconds":1320}""")
        assertEquals(ApiResult.Ok(Trip(7_400u, 1_320u)), api.route("tok", alex, neun))
        val request = server.takeRequest()
        assertEquals("POST", request.method)
        assertEquals("/v1/route", request.url.encodedPath)
        assertFalse("no coordinates in the URL (§9)", request.url.toString().contains("52"))
        assertEquals("Bearer tok", request.headers["Authorization"])
        assertEquals(
            """{"from":{"lat_e7":525219000,"lon_e7":134132000},"to":{"lat_e7":525022870,"lon_e7":134316050}}""",
            request.body!!.utf8(),
        )
    }

    @Test fun searchesSendNearOnlyWhenKnown() = runTest {
        reply(200, """{"places":[{"label":"Markthalle Neun","at":{"lat_e7":525022870,"lon_e7":134316050}}]}""")
        reply(200, """{"places":[]}""")
        assertEquals(ApiResult.Ok(listOf(Place("Markthalle Neun", neun))), api.autocomplete("tok", "Markth", alex))
        assertEquals(ApiResult.Ok(emptyList<Place>()), api.geocode("tok", "Nowhere", null))
        assertEquals("""{"text":"Markth","near":{"lat_e7":525219000,"lon_e7":134132000}}""", server.takeRequest().body!!.utf8())
        val geocode = server.takeRequest()
        assertEquals("/v1/geocode", geocode.url.encodedPath)
        assertEquals("""{"text":"Nowhere"}""", geocode.body!!.utf8())
    }

    @Test fun errorsMapToWhatTheAppDoesNext() = runTest {
        reply(503, """{"error":"route_service_busy"}""")
        reply(429, """{"error":"too_many_requests"}""")
        reply(500, """{"error":"internal"}""")
        reply(422, """{"error":"no_route"}""")
        reply(401, """{"error":"unauthorized"}""")
        reply(422, "not json")
        assertEquals(ApiResult.Unavailable(Why.BUSY), api.route("t", alex, neun))
        assertEquals(ApiResult.Unavailable(Why.BUSY), api.route("t", alex, neun))
        assertEquals(ApiResult.Unavailable(Why.BUSY), api.route("t", alex, neun))
        assertEquals(ApiResult.Refused(422, "no_route"), api.route("t", alex, neun))
        assertEquals(ApiResult.Refused(401, "unauthorized"), api.route("t", alex, neun))
        assertEquals(ApiResult.Refused(422, "unknown"), api.route("t", alex, neun))
    }

    @Test fun aServerThatIsDownIsOffline() = runTest {
        server.close()
        assertEquals(ApiResult.Unavailable(Why.OFFLINE), api.route("t", alex, neun))
    }

    @Test fun noServerUrlMeansNotConfigured() = runTest {
        val none = ServerApi("", OkHttpClient())
        assertEquals(ApiResult.Unavailable(Why.NOT_CONFIGURED), none.catalogue())
        assertEquals(ApiResult.Unavailable(Why.NOT_CONFIGURED), none.route("t", alex, neun))
    }

    @Test fun catalogueAndExpandReturnTheRawJsonForTheCore() = runTest {
        reply(200, """{"catalogue":{"providers":[]},"offers":[],"proposals":[]}""")
        reply(200, """{"type":"place","query":"Mauerpark","near":null}""")
        assertEquals(ApiResult.Ok("""{"catalogue":{"providers":[]},"offers":[],"proposals":[]}"""), api.catalogue())
        assertEquals(ApiResult.Ok("""{"type":"place","query":"Mauerpark","near":null}"""), api.expand("t", "https://maps.app.goo.gl/x"))
        assertEquals("GET", server.takeRequest().method)
        assertEquals("""{"url":"https://maps.app.goo.gl/x"}""", server.takeRequest().body!!.utf8())
    }

    @Test fun registrationSendsTheIntegrityTokenAndReturnsTheBearer() = runTest {
        reply(201, """{"install_id":"123e4567-e89b-42d3-a456-426614174000","token":"bearer-1"}""")
        assertEquals(ApiResult.Ok("bearer-1"), api.register("""{"nonce":"abcdefghijklmnop"}""", "int.tok.en.x.y"))
        val request = server.takeRequest()
        assertEquals("int.tok.en.x.y", request.headers["X-Integrity-Token"])
        assertEquals("""{"nonce":"abcdefghijklmnop"}""", request.body!!.utf8())
    }

    @Test fun registrationHashMatchesTheServersKnownValue() = assertEquals(
        "c59b7c5ccbba73f1c1f2c526838ae9486eaaf2ac726b320a170aad41b31565c3",
        registrationHash("""{"nonce":"abcdefghijklmnop"}"""),
    )
}
```

`InstallsTest.kt` (fakes; MockWebServer for the HTTP side):

```kotlin
package io.github.locadani.whichride.server

import kotlinx.coroutines.test.runTest
import mockwebserver3.MockResponse
import mockwebserver3.MockWebServer
import okhttp3.OkHttpClient
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test

class InstallsTest {
    private val server = MockWebServer()
    private lateinit var api: ServerApi
    private val store = object : TokenStore {
        var token: String? = null
        override fun load() = token
        override fun save(token: String) { this.token = token }
        override fun clear() { token = null }
    }
    private val hashes = mutableListOf<String>()
    private val integrity = IntegrityTokens { hash -> hashes += hash; "a.b.c.d.e" }

    @Before fun start() {
        server.start()
        api = ServerApi(server.url("/").toString().trimEnd('/'), OkHttpClient())
    }

    @After fun stop() = server.close()

    private fun reply(status: Int, body: String) =
        server.enqueue(MockResponse.Builder().code(status).body(body).build())

    private fun installs(tokens: IntegrityTokens = integrity) = Installs(api, tokens, store) { "abcdefghijklmnop" }

    @Test fun registersOnceThenReusesTheStoredToken() = runTest {
        reply(201, """{"install_id":"x","token":"bearer-1"}""")
        val installs = installs()
        assertEquals(ApiResult.Ok("bearer-1"), installs.authorized { ApiResult.Ok(it) })
        assertEquals(ApiResult.Ok("bearer-1"), installs.authorized { ApiResult.Ok(it) })
        assertEquals(1, server.requestCount)
        assertEquals(listOf(registrationHash("""{"nonce":"abcdefghijklmnop"}""")), hashes)
        assertEquals("bearer-1", store.token)
    }

    @Test fun anUnknownTokenRegistersAgainOnce() = runTest {
        store.token = "stale"
        reply(201, """{"install_id":"x","token":"bearer-2"}""")
        val seen = mutableListOf<String>()
        val result = installs().authorized { bearer ->
            seen += bearer
            if (bearer == "stale") ApiResult.Refused(401, "unauthorized") else ApiResult.Ok(bearer)
        }
        assertEquals(ApiResult.Ok("bearer-2"), result)
        assertEquals(listOf("stale", "bearer-2"), seen)
    }

    @Test fun withoutPlayIntegrityThereIsNoInstall() = runTest {
        val result = installs { null }.authorized { ApiResult.Ok(it) }
        assertEquals(ApiResult.Unavailable(Why.NOT_REGISTERED), result)
        assertEquals(0, server.requestCount)
    }

    @Test fun aRefusedRegistrationIsNotRegistered() = runTest {
        reply(403, """{"error":"integrity_failed"}""")
        assertEquals(ApiResult.Unavailable(Why.NOT_REGISTERED), installs().authorized { ApiResult.Ok(it) })
        assertNull(store.token)
    }

    @Test fun aBusyRegistrationIsBusy() = runTest {
        reply(503, """{"error":"integrity_unavailable"}""")
        assertEquals(ApiResult.Unavailable(Why.BUSY), installs().authorized { ApiResult.Ok(it) })
    }
}
```

`ServerRoutesTest.kt`: one test that `expand` turns the server's Destination JSON into the
generated `Destination` through `destinationFromJson`, and that an invalid body becomes
`Refused(200, "invalid_destination")`:

```kotlin
package io.github.locadani.whichride.server

import io.github.locadani.whichride.core.Destination
import kotlinx.coroutines.test.runTest
import mockwebserver3.MockResponse
import mockwebserver3.MockWebServer
import okhttp3.OkHttpClient
import org.junit.Assert.assertEquals
import org.junit.Test

class ServerRoutesTest {
    @Test fun expandReturnsTheCoresDestination() = runTest {
        MockWebServer().use { server ->
            server.enqueue(MockResponse.Builder().body("""{"install_id":"x","token":"b"}""").code(201).build())
            server.enqueue(MockResponse.Builder().body("""{"type":"place","query":"Mauerpark","near":null}""").build())
            server.enqueue(MockResponse.Builder().body("""{"type":"nonsense"}""").build())
            server.start()
            val api = ServerApi(server.url("/").toString().trimEnd('/'), OkHttpClient())
            val store = object : TokenStore {
                var t: String? = null
                override fun load() = t
                override fun save(token: String) { t = token }
                override fun clear() { t = null }
            }
            val routes = ServerRoutes(api, Installs(api, { "a.b.c.d.e" }, store) { "abcdefghijklmnop" })
            assertEquals(ApiResult.Ok(Destination.Place("Mauerpark", null)), routes.expand("https://maps.app.goo.gl/x"))
            assertEquals(ApiResult.Refused(200, "invalid_destination"), routes.expand("https://maps.app.goo.gl/y"))
        }
    }
}
```

(These tests load the Rust library through JNA — `useHostRustInTests()` applies to `:app`.)

- [ ] **Step 2: Run, verify failures.**

- [ ] **Step 3: Implement**

`server/ApiResult.kt`:

```kotlin
package io.github.locadani.whichride.server

/** A server call's outcome, already sorted by what the app does next (R25). */
sealed interface ApiResult<out T> {
    data class Ok<T>(val value: T) : ApiResult<T>
    /** The service can't answer now; the comparison continues with typed km and minutes. */
    data class Unavailable(val why: Why) : ApiResult<Nothing>
    /** The server refused this request (`code` from crates/server/README.md). */
    data class Refused(val status: Int, val code: String) : ApiResult<Nothing>
}

enum class Why {
    /** No server URL in this build (R9). */
    NOT_CONFIGURED,
    /** No connection, or the server didn't answer. */
    OFFLINE,
    /** 429, 503 or another 5xx: quota used up, rate limited or down. */
    BUSY,
    /** Play Integrity unavailable or refused: this install can't register (R9). */
    NOT_REGISTERED,
}

inline fun <T, R> ApiResult<T>.map(transform: (T) -> R): ApiResult<R> = when (this) {
    is ApiResult.Ok -> ApiResult.Ok(transform(value))
    is ApiResult.Unavailable -> this
    is ApiResult.Refused -> this
}
```

`server/ServerApi.kt`:

```kotlin
package io.github.locadani.whichride.server

import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.Trip
import java.io.IOException
import java.security.MessageDigest
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.coroutines.executeAsync

data class Place(val label: String, val at: Coordinate)

@Serializable private data class CoordinateDto(@SerialName("lat_e7") val latE7: Int, @SerialName("lon_e7") val lonE7: Int)
@Serializable private data class RouteBody(val from: CoordinateDto, val to: CoordinateDto)
@Serializable private data class SearchBody(val text: String, val near: CoordinateDto? = null)
@Serializable private data class ExpandBody(val url: String)
@Serializable private data class TripDto(val metres: Long, val seconds: Long)
@Serializable private data class PlaceDto(val label: String, val at: CoordinateDto)
@Serializable private data class PlacesDto(val places: List<PlaceDto>)
@Serializable private data class RegistrationDto(val token: String)
@Serializable private data class ErrorDto(val error: String)
@Serializable data class NonceBody(val nonce: String)

private fun Coordinate.dto() = CoordinateDto(latE7, lonE7)
private fun CoordinateDto.core() = Coordinate(latE7, lonE7)

/** Lowercase hex SHA-256 of the registration's integrity binding (decision 44). */
fun registrationHash(body: String): String =
    MessageDigest.getInstance("SHA-256").digest("POST /v1/installs\n$body".toByteArray())
        .joinToString("") { "%02x".format(it) }

/**
 * The whichride server's HTTP API (crates/server/README.md). Coordinates always travel in
 * request bodies, never in URLs (§9). `baseUrl` empty = this build has no server (R9).
 */
class ServerApi(private val baseUrl: String, private val http: OkHttpClient) {
    private val json = Json { ignoreUnknownKeys = true; explicitNulls = false }

    suspend fun catalogue(): ApiResult<String> = call(Request.Builder().url(url("/v1/catalogue")).get()) { it }

    suspend fun register(body: String, integrityToken: String): ApiResult<String> =
        call(post("/v1/installs", body).header("X-Integrity-Token", integrityToken)) {
            json.decodeFromString<RegistrationDto>(it).token
        }

    suspend fun route(bearer: String, from: Coordinate, to: Coordinate): ApiResult<Trip> =
        call(authed("/v1/route", json.encodeToString(RouteBody(from.dto(), to.dto())), bearer)) {
            val trip = json.decodeFromString<TripDto>(it)
            Trip(trip.metres.toUInt(), trip.seconds.toUInt())
        }

    suspend fun geocode(bearer: String, text: String, near: Coordinate?) = search("/v1/geocode", bearer, text, near)

    suspend fun autocomplete(bearer: String, text: String, near: Coordinate?) = search("/v1/autocomplete", bearer, text, near)

    /** The expanded destination as the server's JSON; the core decodes it. */
    suspend fun expand(bearer: String, shortLink: String): ApiResult<String> =
        call(authed("/v1/expand", json.encodeToString(ExpandBody(shortLink)), bearer)) { it }

    private suspend fun search(path: String, bearer: String, text: String, near: Coordinate?): ApiResult<List<Place>> =
        call(authed(path, json.encodeToString(SearchBody(text, near?.dto())), bearer)) { body ->
            json.decodeFromString<PlacesDto>(body).places.map { Place(it.label, it.at.core()) }
        }

    private fun url(path: String) = "$baseUrl$path"

    private fun post(path: String, body: String) =
        Request.Builder().url(url(path)).post(body.toRequestBody(JSON))

    private fun authed(path: String, body: String, bearer: String) = post(path, body).header("Authorization", "Bearer $bearer")

    private suspend fun <T> call(request: Request.Builder, read: (String) -> T): ApiResult<T> {
        if (baseUrl.isEmpty()) return ApiResult.Unavailable(Why.NOT_CONFIGURED)
        return try {
            http.newCall(request.build()).executeAsync().use { response ->
                val body = response.body.string()
                when {
                    response.isSuccessful -> runCatching { ApiResult.Ok(read(body)) }
                        .getOrElse { ApiResult.Refused(response.code, "unreadable") }
                    response.code == 429 || response.code >= 500 -> ApiResult.Unavailable(Why.BUSY)
                    else -> ApiResult.Refused(
                        response.code,
                        runCatching { json.decodeFromString<ErrorDto>(body).error }.getOrDefault("unknown"),
                    )
                }
            }
        } catch (_: IOException) {
            ApiResult.Unavailable(Why.OFFLINE)
        } catch (_: IllegalArgumentException) { // a malformed base URL
            ApiResult.Unavailable(Why.NOT_CONFIGURED)
        }
    }

    companion object {
        private val JSON = "application/json".toMediaType()
    }
}
```

`server/IntegrityTokens.kt`:

```kotlin
package io.github.locadani.whichride.server

import android.content.Context
import com.google.android.play.core.integrity.IntegrityManagerFactory
import com.google.android.play.core.integrity.StandardIntegrityManager.PrepareIntegrityTokenRequest
import com.google.android.play.core.integrity.StandardIntegrityManager.StandardIntegrityTokenProvider
import com.google.android.play.core.integrity.StandardIntegrityManager.StandardIntegrityTokenRequest
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.coroutines.tasks.await

/** A Play Integrity token bound to `requestHash`, or null when Play can't give one. */
fun interface IntegrityTokens {
    suspend fun token(requestHash: String): String?
}

/** Play Integrity standard requests (decision 33); the provider is prepared once and re-prepared if it expired. */
class PlayIntegrityTokens(context: Context, private val cloudProjectNumber: Long) : IntegrityTokens {
    private val manager = IntegrityManagerFactory.createStandard(context.applicationContext)
    private val mutex = Mutex()
    private var provider: StandardIntegrityTokenProvider? = null

    override suspend fun token(requestHash: String): String? =
        request(requestHash) ?: run { mutex.withLock { provider = null }; request(requestHash) }

    private suspend fun request(requestHash: String): String? = runCatching {
        val provider = mutex.withLock {
            provider ?: manager.prepareIntegrityToken(
                PrepareIntegrityTokenRequest.builder().setCloudProjectNumber(cloudProjectNumber).build(),
            ).await().also { provider = it }
        }
        provider.request(StandardIntegrityTokenRequest.builder().setRequestHash(requestHash).build()).await().token()
    }.getOrNull()
}
```

(`runCatching` here must not swallow `CancellationException`: rethrow it —
`.onFailure { if (it is CancellationException) throw it }` before `.getOrNull()`.)

`server/TokenStore.kt`:

```kotlin
package io.github.locadani.whichride.server

import android.content.SharedPreferences
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/** Where the install's bearer token lives between launches. */
interface TokenStore {
    fun load(): String?
    fun save(token: String)
    fun clear()
}

/**
 * The bearer token encrypted with an AndroidKeyStore AES-256-GCM key (decision 32, R10). If the
 * key is gone (a restored or migrated phone) the token can't be read and the app registers again.
 */
class KeystoreTokenStore(private val prefs: SharedPreferences) : TokenStore {
    override fun load(): String? = runCatching {
        val stored = prefs.getString(KEY, null) ?: return null
        val (iv, data) = stored.split(':').map { Base64.decode(it, Base64.NO_WRAP) }
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, iv))
        String(cipher.doFinal(data), Charsets.UTF_8)
    }.getOrElse { clear(); null }

    override fun save(token: String) {
        val cipher = Cipher.getInstance(TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key())
        val data = cipher.doFinal(token.toByteArray(Charsets.UTF_8))
        val encoded = listOf(cipher.iv, data).joinToString(":") { Base64.encodeToString(it, Base64.NO_WRAP) }
        prefs.edit().putString(KEY, encoded).apply()
    }

    override fun clear() = prefs.edit().remove(KEY).apply()

    private fun key(): SecretKey {
        val keyStore = KeyStore.getInstance(ANDROID_KEYSTORE).apply { load(null) }
        (keyStore.getKey(ALIAS, null) as? SecretKey)?.let { return it }
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEYSTORE).apply {
            init(
                KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                    .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                    .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                    .setKeySize(256)
                    .build(),
            )
        }.generateKey()
    }

    private companion object {
        const val ANDROID_KEYSTORE = "AndroidKeyStore"
        const val ALIAS = "whichride.install-token"
        const val TRANSFORMATION = "AES/GCM/NoPadding"
        const val KEY = "install_token"
    }
}
```

(`KeystoreTokenStore` needs a real AndroidKeyStore, which Robolectric doesn't provide; it is
covered by the on-device check in Task 11, not by a unit test. Recorded as a deferred minor.)

`server/Installs.kt`:

```kotlin
package io.github.locadani.whichride.server

import java.security.SecureRandom
import java.util.Base64
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import kotlinx.serialization.json.Json

/**
 * This install's identity on the server (decisions 32–33, 44): registers once with a Play
 * Integrity token bound to a fresh nonce, keeps the bearer token, and registers again once if
 * the server no longer knows it.
 */
class Installs(
    private val api: ServerApi,
    private val integrity: IntegrityTokens,
    private val store: TokenStore,
    private val nonce: () -> String = ::randomNonce,
) {
    private val mutex = Mutex()

    suspend fun <T> authorized(call: suspend (bearer: String) -> ApiResult<T>): ApiResult<T> {
        val first = when (val bearer = bearer()) {
            is ApiResult.Ok -> bearer.value
            is ApiResult.Unavailable -> return bearer
            is ApiResult.Refused -> return bearer
        }
        val result = call(first)
        if (result !is ApiResult.Refused || result.status != 401) return result
        mutex.withLock { if (store.load() == first) store.clear() }
        return when (val again = bearer()) {
            is ApiResult.Ok -> call(again.value)
            is ApiResult.Unavailable -> again
            is ApiResult.Refused -> again
        }
    }

    private suspend fun bearer(): ApiResult<String> = mutex.withLock {
        store.load()?.let { return ApiResult.Ok(it) }
        val body = Json.encodeToString(NonceBody(nonce()))
        val token = integrity.token(registrationHash(body)) ?: return ApiResult.Unavailable(Why.NOT_REGISTERED)
        when (val registered = api.register(body, token)) {
            is ApiResult.Ok -> registered.also { store.save(it.value) }
            is ApiResult.Refused -> ApiResult.Unavailable(Why.NOT_REGISTERED)
            is ApiResult.Unavailable -> registered
        }
    }
}

/** 24 random bytes as 32 base64url characters (the server accepts 16–128 of `[A-Za-z0-9_-]`). */
fun randomNonce(): String =
    Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(24).also(SecureRandom()::nextBytes))
```

`server/Routes.kt`:

```kotlin
package io.github.locadani.whichride.server

import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.DataException
import io.github.locadani.whichride.core.Destination
import io.github.locadani.whichride.core.Trip
import io.github.locadani.whichride.core.destinationFromJson

/** What the compare flow needs from the server; the ViewModel's tests fake it. */
interface Routes {
    suspend fun route(from: Coordinate, to: Coordinate): ApiResult<Trip>
    suspend fun geocode(text: String, near: Coordinate?): ApiResult<List<Place>>
    suspend fun autocomplete(text: String, near: Coordinate?): ApiResult<List<Place>>
    suspend fun expand(shortLink: String): ApiResult<Destination>
}

class ServerRoutes(private val api: ServerApi, private val installs: Installs) : Routes {
    override suspend fun route(from: Coordinate, to: Coordinate) = installs.authorized { api.route(it, from, to) }
    override suspend fun geocode(text: String, near: Coordinate?) = installs.authorized { api.geocode(it, text, near) }
    override suspend fun autocomplete(text: String, near: Coordinate?) = installs.authorized { api.autocomplete(it, text, near) }
    override suspend fun expand(shortLink: String): ApiResult<Destination> =
        when (val json = installs.authorized { api.expand(it, shortLink) }) {
            is ApiResult.Ok -> try {
                ApiResult.Ok(destinationFromJson(json.value))
            } catch (_: DataException) {
                ApiResult.Refused(200, "invalid_destination")
            }
            is ApiResult.Unavailable -> json
            is ApiResult.Refused -> json
        }
}
```

- [ ] **Step 4: Run** `./gradlew :app:testDebugUnitTest` — pass.

- [ ] **Step 5: Commit** — `feat(android): server client and install registration`

---

### Task 7: Prices — server, cached copy or built-in seed

**Files:**
- Create: `…/prices/PriceBook.kt`
- Test: `…/prices/PriceBookTest.kt`

**Interfaces:**
- Consumes: `ServerApi.catalogue()`, generated `Prices`.
- Produces: `enum class PriceSource { SERVER, CACHED, BUILT_IN }`, `class LoadedPrices(val prices: Prices, val source: PriceSource)`,
  `interface PriceLoader { suspend fun offline(): LoadedPrices; suspend fun load(): LoadedPrices }`,
  `class PriceBook(api: ServerApi, cacheFile: File, seed: () -> String) : PriceLoader`. `offline()` never
  touches the network (cached copy, else seed), so the result screen has prices at once; `load()` asks
  the server first and the ViewModel swaps in its answer.

- [ ] **Step 1: Failing tests**

```kotlin
package io.github.locadani.whichride.prices

import io.github.locadani.whichride.server.ServerApi
import java.io.File
import kotlin.io.path.createTempDirectory
import kotlinx.coroutines.test.runTest
import mockwebserver3.MockResponse
import mockwebserver3.MockWebServer
import okhttp3.OkHttpClient
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class PriceBookTest {
    private val seed = File("../../crates/server/catalogue.json").readText()
    private val server = MockWebServer()
    private val cache = createTempDirectory().resolve("catalogue.json").toFile()
    private lateinit var api: ServerApi

    private val response =
        """{"catalogue":$seed,"offers":[{"offer":"miles.s","confirmed_on":20744,"locked":false,"pending":null}],"proposals":[]}"""

    @Before fun start() {
        server.start()
        api = ServerApi(server.url("/").toString().trimEnd('/'), OkHttpClient())
    }

    @After fun stop() = server.close()

    private fun book(api: ServerApi = this.api) = PriceBook(api, cache) { seed }

    @Test fun usesTheServersPricesAndKeepsACopy() = runTest {
        server.enqueue(MockResponse.Builder().body(response).build())
        val loaded = book().load()
        assertEquals(PriceSource.SERVER, loaded.source)
        assertEquals(response, cache.readText())
    }

    @Test fun fallsBackToTheCopyWhenTheServerIsBusy() = runTest {
        cache.writeText(response)
        server.enqueue(MockResponse.Builder().code(503).body("""{"error":"internal"}""").build())
        assertEquals(PriceSource.CACHED, book().load().source)
    }

    @Test fun fallsBackToTheSeedWithoutServerOrCopy() = runTest {
        val loaded = book(ServerApi("", OkHttpClient())).load()
        assertEquals(PriceSource.BUILT_IN, loaded.source)
        assertEquals(3, loaded.prices.providers().size)
        assertFalse(cache.exists())
    }

    @Test fun offlineNeverAsksTheServer() = runTest {
        cache.writeText(response)
        assertEquals(PriceSource.CACHED, book().offline().source)
        assertEquals(0, server.requestCount)
    }

    @Test fun aBrokenServerAnswerOrCopyIsIgnored() = runTest {
        cache.writeText("{ broken")
        server.enqueue(MockResponse.Builder().body("""{"catalogue":{"providers":[]}}""").build())
        assertEquals(PriceSource.BUILT_IN, book().load().source)
        assertTrue("the broken copy is not overwritten with a broken answer", cache.readText() == "{ broken")
    }
}
```

- [ ] **Step 2: Run, verify failure.**

- [ ] **Step 3: Implement**

```kotlin
package io.github.locadani.whichride.prices

import io.github.locadani.whichride.core.DataException
import io.github.locadani.whichride.core.Prices
import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.ServerApi
import java.io.File
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

enum class PriceSource { SERVER, CACHED, BUILT_IN }

class LoadedPrices(val prices: Prices, val source: PriceSource)

/**
 * The prices to compare with (decision 60): the server's catalogue, else the last one it sent,
 * else the seed bundled in the APK. The catalogue is public data, so caching it is no privacy
 * concern; it's validated by the core before use.
 */
interface PriceLoader {
    /** The cached copy, else the seed; no network. */
    suspend fun offline(): LoadedPrices
    /** The server's catalogue, else [offline]. */
    suspend fun load(): LoadedPrices
}

class PriceBook(private val api: ServerApi, private val cacheFile: File, private val seed: () -> String) : PriceLoader {
    override suspend fun load(): LoadedPrices {
        val answer = api.catalogue()
        if (answer is ApiResult.Ok) {
            parse(answer.value)?.let { prices ->
                withContext(Dispatchers.IO) { writeAtomically(answer.value) }
                return LoadedPrices(prices, PriceSource.SERVER)
            }
        }
        return offline()
    }

    override suspend fun offline(): LoadedPrices {
        val cached = withContext(Dispatchers.IO) { cacheFile.takeIf(File::exists)?.readText() }
        cached?.let(::parse)?.let { return LoadedPrices(it, PriceSource.CACHED) }
        // The seed is checked by the build's tests; a broken one is a bug worth crashing on.
        return LoadedPrices(Prices.fromSeed(withContext(Dispatchers.IO) { seed() }), PriceSource.BUILT_IN)
    }

    private fun parse(json: String): Prices? = try {
        Prices.fromServerResponse(json)
    } catch (_: DataException) {
        null
    }

    private fun writeAtomically(json: String) {
        val temporary = File(cacheFile.parentFile, "${cacheFile.name}.tmp")
        temporary.writeText(json)
        temporary.renameTo(cacheFile)
    }
}
```

- [ ] **Step 4: Run** — pass. **Step 5: Commit** — `feat(android): load prices from the server, a cached copy or the seed`

---

### Task 8: Location and launching other apps

**Files:**
- Create: `…/location/LocationSource.kt`, `…/launch/Launcher.kt`
- Test: `…/launch/LauncherTest.kt` (Robolectric)

**Interfaces:**
- Produces: `fun interface LocationSource { suspend fun current(): Coordinate? }`, `AndroidLocationSource(context)`,
  `fun hasLocationPermission(context): Boolean`; `interface Launcher { fun openProvider(androidPackage: String); fun openUrl(url: String) }`,
  `AndroidLauncher(context)`.

- [ ] **Step 1: Failing test**

```kotlin
package io.github.locadani.whichride.launch

import android.app.Application
import android.content.ComponentName
import android.content.Intent
import android.content.IntentFilter
import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Shadows.shadowOf

@RunWith(AndroidJUnit4::class)
class LauncherTest {
    private val app = ApplicationProvider.getApplicationContext<Application>()
    private val launcher = AndroidLauncher(app)

    @Before fun strict() = shadowOf(app).checkActivities(true)

    private fun install(pkg: String, filter: IntentFilter) {
        val component = ComponentName(pkg, "$pkg.Main")
        shadowOf(app.packageManager).addActivityIfNotPresent(component)
        shadowOf(app.packageManager).addIntentFilterForActivity(component, filter)
    }

    @Test fun opensAnInstalledProviderApp() {
        install("ee.mtakso.client", IntentFilter(Intent.ACTION_MAIN).apply { addCategory(Intent.CATEGORY_LAUNCHER) })
        launcher.openProvider("ee.mtakso.client")
        val started = shadowOf(app).nextStartedActivity
        assertEquals(Intent.ACTION_MAIN, started.action)
        assertEquals("ee.mtakso.client", started.`package`)
    }

    @Test fun opensThePlayStoreWhenTheAppIsMissing() {
        install("com.android.vending", IntentFilter(Intent.ACTION_VIEW).apply { addDataScheme("market") })
        launcher.openProvider("com.trafi.whitelabel.bvg")
        assertEquals("market://details?id=com.trafi.whitelabel.bvg", shadowOf(app).nextStartedActivity.dataString)
    }

    @Test fun opensTheWebStoreWithoutAStoreApp() {
        install("org.browser", IntentFilter(Intent.ACTION_VIEW).apply { addDataScheme("https"); addCategory(Intent.CATEGORY_DEFAULT) })
        launcher.openProvider("com.driveby.app")
        assertEquals("https://play.google.com/store/apps/details?id=com.driveby.app", shadowOf(app).nextStartedActivity.dataString)
    }

    @Test fun opensMapsUrlsInGoogleMapsFirst() {
        install("com.google.android.apps.maps", IntentFilter(Intent.ACTION_VIEW).apply { addDataScheme("https"); addCategory(Intent.CATEGORY_DEFAULT) })
        launcher.openUrl("https://www.google.com/maps/dir/?api=1&destination=52.5,13.4&travelmode=driving")
        assertEquals("com.google.android.apps.maps", shadowOf(app).nextStartedActivity.`package`)
    }
}
```

(Robolectric's intent-filter matching is less precise than Android's; if a filter needs extra
categories or the `checkActivities` resolution doesn't consider `setPackage`, adjust the test
setup, not the production code, and say so in the commit message.)

- [ ] **Step 2: Run, verify failure.**

- [ ] **Step 3: Implement**

`launch/Launcher.kt`:

```kotlin
package io.github.locadani.whichride.launch

import android.content.ActivityNotFoundException
import android.content.Context
import android.content.Intent
import android.net.Uri

/** Opens other apps; the screens never build intents themselves. */
interface Launcher {
    /** The provider's app, or its Play Store page when it isn't installed (decision 62). */
    fun openProvider(androidPackage: String)
    /** A Google Maps URL: in the Maps app when installed, else any handler. */
    fun openUrl(url: String)
}

class AndroidLauncher(private val context: Context) : Launcher {
    override fun openProvider(androidPackage: String) {
        val app = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER).setPackage(androidPackage)
        start(app) ||
            start(Intent(Intent.ACTION_VIEW, Uri.parse("market://details?id=$androidPackage"))) ||
            start(Intent(Intent.ACTION_VIEW, Uri.parse("https://play.google.com/store/apps/details?id=$androidPackage")))
    }

    override fun openUrl(url: String) {
        val uri = Uri.parse(url)
        start(Intent(Intent.ACTION_VIEW, uri).setPackage(GOOGLE_MAPS)) || start(Intent(Intent.ACTION_VIEW, uri))
    }

    private fun start(intent: Intent): Boolean = try {
        context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        true
    } catch (_: ActivityNotFoundException) {
        false
    }

    private companion object {
        const val GOOGLE_MAPS = "com.google.android.apps.maps"
    }
}
```

`location/LocationSource.kt`:

```kotlin
package io.github.locadani.whichride.location

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationManager
import android.os.Build
import androidx.core.content.ContextCompat
import androidx.core.location.LocationManagerCompat
import androidx.core.os.CancellationSignal
import io.github.locadani.whichride.core.Coordinate
import kotlin.coroutines.resume
import kotlin.math.roundToInt
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull

/** Where the phone is now; null without permission or when no fix comes within 15 s (R19). */
fun interface LocationSource {
    suspend fun current(): Coordinate?
}

fun hasLocationPermission(context: Context): Boolean = listOf(
    Manifest.permission.ACCESS_FINE_LOCATION,
    Manifest.permission.ACCESS_COARSE_LOCATION,
).any { ContextCompat.checkSelfPermission(context, it) == PackageManager.PERMISSION_GRANTED }

/** One fresh fix, kept in memory only (§9). */
class AndroidLocationSource(private val context: Context) : LocationSource {
    override suspend fun current(): Coordinate? {
        if (!hasLocationPermission(context)) return null
        val manager = context.getSystemService(LocationManager::class.java) ?: return null
        val provider = listOf(
            LocationManager.FUSED_PROVIDER.takeIf { Build.VERSION.SDK_INT >= Build.VERSION_CODES.S },
            LocationManager.NETWORK_PROVIDER,
            LocationManager.GPS_PROVIDER,
        ).filterNotNull().firstOrNull { manager.isProviderEnabled(it) } ?: return null
        return withTimeoutOrNull(TIMEOUT_MILLIS) {
            suspendCancellableCoroutine { continuation ->
                val signal = CancellationSignal()
                continuation.invokeOnCancellation { signal.cancel() }
                try {
                    LocationManagerCompat.getCurrentLocation(
                        manager, provider, signal, ContextCompat.getMainExecutor(context),
                    ) { location: Location? -> continuation.resume(location?.toCoordinate()) }
                } catch (_: SecurityException) { // permission revoked between the check and the call
                    continuation.resume(null)
                }
            }
        }
    }

    private companion object {
        const val TIMEOUT_MILLIS = 15_000L
    }
}

private fun Location.toCoordinate() = Coordinate((latitude * 1e7).roundToInt(), (longitude * 1e7).roundToInt())
```

(`FUSED_PROVIDER` is API 31; `isProviderEnabled` returns false for an absent provider.)

- [ ] **Step 4: Run** — pass. **Step 5: Commit** — `feat(android): location and opening provider apps and Google Maps`

---
### Task 9: The compare flow — `CompareViewModel`

**Files:**
- Create: `…/compare/Screen.kt`, `…/compare/UiState.kt`, `…/compare/CompareViewModel.kt`, `…/AppContainer.kt` (replace the stub)
- Modify: `…/MainActivity.kt` (wires the ViewModel; content still a placeholder until Task 10)
- Test: `…/compare/CompareViewModelTest.kt`, `…/compare/Fakes.kt`

**Interfaces:**
- Consumes: `Routes`, `Place`, `ApiResult`, `Why` (Task 6); `LocationSource`, `Launcher`, `mapsUrl`,
  `MapsTarget` (Tasks 5, 8); `ChoiceStore`, `toggle`, `forProvider`, `replacingProvider` (Task 5);
  `PriceLoader`, `LoadedPrices`, `PriceSource` (Task 7); `parseKilometres`, `parseMinutes`,
  `kilometresInput`, `formatMinutes` (Task 5); generated `parseDestination`, `sharedLabel`,
  `Destination`, `LinkException`, `Trip`, `Choice`, `Quote`, `Options`, `ProviderInfo`.
- Produces (used by Task 10's screens):
  - `sealed interface Screen { Home; Resolving; ManualTrip; Result; Providers; Cars(providerId); Start; Licences }`
  - `enum class ManualReason { BUSY, OFFLINE, NO_ROUTE, NO_LOCATION, EDIT }`, `enum class InputError { NOT_A_MAPS_LINK, NO_DESTINATION, NOT_FOUND, TOO_LONG }`,
    `enum class ResolvingStep { FINDING_PLACE, FINDING_LOCATION, ROUTING }`
  - `data class Target(val label: String?, val at: Coordinate?, val maps: MapsTarget)`, `sealed interface Start { MyLocation; Chosen(place) }`
  - `data class UiState(...)` with `screen`, `query`, `linkInQuery`, `suggestions`, `suggestionsUnavailable`,
    `inputError`, `start`, `startQuery`, `startSuggestions`, `target`, `step`, `manualReason`, `manualKm`,
    `manualMinutes`, `manualKmInvalid`, `manualMinutesInvalid`, `trip`, `priceSource`, `providers`, `quotes`,
    `options`, `choices`, `selected`, `draft`, `carFilter`, `askLocation`
  - `CompareViewModel` methods: `onShared(text)`, `onQuery(text)`, `submitQuery()`, `pickSuggestion(place)`,
    `changeStart()`, `onStartQuery(text)`, `pickStart(place)`, `useMyLocation()`, `onLocationAnswered()`,
    `onManualKm(text)`, `onManualMinutes(text)`, `submitManual()`, `editTrip()`, `addCars()`,
    `pickProvider(id)`, `toggle(choice)`, `onCarFilter(text)`, `commitCars()`, `remove(choice)`,
    `select(choice)`, `openChosen()`, `checkIn(providerId)`, `openMaps()`, `openLicences()`, `back(): Boolean`, `onResume()`
  - `AppContainer.viewModelFactory: ViewModelProvider.Factory`

- [ ] **Step 1: Fakes** (`src/test/…/compare/Fakes.kt`)

```kotlin
package io.github.locadani.whichride.compare

import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.Destination
import io.github.locadani.whichride.core.Prices
import io.github.locadani.whichride.core.Trip
import io.github.locadani.whichride.launch.Launcher
import io.github.locadani.whichride.options.ChoiceStore
import io.github.locadani.whichride.prices.LoadedPrices
import io.github.locadani.whichride.prices.PriceLoader
import io.github.locadani.whichride.prices.PriceSource
import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.Place
import io.github.locadani.whichride.server.Routes
import io.github.locadani.whichride.server.Why
import java.io.File
import kotlinx.coroutines.CompletableDeferred

val SEED: String = File("../../crates/server/catalogue.json").readText()
val ALEX = Coordinate(525_219_000, 134_132_000)
val NEUN = Coordinate(525_022_870, 134_316_050)
val TRIP = Trip(7_400u, 1_320u)

class FakeRoutes : Routes {
    var route: ApiResult<Trip> = ApiResult.Ok(TRIP)
    var geocode: ApiResult<List<Place>> = ApiResult.Ok(listOf(Place("Markthalle Neun, Eisenbahnstraße 42, Berlin", NEUN)))
    var autocomplete: ApiResult<List<Place>> = ApiResult.Ok(listOf(Place("Mauerpark, Berlin", NEUN)))
    var expand: ApiResult<Destination> = ApiResult.Unavailable(Why.OFFLINE)
    /** When set, `route` waits for it, so a test can replace a trip that is still routing. */
    var routeGate: CompletableDeferred<Unit>? = null
    val routed = mutableListOf<Pair<Coordinate, Coordinate>>()
    val autocompleted = mutableListOf<String>()

    override suspend fun route(from: Coordinate, to: Coordinate): ApiResult<Trip> {
        routeGate?.await()
        routed += from to to
        return route
    }
    override suspend fun geocode(text: String, near: Coordinate?) = geocode
    override suspend fun autocomplete(text: String, near: Coordinate?) = autocomplete.also { autocompleted += text }
    override suspend fun expand(shortLink: String) = expand
}

class FakeLauncher : Launcher {
    val providers = mutableListOf<String>()
    val urls = mutableListOf<String>()
    override fun openProvider(androidPackage: String) { providers += androidPackage }
    override fun openUrl(url: String) { urls += url }
}

class FakeStore(var choices: Set<Choice> = emptySet()) : ChoiceStore {
    override fun load() = choices
    override fun save(choices: Set<Choice>) { this.choices = choices }
}

class SeedPrices : PriceLoader {
    override suspend fun offline() = LoadedPrices(Prices.fromSeed(SEED), PriceSource.BUILT_IN)
    override suspend fun load() = offline()
}
```

- [ ] **Step 2: Failing tests** (`CompareViewModelTest.kt`; plain JVM with the host Rust library)

```kotlin
package io.github.locadani.whichride.compare

import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.Destination
import io.github.locadani.whichride.launch.MapsTarget
import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.Place
import io.github.locadani.whichride.server.Why
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class CompareViewModelTest {
    private val dispatcher = StandardTestDispatcher()
    private val routes = FakeRoutes()
    private val launcher = FakeLauncher()
    private val store = FakeStore()
    private var here: io.github.locadani.whichride.core.Coordinate? = ALEX
    private var permission = true

    @Before fun main() = Dispatchers.setMain(dispatcher)
    @After fun reset() = Dispatchers.resetMain()

    private fun TestScope.viewModel() = CompareViewModel(
        routes = routes,
        location = { here },
        launcher = launcher,
        store = store,
        priceLoader = SeedPrices(),
        hasLocationPermission = { permission },
        now = { 1_792_497_600_000L },
    ).also { advanceUntilIdle() }

    private val coordinateLink = "https://www.google.com/maps/search/?api=1&query=52.502287,13.431605"

    @Test fun `a shared coordinate link is routed from here and priced`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        val state = vm.state.value
        assertEquals(Screen.Result, state.screen)
        assertEquals(listOf(Screen.Home, Screen.Result), state.stack)
        assertEquals(TRIP, state.trip)
        assertEquals(listOf(ALEX to NEUN), routes.routed)
        assertEquals(9, state.quotes.size)
        assertTrue(state.options!!.ranked.isEmpty())
    }

    @Test fun `short link without server goes to manual entry with the shared name`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared("Markthalle Neun\nhttps://maps.app.goo.gl/x7Qa2Lm")
        advanceUntilIdle()
        val state = vm.state.value
        assertEquals(Screen.ManualTrip, state.screen)
        assertEquals(ManualReason.OFFLINE, state.manualReason)
        assertEquals("Markthalle Neun", state.target!!.label)
        vm.openMaps()
        assertEquals(listOf("https://maps.app.goo.gl/x7Qa2Lm"), launcher.urls)
    }

    @Test fun `an expanded short link is routed`() = runTest(dispatcher) {
        routes.expand = ApiResult.Ok(Destination.Coordinates(NEUN, "Markthalle Neun"))
        val vm = viewModel()
        vm.onShared("https://maps.app.goo.gl/x7Qa2Lm")
        advanceUntilIdle()
        assertEquals(Screen.Result, vm.state.value.screen)
        assertEquals("Markthalle Neun", vm.state.value.target!!.label)
    }

    @Test fun `a busy route service asks for km and minutes`() = runTest(dispatcher) {
        routes.route = ApiResult.Unavailable(Why.BUSY)
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertEquals(ManualReason.BUSY, vm.state.value.manualReason)
        vm.onManualKm("7,4")
        vm.onManualMinutes("22")
        vm.submitManual()
        assertEquals(Screen.Result, vm.state.value.screen)
        assertEquals(TRIP, vm.state.value.trip)
    }

    @Test fun `invalid km and minutes are flagged and nothing is compared`() = runTest(dispatcher) {
        routes.route = ApiResult.Unavailable(Why.BUSY)
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.onManualKm("7.4.1")
        vm.onManualMinutes("0")
        vm.submitManual()
        val state = vm.state.value
        assertEquals(Screen.ManualTrip, state.screen)
        assertTrue(state.manualKmInvalid && state.manualMinutesInvalid)
    }

    @Test fun `no route by car asks for km and minutes`() = runTest(dispatcher) {
        routes.route = ApiResult.Refused(422, "no_route")
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertEquals(ManualReason.NO_ROUTE, vm.state.value.manualReason)
    }

    @Test fun `no location goes to manual entry`() = runTest(dispatcher) {
        here = null
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertEquals(ManualReason.NO_LOCATION, vm.state.value.manualReason)
        assertTrue(routes.routed.isEmpty())
    }

    @Test fun `location permission is asked once and the trip waits for the answer`() = runTest(dispatcher) {
        permission = false
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertTrue(vm.state.value.askLocation)
        assertEquals(Screen.Resolving, vm.state.value.screen)
        permission = true
        vm.onLocationAnswered()
        advanceUntilIdle()
        assertFalse(vm.state.value.askLocation)
        assertEquals(Screen.Result, vm.state.value.screen)
        permission = false
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertFalse("asked only once per session", vm.state.value.askLocation)
    }

    @Test fun `a chosen start is used instead of the location`() = runTest(dispatcher) {
        here = null
        val vm = viewModel()
        vm.changeStart()
        assertEquals(Screen.Start, vm.state.value.screen)
        vm.pickStart(Place("Alexanderplatz, Berlin", ALEX))
        assertEquals(Screen.Home, vm.state.value.screen)
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertEquals(listOf(ALEX to NEUN), routes.routed)
        vm.openMaps()
        assertEquals(
            "https://www.google.com/maps/dir/?api=1&origin=52.5219,13.4132&destination=52.502287,13.431605&travelmode=driving",
            launcher.urls.single(),
        )
    }

    @Test fun `choosing a start after a failed location routes the waiting trip`() = runTest(dispatcher) {
        here = null
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.changeStart()
        vm.pickStart(Place("Alexanderplatz, Berlin", ALEX))
        advanceUntilIdle()
        assertEquals(Screen.Result, vm.state.value.screen)
    }

    @Test fun `a place name is geocoded`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared("https://www.google.com/maps/place/Markthalle+Neun")
        advanceUntilIdle()
        assertEquals(Screen.Result, vm.state.value.screen)
        assertEquals("Markthalle Neun, Eisenbahnstraße 42, Berlin", vm.state.value.target!!.label)
    }

    @Test fun `a place nobody finds goes back home with the reason`() = runTest(dispatcher) {
        routes.geocode = ApiResult.Ok(emptyList())
        val vm = viewModel()
        vm.onQuery("Nowhere street 99")
        vm.submitQuery()
        advanceUntilIdle()
        assertEquals(Screen.Home, vm.state.value.screen)
        assertEquals(InputError.NOT_FOUND, vm.state.value.inputError)
        assertEquals("Nowhere street 99", vm.state.value.query)
    }

    @Test fun `a typed place without the server can still be compared by hand`() = runTest(dispatcher) {
        routes.geocode = ApiResult.Unavailable(Why.NOT_CONFIGURED)
        val vm = viewModel()
        vm.onQuery("Markthalle Neun")
        vm.submitQuery()
        advanceUntilIdle()
        val state = vm.state.value
        assertEquals(ManualReason.OFFLINE, state.manualReason)
        assertEquals(MapsTarget.Query("Markthalle Neun"), state.target!!.maps)
    }

    @Test fun `a link that is not from Google Maps is refused on the home screen`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared("look https://example.com/place")
        assertEquals(Screen.Home, vm.state.value.screen)
        assertEquals(InputError.NOT_A_MAPS_LINK, vm.state.value.inputError)
    }

    @Test fun `shared text without a link is searched like a typed address`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared("Eisenbahnstraße 42, Berlin")
        advanceUntilIdle()
        assertEquals(Screen.Result, vm.state.value.screen)
    }

    @Test fun `a pasted link is offered instead of suggestions`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onQuery("https://maps.app.goo.gl/x7Qa2Lm")
        advanceUntilIdle()
        assertTrue(vm.state.value.linkInQuery)
        assertTrue(routes.autocompleted.isEmpty())
    }

    @Test fun `suggestions wait for a pause and three characters`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onQuery("Ma")
        advanceTimeBy(400)
        vm.onQuery("Mau")
        advanceTimeBy(100)
        vm.onQuery("Maue")
        advanceTimeBy(100)
        vm.onQuery("Mauer")
        advanceUntilIdle()
        assertEquals(listOf("Mauer"), routes.autocompleted)
        assertEquals(listOf(Place("Mauerpark, Berlin", NEUN)), vm.state.value.suggestions)
    }

    @Test fun `suggestions say when the server is unavailable`() = runTest(dispatcher) {
        routes.autocomplete = ApiResult.Unavailable(Why.NOT_CONFIGURED)
        val vm = viewModel()
        vm.onQuery("Mauer")
        advanceUntilIdle()
        assertTrue(vm.state.value.suggestionsUnavailable)
    }

    @Test fun `adding cars ranks them and Open launches the cheapest providers app`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.addCars()
        vm.pickProvider("sixt")
        assertEquals(Screen.Cars("sixt"), vm.state.value.screen)
        vm.toggle(Choice.Offer("sixt.volkswagen-t-roc"))
        vm.commitCars()
        vm.addCars()
        vm.pickProvider("miles")
        vm.toggle(Choice.AnyOf("miles"))
        vm.commitCars()
        val state = vm.state.value
        assertEquals(listOf(Screen.Home, Screen.Result), state.stack)
        assertEquals(2, state.options!!.ranked.size)
        assertEquals(setOf(Choice.Offer("sixt.volkswagen-t-roc"), Choice.AnyOf("miles")), store.choices)
        vm.openChosen()
        val cheapest = state.options!!.ranked.first().quote.providerId
        val pkg = state.providers.single { it.id == cheapest }.androidPackage
        assertEquals(listOf(pkg), launcher.providers)
    }

    @Test fun `the selected option is the one opened`() = runTest(dispatcher) {
        store.choices = setOf(Choice.AnyOf("miles"), Choice.Offer("sixt.volkswagen-t-roc"))
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.select(Choice.Offer("sixt.volkswagen-t-roc"))
        vm.openChosen()
        assertEquals(listOf("com.trafi.whitelabel.bvg"), launcher.providers)
    }

    @Test fun `re-entering a provider shows its cars ticked and removing works`() = runTest(dispatcher) {
        store.choices = setOf(Choice.Offer("bolt.vw-polo"), Choice.AnyOf("miles"))
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.addCars()
        vm.pickProvider("bolt")
        assertEquals(setOf(Choice.Offer("bolt.vw-polo")), vm.state.value.draft)
        assertTrue(vm.back())
        assertTrue(vm.back())
        vm.remove(Choice.AnyOf("miles"))
        assertEquals(setOf(Choice.Offer("bolt.vw-polo")), store.choices)
        assertEquals(1, vm.state.value.options!!.ranked.size)
    }

    @Test fun `check in opens the provider without leaving step 2`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.addCars()
        vm.pickProvider("sixt")
        vm.toggle(Choice.Offer("sixt.volkswagen-t-roc"))
        vm.checkIn("sixt")
        assertEquals(listOf("com.trafi.whitelabel.bvg"), launcher.providers)
        assertEquals(Screen.Cars("sixt"), vm.state.value.screen)
        assertEquals(setOf(Choice.Offer("sixt.volkswagen-t-roc")), vm.state.value.draft)
    }

    @Test fun `a new share replaces the trip and keeps the cars`() = runTest(dispatcher) {
        store.choices = setOf(Choice.AnyOf("bolt"))
        val gate = CompletableDeferred<Unit>()
        routes.routeGate = gate
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertEquals(Screen.Resolving, vm.state.value.screen)
        routes.routeGate = null
        vm.onShared("https://www.google.com/maps/search/?api=1&query=52.5219,13.4132")
        gate.complete(Unit)
        advanceUntilIdle()
        val state = vm.state.value
        assertEquals(listOf(NEUN to ALEX).map { ALEX to it.second }, routes.routed.takeLast(1))
        assertEquals(1, routes.routed.size)
        assertEquals(setOf(Choice.AnyOf("bolt")), state.choices)
        assertEquals(1, state.options!!.ranked.size)
    }

    @Test fun `editing km and minutes keeps the result`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        vm.editTrip()
        assertEquals("7.4", vm.state.value.manualKm)
        assertEquals("22", vm.state.value.manualMinutes)
        vm.onManualKm("10")
        vm.submitManual()
        assertEquals(listOf(Screen.Home, Screen.Result), vm.state.value.stack)
        assertEquals(10_000u, vm.state.value.trip!!.metres)
    }

    @Test fun `back walks the stack and cancels routing`() = runTest(dispatcher) {
        routes.routeGate = CompletableDeferred()
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertTrue(vm.back())
        advanceUntilIdle()
        assertEquals(Screen.Home, vm.state.value.screen)
        assertNull(vm.state.value.step)
        assertFalse(vm.back())
    }

    @Test fun `cars expire while the app is in the background`() = runTest(dispatcher) {
        store.choices = setOf(Choice.AnyOf("bolt"))
        val vm = viewModel()
        store.choices = emptySet() // what the real store returns after 15 minutes
        vm.onResume()
        assertTrue(vm.state.value.choices.isEmpty())
    }
}
```

(The replace-the-trip test asserts that only the second destination was routed: the first
coroutine was cancelled while waiting. Simplify its first assertion to
`assertEquals(listOf(ALEX to ALEX), routes.routed)` when writing it — the second link points at
`ALEX`.)

- [ ] **Step 3: Run, verify failure.**

- [ ] **Step 4: Implement** — `compare/Screen.kt` and `compare/UiState.kt`

```kotlin
package io.github.locadani.whichride.compare

/** One entry of the app's back stack (decision 63). */
sealed interface Screen {
    data object Home : Screen
    data object Resolving : Screen
    data object ManualTrip : Screen
    data object Result : Screen
    data object Providers : Screen
    data class Cars(val providerId: String) : Screen
    data object Start : Screen
    data object Licences : Screen
}
```

```kotlin
package io.github.locadani.whichride.compare

import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.Options
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.core.Quote
import io.github.locadani.whichride.core.Trip
import io.github.locadani.whichride.launch.MapsTarget
import io.github.locadani.whichride.prices.PriceSource
import io.github.locadani.whichride.server.Place

/** Why the user is asked for km and minutes (R25). */
enum class ManualReason { BUSY, OFFLINE, NO_ROUTE, NO_LOCATION, EDIT }

enum class InputError { NOT_A_MAPS_LINK, NO_DESTINATION, NOT_FOUND, TOO_LONG }

enum class ResolvingStep { FINDING_PLACE, FINDING_LOCATION, ROUTING }

/** Where the trip goes: its name, its position when known, and what Google Maps should open. */
data class Target(val label: String?, val at: Coordinate?, val maps: MapsTarget)

sealed interface Start {
    data object MyLocation : Start
    data class Chosen(val place: Place) : Start
}

data class UiState(
    val stack: List<Screen> = listOf(Screen.Home),
    // Home
    val query: String = "",
    val linkInQuery: Boolean = false,
    val suggestions: List<Place> = emptyList(),
    val suggestionsUnavailable: Boolean = false,
    val inputError: InputError? = null,
    // Start
    val start: Start = Start.MyLocation,
    val startQuery: String = "",
    val startSuggestions: List<Place> = emptyList(),
    // Trip
    val target: Target? = null,
    val step: ResolvingStep? = null,
    val askLocation: Boolean = false,
    val manualReason: ManualReason? = null,
    val manualKm: String = "",
    val manualMinutes: String = "",
    val manualKmInvalid: Boolean = false,
    val manualMinutesInvalid: Boolean = false,
    val trip: Trip? = null,
    // Prices and options
    val priceSource: PriceSource? = null,
    val providers: List<ProviderInfo> = emptyList(),
    val quotes: List<Quote> = emptyList(),
    val options: Options? = null,
    val choices: Set<Choice> = emptySet(),
    val selected: Choice? = null,
    val draft: Set<Choice> = emptySet(),
    val carFilter: String = "",
) {
    val screen: Screen get() = stack.last()
}
```

`compare/CompareViewModel.kt`:

```kotlin
package io.github.locadani.whichride.compare

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.core.Destination
import io.github.locadani.whichride.core.LinkException
import io.github.locadani.whichride.core.Prices
import io.github.locadani.whichride.core.Trip
import io.github.locadani.whichride.core.parseDestination
import io.github.locadani.whichride.core.sharedLabel
import io.github.locadani.whichride.format.kilometresInput
import io.github.locadani.whichride.format.formatMinutes
import io.github.locadani.whichride.format.parseKilometres
import io.github.locadani.whichride.format.parseMinutes
import io.github.locadani.whichride.launch.Launcher
import io.github.locadani.whichride.launch.MapsTarget
import io.github.locadani.whichride.launch.mapsUrl
import io.github.locadani.whichride.location.LocationSource
import io.github.locadani.whichride.options.ChoiceStore
import io.github.locadani.whichride.options.forProvider
import io.github.locadani.whichride.options.replacingProvider
import io.github.locadani.whichride.options.toggle as toggled
import io.github.locadani.whichride.prices.PriceLoader
import io.github.locadani.whichride.prices.PriceSource
import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.Place
import io.github.locadani.whichride.server.Routes
import io.github.locadani.whichride.server.Why
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * The whole compare flow (spec §4): destination → start → km and minutes → prices → the cars
 * near the user → open an app. Holds the back stack (decision 63); locations stay in memory (§9).
 */
class CompareViewModel(
    private val routes: Routes,
    private val location: LocationSource,
    private val launcher: Launcher,
    private val store: ChoiceStore,
    private val priceLoader: PriceLoader,
    private val hasLocationPermission: () -> Boolean,
    private val now: () -> Long = System::currentTimeMillis,
) : ViewModel() {
    private val mutableState = MutableStateFlow(UiState(choices = store.load()))
    val state: StateFlow<UiState> = mutableState.asStateFlow()

    private var prices: Prices? = null
    private var tripJob: Job? = null
    private var searchJob: Job? = null
    private var startSearchJob: Job? = null
    private var permissionAnswer: CompletableDeferred<Unit>? = null
    private var permissionAsked = false
    /** The last location fix, for search bias and the Maps link; memory only. */
    private var here: Coordinate? = null

    init {
        viewModelScope.launch {
            usePrices(priceLoader.offline().let { it.prices to it.source })
            val fresh = priceLoader.load()
            if (fresh.source == PriceSource.SERVER) usePrices(fresh.prices to fresh.source)
        }
    }

    // --- Destination input -------------------------------------------------------------

    /** A Google Maps share, a `geo:` link, or a pasted link (spec §4 step 1). */
    fun onShared(text: String) {
        val destination = try {
            parseDestination(text)
        } catch (e: LinkException) {
            if (e is LinkException.NoLink) {
                onQuery(text.trim().take(MAX_QUERY_CHARS))
                return submitQuery()
            }
            tripJob?.cancel()
            mutableState.update { it.copy(stack = listOf(Screen.Home), step = null, inputError = e.toInputError()) }
            return
        }
        startTrip(destination, sharedLabel(text))
    }

    fun onQuery(text: String) {
        val isLink = try {
            parseDestination(text)
            true
        } catch (e: LinkException) {
            e !is LinkException.NoLink
        }
        mutableState.update { it.copy(query = text, linkInQuery = isLink, inputError = null) }
        searchJob?.cancel()
        if (isLink || text.trim().length < MIN_SEARCH_CHARS) {
            mutableState.update { it.copy(suggestions = emptyList(), suggestionsUnavailable = false) }
            return
        }
        searchJob = viewModelScope.launch {
            delay(SEARCH_DELAY_MILLIS)
            val result = routes.autocomplete(text.trim(), startCoordinateIfKnown())
            mutableState.update {
                it.copy(
                    suggestions = (result as? ApiResult.Ok)?.value.orEmpty(),
                    suggestionsUnavailable = result !is ApiResult.Ok,
                )
            }
        }
    }

    fun submitQuery() {
        val text = state.value.query.trim()
        if (text.isEmpty()) return
        if (state.value.linkInQuery) return onShared(text)
        startTrip(Destination.Place(text, null), null)
    }

    fun pickSuggestion(place: Place) = startTrip(Destination.Coordinates(place.at, place.label), null)

    // --- Start -------------------------------------------------------------------------

    fun changeStart() = push(Screen.Start)

    fun onStartQuery(text: String) {
        mutableState.update { it.copy(startQuery = text) }
        startSearchJob?.cancel()
        if (text.trim().length < MIN_SEARCH_CHARS) return mutableState.update { it.copy(startSuggestions = emptyList()) }
        startSearchJob = viewModelScope.launch {
            delay(SEARCH_DELAY_MILLIS)
            val result = routes.autocomplete(text.trim(), here)
            mutableState.update { it.copy(startSuggestions = (result as? ApiResult.Ok)?.value.orEmpty()) }
        }
    }

    fun pickStart(place: Place) = setStart(Start.Chosen(place))

    fun useMyLocation() = setStart(Start.MyLocation)

    private fun setStart(start: Start) {
        mutableState.update { it.copy(start = start, startQuery = "", startSuggestions = emptyList()) }
        pop()
        val waiting = state.value.target
        if (state.value.screen == Screen.ManualTrip && waiting?.at != null) {
            tripJob?.cancel()
            tripJob = viewModelScope.launch { route(waiting) }
        }
    }

    fun onLocationAnswered() {
        mutableState.update { it.copy(askLocation = false) }
        permissionAnswer?.complete(Unit)
    }

    // --- Trip --------------------------------------------------------------------------

    private fun startTrip(destination: Destination, label: String?) {
        tripJob?.cancel()
        searchJob?.cancel()
        mutableState.update {
            it.copy(
                stack = listOf(Screen.Home, Screen.Resolving),
                query = "", linkInQuery = false, suggestions = emptyList(), inputError = null,
                target = Target(label, null, MapsTarget.Query(label.orEmpty())),
                step = ResolvingStep.FINDING_PLACE, trip = null, manualReason = null, selected = null,
            )
        }
        tripJob = viewModelScope.launch {
            when (val resolution = resolve(destination, label)) {
                is Resolution.Found -> {
                    mutableState.update { it.copy(target = resolution.target) }
                    route(resolution.target)
                }
                is Resolution.Unrouted -> {
                    mutableState.update { it.copy(target = resolution.target) }
                    manual(resolution.reason)
                }
                is Resolution.Failed -> mutableState.update {
                    it.copy(stack = listOf(Screen.Home), step = null, query = resolution.query, inputError = resolution.error)
                }
            }
        }
    }

    private sealed interface Resolution {
        data class Found(val target: Target) : Resolution
        data class Unrouted(val target: Target, val reason: ManualReason) : Resolution
        data class Failed(val error: InputError, val query: String) : Resolution
    }

    private suspend fun resolve(destination: Destination, label: String?): Resolution = when (destination) {
        is Destination.Coordinates -> Resolution.Found(
            Target(destination.label ?: label, destination.at, MapsTarget.At(destination.at)),
        )
        is Destination.Place -> when (val found = routes.geocode(destination.query, destination.near ?: startCoordinateIfKnown())) {
            is ApiResult.Ok -> found.value.firstOrNull()
                ?.let { Resolution.Found(Target(it.label, it.at, MapsTarget.At(it.at))) }
                ?: Resolution.Failed(InputError.NOT_FOUND, destination.query)
            is ApiResult.Unavailable -> Resolution.Unrouted(
                Target(destination.query, null, MapsTarget.Query(destination.query)), found.why.reason(),
            )
            is ApiResult.Refused -> Resolution.Unrouted(
                Target(destination.query, null, MapsTarget.Query(destination.query)), ManualReason.OFFLINE,
            )
        }
        is Destination.ShortLink -> when (val expanded = routes.expand(destination.url)) {
            is ApiResult.Ok -> if (expanded.value is Destination.ShortLink) {
                Resolution.Unrouted(Target(label, null, MapsTarget.Link(destination.url)), ManualReason.OFFLINE)
            } else {
                resolve(expanded.value, label)
            }
            is ApiResult.Unavailable -> Resolution.Unrouted(Target(label, null, MapsTarget.Link(destination.url)), expanded.why.reason())
            is ApiResult.Refused -> if (expanded.code == "no_destination") {
                Resolution.Failed(InputError.NO_DESTINATION, "")
            } else {
                Resolution.Unrouted(Target(label, null, MapsTarget.Link(destination.url)), ManualReason.OFFLINE)
            }
        }
    }

    private suspend fun route(target: Target) {
        val to = target.at ?: return manual(ManualReason.OFFLINE)
        mutableState.update { it.copy(step = ResolvingStep.FINDING_LOCATION) }
        val from = startCoordinate() ?: return manual(ManualReason.NO_LOCATION)
        mutableState.update { it.copy(step = ResolvingStep.ROUTING) }
        when (val trip = routes.route(from, to)) {
            is ApiResult.Ok -> showResult(trip.value)
            is ApiResult.Unavailable -> manual(trip.why.reason())
            is ApiResult.Refused -> manual(if (trip.code == "no_route") ManualReason.NO_ROUTE else ManualReason.OFFLINE)
        }
    }

    private suspend fun startCoordinate(): Coordinate? = when (val start = state.value.start) {
        is Start.Chosen -> start.place.at
        Start.MyLocation -> {
            askForLocationOnce()
            location.current()?.also { here = it }
        }
    }

    private fun startCoordinateIfKnown(): Coordinate? = when (val start = state.value.start) {
        is Start.Chosen -> start.place.at
        Start.MyLocation -> here
    }

    /** Asks in context, the first time a trip needs the start (R19), and waits for the answer. */
    private suspend fun askForLocationOnce() {
        if (hasLocationPermission() || permissionAsked) return
        permissionAsked = true
        val answer = CompletableDeferred<Unit>().also { permissionAnswer = it }
        mutableState.update { it.copy(askLocation = true) }
        answer.await()
    }

    private fun manual(reason: ManualReason) {
        val trip = state.value.trip
        mutableState.update {
            it.copy(
                stack = it.stack.dropLastWhile { screen -> screen == Screen.Resolving } + Screen.ManualTrip,
                step = null, manualReason = reason,
                manualKm = trip?.let { t -> kilometresInput(t.metres) }.orEmpty(),
                manualMinutes = trip?.let { t -> formatMinutes(t.seconds) }.orEmpty(),
                manualKmInvalid = false, manualMinutesInvalid = false,
            )
        }
    }

    fun onManualKm(text: String) = mutableState.update { it.copy(manualKm = text, manualKmInvalid = false) }

    fun onManualMinutes(text: String) = mutableState.update { it.copy(manualMinutes = text, manualMinutesInvalid = false) }

    fun submitManual() {
        val metres = parseKilometres(state.value.manualKm)
        val seconds = parseMinutes(state.value.manualMinutes)
        if (metres == null || seconds == null) {
            return mutableState.update { it.copy(manualKmInvalid = metres == null, manualMinutesInvalid = seconds == null) }
        }
        showResult(Trip(metres, seconds))
    }

    /** Manual entry is always available (§8), prefilled with the current trip. */
    fun editTrip() = manual(ManualReason.EDIT)

    private fun showResult(trip: Trip) {
        mutableState.update { it.copy(trip = trip, step = null, stack = listOf(Screen.Home, Screen.Result)) }
        reprice()
    }

    // --- Cars near you (§4.1) ----------------------------------------------------------

    fun addCars() = push(Screen.Providers)

    fun pickProvider(providerId: String) {
        val draft = state.value.choices.forProvider(providerId, offerIds(providerId))
        mutableState.update { it.copy(draft = draft, carFilter = "") }
        push(Screen.Cars(providerId))
    }

    fun toggle(choice: Choice) = mutableState.update { it.copy(draft = toggled(it.draft, choice)) }

    fun onCarFilter(text: String) = mutableState.update { it.copy(carFilter = text) }

    fun commitCars() {
        val providerId = (state.value.screen as? Screen.Cars)?.providerId ?: return
        val choices = state.value.choices.replacingProvider(providerId, offerIds(providerId), state.value.draft)
        store.save(choices)
        mutableState.update { it.copy(choices = choices, draft = emptySet(), selected = null, stack = listOf(Screen.Home, Screen.Result)) }
        reprice()
    }

    fun remove(choice: Choice) {
        val choices = state.value.choices - choice
        store.save(choices)
        mutableState.update { it.copy(choices = choices, selected = it.selected.takeIf { s -> s != choice }) }
        reprice()
    }

    fun select(choice: Choice) = mutableState.update { it.copy(selected = choice) }

    /** Open <provider> for the selected option, else the cheapest. */
    fun openChosen() {
        val ranked = state.value.options?.ranked.orEmpty()
        val chosen = ranked.firstOrNull { it.choice == state.value.selected } ?: ranked.firstOrNull() ?: return
        packageOf(chosen.quote.providerId)?.let(launcher::openProvider)
    }

    /** "Check in <provider>" from step 2; the ticks stay (§4.1). */
    fun checkIn(providerId: String) {
        packageOf(providerId)?.let(launcher::openProvider)
    }

    fun openMaps() {
        val target = state.value.target ?: return
        launcher.openUrl(mapsUrl(startCoordinateIfKnown(), target.maps))
    }

    fun openLicences() = push(Screen.Licences)

    /** Cars expire after 15 minutes even while the app is in the background. */
    fun onResume() {
        val choices = store.load()
        if (choices != state.value.choices) {
            mutableState.update { it.copy(choices = choices) }
            reprice()
        }
    }

    // --- Navigation --------------------------------------------------------------------

    /** System back; false when there is nothing left to go back to (the activity finishes). */
    fun back(): Boolean {
        val stack = state.value.stack
        if (stack.size <= 1) return false
        if (stack.last() == Screen.Resolving) {
            tripJob?.cancel()
            permissionAnswer?.cancel()
        }
        mutableState.update { it.copy(stack = stack.dropLast(1), step = null, askLocation = false) }
        return true
    }

    private fun push(screen: Screen) = mutableState.update { it.copy(stack = it.stack + screen) }

    private fun pop() = mutableState.update { it.copy(stack = it.stack.dropLast(1).ifEmpty { listOf(Screen.Home) }) }

    // --- Prices ------------------------------------------------------------------------

    private fun usePrices(loaded: Pair<Prices, PriceSource>) {
        prices = loaded.first
        mutableState.update { it.copy(priceSource = loaded.second, providers = loaded.first.providers()) }
        reprice()
    }

    private fun reprice() {
        val prices = prices
        val trip = state.value.trip
        if (prices == null || trip == null) return mutableState.update { it.copy(quotes = emptyList(), options = null) }
        val today = now() / 1_000
        val options = prices.options(trip, state.value.choices.toList(), today)
        mutableState.update {
            it.copy(
                quotes = prices.quotes(trip, today),
                options = options,
                selected = it.selected?.takeIf { s -> options.ranked.any { o -> o.choice == s } },
            )
        }
    }

    private fun offerIds(providerId: String): Set<String> =
        state.value.providers.firstOrNull { it.id == providerId }?.offers?.mapTo(mutableSetOf()) { it.id }.orEmpty()

    private fun packageOf(providerId: String) = state.value.providers.firstOrNull { it.id == providerId }?.androidPackage

    private companion object {
        const val SEARCH_DELAY_MILLIS = 300L
        const val MIN_SEARCH_CHARS = 3
        const val MAX_QUERY_CHARS = 200
    }
}

private fun Why.reason() = if (this == Why.BUSY) ManualReason.BUSY else ManualReason.OFFLINE

private fun LinkException.toInputError() = when (this) {
    is LinkException.TooLong -> InputError.TOO_LONG
    is LinkException.NoDestination, is LinkException.InvalidCoordinates -> InputError.NO_DESTINATION
    else -> InputError.NOT_A_MAPS_LINK
}
```

(`manual()` for `EDIT` pushes `ManualTrip` on top of `Result`, so back returns to the result;
`submitManual` always rebuilds `[Home, Result]`.)

`AppContainer.kt`:

```kotlin
package io.github.locadani.whichride

import android.app.Application
import android.content.Context
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.viewmodel.initializer
import androidx.lifecycle.viewmodel.viewModelFactory
import io.github.locadani.whichride.compare.CompareViewModel
import io.github.locadani.whichride.launch.AndroidLauncher
import io.github.locadani.whichride.location.AndroidLocationSource
import io.github.locadani.whichride.location.hasLocationPermission
import io.github.locadani.whichride.options.OptionsStore
import io.github.locadani.whichride.prices.PriceBook
import io.github.locadani.whichride.server.Installs
import io.github.locadani.whichride.server.IntegrityTokens
import io.github.locadani.whichride.server.KeystoreTokenStore
import io.github.locadani.whichride.server.PlayIntegrityTokens
import io.github.locadani.whichride.server.ServerApi
import io.github.locadani.whichride.server.ServerRoutes
import java.io.File
import java.util.concurrent.TimeUnit
import okhttp3.OkHttpClient

/** Wires the app by hand (R33). */
class AppContainer(private val app: Application) {
    private val http = OkHttpClient.Builder()
        .connectTimeout(10, TimeUnit.SECONDS)
        .callTimeout(20, TimeUnit.SECONDS)
        .build()
    private val api = ServerApi(BuildConfig.SERVER_URL, http)
    private val integrity: IntegrityTokens =
        if (BuildConfig.CLOUD_PROJECT_NUMBER == 0L) IntegrityTokens { null }
        else PlayIntegrityTokens(app, BuildConfig.CLOUD_PROJECT_NUMBER)
    private val installs = Installs(api, integrity, KeystoreTokenStore(prefs("install")))
    private val priceBook = PriceBook(api, File(app.filesDir, "catalogue.json")) {
        app.assets.open("seed-catalogue.json").bufferedReader().use { it.readText() }
    }

    val viewModelFactory: ViewModelProvider.Factory = viewModelFactory {
        initializer {
            CompareViewModel(
                routes = ServerRoutes(api, installs),
                location = AndroidLocationSource(app),
                launcher = AndroidLauncher(app),
                store = OptionsStore(prefs("options"), System::currentTimeMillis),
                priceLoader = priceBook,
                hasLocationPermission = { hasLocationPermission(app) },
            )
        }
    }

    private fun prefs(name: String) = app.getSharedPreferences(name, Context.MODE_PRIVATE)
}
```

`MainActivity.kt`:

```kotlin
package io.github.locadani.whichride

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.viewModels
import io.github.locadani.whichride.compare.CompareViewModel
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.entry.sharedText
import io.github.locadani.whichride.ui.WhichrideNav

class MainActivity : ComponentActivity() {
    private val viewModel: CompareViewModel by viewModels {
        (application as WhichrideApplication).container.viewModelFactory
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        // A recreated activity (rotation) already handed its share to the ViewModel.
        if (savedInstanceState == null) sharedText(intent)?.let(viewModel::onShared)
        setContent { WhichrideTheme { WhichrideNav(viewModel, onExit = ::finish) } }
    }

    /** A second share while the app is open replaces the trip (R31). */
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        sharedText(intent)?.let(viewModel::onShared)
    }

    override fun onResume() {
        super.onResume()
        viewModel.onResume()
    }
}
```

(Until Task 10 exists, `WhichrideNav` is a stub composable showing the screen name; commit it in
this task so the app builds.)

- [ ] **Step 5: Run** `./gradlew :app:testDebugUnitTest` — all pass.

- [ ] **Step 6: Commit** — `feat(android): the compare flow from destination to the cars near you`

---
### Task 10: Screens

**Files:**
- Modify: `design/tokens.json` (3 size tokens), `design/contrast.json` (1 pair), regenerated `WhichrideTokens.kt`
  and `web/src/styles/tokens.css` (`npm run build`), `android/app/src/main/res/values/strings.xml`
- Create: `…/ui/WhichrideNav.kt` (replaces Task 9's stub), `…/ui/Components.kt`, `…/ui/Text.kt`,
  `…/ui/HomeScreen.kt`, `…/ui/ResolvingScreen.kt`, `…/ui/ManualTripScreen.kt`, `…/ui/ResultScreen.kt`,
  `…/ui/ProvidersScreen.kt`, `…/ui/CarsScreen.kt`, `…/ui/StartScreen.kt`, `…/ui/LicencesScreen.kt`
- Test: `…/ui/ResultScreenTest.kt`, `…/ui/CarsScreenTest.kt`, `…/ui/HomeScreenTest.kt` (Robolectric Compose)

**Interfaces:**
- Consumes: `UiState`, `Screen`, `ManualReason`, `InputError`, `ResolvingStep`, `Start` and every
  `CompareViewModel` method (Task 9); `formatMoney`, `formatKilometres`, `formatMinutes` (Task 5);
  `PriceSource`; generated `Quote`, `RankedOption`, `Choice`, `ProviderInfo`; `WhichrideTheme`,
  `WhichrideTokens` (+ new `Size.monogram`, `Size.stroke`, `Size.strokeStrong`).
- Produces: `@Composable fun WhichrideNav(viewModel: CompareViewModel)` and one stateless
  composable per screen taking `UiState` plus callbacks (signatures below), so tests drive them
  without a ViewModel.

**Design mapping (canvas → tokens):** screen padding `Space.md`; title padding `Space.xxs`;
cards `Surface(color = colors.surface, shape = shapes.large)` (radius 20); rows `shapes.small`
(14); buttons `shapes.medium` (16), primary height `Size.controlLg` (56), secondary
`Size.controlMd` (48); monogram circles `Size.monogram` (32) and `Size.controlMd` (48, provider
step); borders `Size.stroke` (1, outline) and `Size.strokeStrong` (2, selected option / ticked
car); hero card `colors.hero`/`onHero`; disabled Open button `colors.disabled`/`onDisabled`;
prices `WhichrideTheme.numbers.large/medium/small` (JetBrains Mono); "Step 1 of 2" and trip
line `numbers.small`, muted. Canvas items left out: **Recent** (R13), the clipboard card (R13),
**Discounts** pill (5b), "Car not listed? Add it" (a §5.1 proposal needs Play Integrity writes:
5b).

- [ ] **Step 1: Tokens** — in `design/tokens.json` `size` add:

```json
"monogram": { "$value": "32px", "$description": "Provider initial circles in option rows (canvas 32 px)" },
"stroke": { "$value": "1px", "$description": "Outlines of buttons, the Maps button and the pasted-link card" },
"stroke-strong": { "$value": "2px", "$description": "The selected option's ring and a ticked car's border" }
```

`design/contrast.json` pairs gain `{ "fg": "accent", "bg": "surface-highest", "kind": "non-text" }`
(progress bar on its track). Run `cd design && npm test && npm run contrast && npm run build`;
`WhichrideTokens.Size` gains `monogram`, `stroke`, `strokeStrong`. If the generator's model
rejects a token key it doesn't know, extend `design/lib/model.mjs`'s `size` handling and its test
first (it already treats `size.*` generically, so this is expected to just work).

- [ ] **Step 2: Strings** — `values/strings.xml`:

```xml
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">whichride</string>
    <string name="back">Back</string>
    <string name="close">Close</string>
    <string name="licences">Licences</string>

    <string name="home_title">Where to?</string>
    <string name="home_title_accent">to?</string>
    <string name="start_my_location">Your location</string>
    <string name="start_change">Change</string>
    <string name="destination_label">Destination</string>
    <string name="destination_hint">Address, place or Maps link</string>
    <string name="maps_link_found">Google Maps link</string>
    <string name="compare_this_trip">Compare this trip</string>
    <string name="suggestions_unavailable">Suggestions need our route service, which isn\'t reachable now. Search the address anyway, or paste a Google Maps link.</string>
    <string name="home_tip">Tip: in Google Maps, tap Share on a place and pick whichride.</string>
    <string name="error_not_a_maps_link">That link isn\'t from Google Maps.</string>
    <string name="error_no_destination">That link doesn\'t point to a place.</string>
    <string name="error_not_found">We couldn\'t find that place.</string>
    <string name="error_too_long">That text is too long to be a link.</string>

    <string name="resolving_from_maps">from Google Maps</string>
    <string name="resolving_unnamed">Your destination</string>
    <string name="resolving_place">Finding the place…</string>
    <string name="resolving_location">Finding your location…</string>
    <string name="resolving_route">Calculating the route…</string>

    <string name="manual_title_busy">Route service is busy</string>
    <string name="manual_title_offline">Can\'t reach the route service</string>
    <string name="manual_title_no_route">No route by car</string>
    <string name="manual_title_no_location">We couldn\'t find your location</string>
    <string name="manual_title_edit">Distance and time</string>
    <string name="manual_body">Enter the distance and time from Google Maps. The comparison still works.</string>
    <string name="trip_to">To %1$s</string>
    <string name="manual_km">Distance (km)</string>
    <string name="manual_minutes">Time (min)</string>
    <string name="manual_km_invalid">Between 0.1 and 2000 km</string>
    <string name="manual_minutes_invalid">Between 1 and 1440 minutes</string>
    <string name="manual_choose_start">Choose a start</string>
    <string name="compare">Compare</string>
    <string name="open_in_maps">Open in Google Maps</string>

    <string name="trip_line">%1$s km · %2$s min</string>
    <string name="built_in_prices">built-in prices</string>
    <string name="edit">Edit</string>
    <string name="loading_prices">Loading prices…</string>
    <string name="intro_title">Which cars are near you?</string>
    <string name="intro_title_accent">near you?</string>
    <string name="intro_body">Look in %1$s, then add the cars you see. We\'ll show which one is cheapest for this trip.</string>
    <string name="list_or">%1$s or %2$s</string>
    <string name="cheapest_of_options">Cheapest of your options</string>
    <string name="any_car">Any car</string>
    <string name="any_car_meta">cheapest is %1$s</string>
    <string name="package_meta">%1$s package</string>
    <string name="confirmed_today">confirmed today</string>
    <plurals name="confirmed_days_ago">
        <item quantity="one">confirmed %d day ago</item>
        <item quantity="other">confirmed %d days ago</item>
    </plurals>
    <string name="not_confirmed">not confirmed yet</string>
    <string name="built_in_price">built-in price</string>
    <string name="price_difference">+%1$s</string>
    <string name="remove_option">Remove %1$s %2$s</string>
    <string name="add_cars">Add the cars near you</string>
    <string name="add_more_cars">Add more cars</string>
    <plurals name="cheaper_not_added">
        <item quantity="one">%1$d car you did not add is cheaper: %2$s (%3$s).</item>
        <item quantity="other">%1$d cars you did not add are cheaper, from %2$s (%3$s).</item>
    </plurals>
    <string name="nothing_cheaper">No car you did not add is cheaper.</string>
    <string name="open_provider">Open %1$s</string>
    <string name="add_cars_to_compare">Add cars to compare</string>

    <string name="step_1">Step 1 of 2</string>
    <string name="step_2">Step 2 of 2</string>
    <string name="which_provider">Which provider?</string>
    <plurals name="provider_offers">
        <item quantity="one">%1$d car · from %2$s</item>
        <item quantity="other">%1$d cars · from %2$s</item>
    </plurals>
    <string name="added_count">%1$d added</string>
    <string name="which_cars">%1$s: which cars?</string>
    <string name="which_cars_body">Tick every car you see nearby. Prices are for this trip.</string>
    <string name="check_in">Check in %1$s</string>
    <string name="filter_cars">Filter %1$d cars, e.g. polo</string>
    <string name="any_provider_car">Any %1$s car</string>
    <string name="any_car_sub">I don\'t know which model</string>
    <string name="from_price">from %1$s</string>
    <string name="no_car_matches">No %1$s car matches “%2$s”.</string>
    <string name="done">Done</string>
    <string name="add_any_car">Add any %1$s car</string>
    <plurals name="add_n_cars">
        <item quantity="one">Add %d car</item>
        <item quantity="other">Add %d cars</item>
    </plurals>

    <string name="start_title">Start from</string>
    <string name="start_hint">Address or place</string>
    <string name="use_my_location">Use my location</string>

    <string name="licences_body">whichride is open source under the MIT licence. It includes the Sora and JetBrains Mono fonts, under the SIL Open Font License 1.1.</string>
</resources>
```

- [ ] **Step 3: Failing UI tests**

`ResultScreenTest.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.Screen
import io.github.locadani.whichride.compare.Target
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.OfferInfo
import io.github.locadani.whichride.core.Options
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.core.Quote
import io.github.locadani.whichride.core.RankedOption
import io.github.locadani.whichride.core.Trip
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.launch.MapsTarget
import io.github.locadani.whichride.prices.PriceSource
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

fun quote(provider: String, offer: String, name: String, cents: Long, days: UInt? = 2u) =
    Quote(provider, offer, name, "Standard", cents, false, days)

val PROVIDERS = listOf(
    ProviderInfo("miles", "MILES", "com.driveby.app", listOf(OfferInfo("miles.s", "S"))),
    ProviderInfo("bolt", "Bolt Drive", "ee.mtakso.client", listOf(OfferInfo("bolt.picanto", "Kia Picanto"), OfferInfo("bolt.polo", "VW Polo"))),
)
val QUOTES = listOf(quote("bolt", "bolt.picanto", "Kia Picanto", 790), quote("miles", "miles.s", "S", 849), quote("bolt", "bolt.polo", "VW Polo", 912))

fun resultState(ranked: List<RankedOption> = emptyList(), cheaper: List<Quote> = emptyList()) = UiState(
    stack = listOf(Screen.Home, Screen.Result),
    target = Target("Markthalle Neun", null, MapsTarget.Query("Markthalle Neun")),
    trip = Trip(7_400u, 1_320u),
    priceSource = PriceSource.SERVER,
    providers = PROVIDERS,
    quotes = QUOTES,
    options = Options(ranked, cheaper),
)

@RunWith(AndroidJUnit4::class)
class ResultScreenTest {
    @get:Rule val compose = createComposeRule()
    private var opened = 0
    private var added = 0

    private fun show(state: UiState) = compose.setContent {
        WhichrideTheme {
            ResultScreen(state, onBack = {}, onEdit = {}, onAddCars = { added++ }, onRemove = {}, onSelect = {}, onOpen = { opened++ }, onOpenMaps = {})
        }
    }

    @Test fun withoutOptionsOpenIsDisabledAndAddingIsTheWayOn() {
        show(resultState())
        compose.onNodeWithText("Add cars to compare").assertIsNotEnabled()
        compose.onNodeWithText("Which cars are near you?").assertExists()
        compose.onNodeWithText("Add the cars near you").performClick()
        assertEquals(1, added)
    }

    @Test fun theCheapestOptionIsHighlightedAndOpened() {
        val polo = RankedOption(Choice.Offer("bolt.polo"), QUOTES[2])
        val miles = RankedOption(Choice.AnyOf("miles"), QUOTES[1])
        show(resultState(listOf(miles, polo), cheaper = listOf(QUOTES[0])))
        compose.onNodeWithText("Cheapest of your options").assertExists()
        compose.onNodeWithText("€8.49").assertExists()
        compose.onNodeWithText("+€0.63").assertExists()
        compose.onNodeWithText("1 car you did not add is cheaper: €7.90 (Bolt Drive Kia Picanto).").assertExists()
        compose.onNodeWithContentDescription("Remove Bolt Drive VW Polo").assertExists()
        compose.onNodeWithText("Open MILES").assertIsEnabled().performClick()
        assertEquals(1, opened)
    }

    @Test fun theTripLineShowsKmMinutesAndBuiltInPrices() {
        show(resultState().copy(priceSource = PriceSource.BUILT_IN))
        compose.onNodeWithText("7.4 km · 22 min · built-in prices").assertExists()
    }
}
```

`CarsScreenTest.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.Screen
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.OfferInfo
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.designsystem.WhichrideTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class CarsScreenTest {
    @get:Rule val compose = createComposeRule()
    private val toggled = mutableListOf<Choice>()

    private val models = listOf("Kia Picanto", "VW Polo", "Toyota Yaris", "Opel Corsa", "Skoda Fabia", "Seat Ibiza", "Renault Clio")

    private fun show(offers: Int, draft: Set<Choice> = emptySet(), filter: String = "") {
        val names = models.take(offers)
        val bolt = ProviderInfo("bolt", "Bolt Drive", "ee.mtakso.client", names.map { OfferInfo("bolt.${it.lowercase()}", it) })
        val quotes = names.mapIndexed { i, name -> quote("bolt", "bolt.${name.lowercase()}", name, 790L + i * 10) }
        val state = resultState().copy(stack = listOf(Screen.Home, Screen.Result, Screen.Providers, Screen.Cars("bolt")),
            providers = listOf(bolt), quotes = quotes, draft = draft, carFilter = filter)
        compose.setContent {
            WhichrideTheme {
                CarsScreen(state, "bolt", onBack = {}, onCheckIn = {}, onToggle = { toggled += it }, onFilter = {}, onCommit = {})
            }
        }
    }

    @Test fun noSearchBoxForSixCarsOrFewer() {
        show(offers = 6)
        compose.onNodeWithText("Filter 6 cars, e.g. polo").assertDoesNotExist()
    }

    @Test fun aSearchBoxForMoreThanSixCars() {
        show(offers = 7)
        compose.onNodeWithText("Filter 7 cars, e.g. polo").assertExists()
    }

    @Test fun anyCarComesFirstAndTogglesItsChoice() {
        show(offers = 3)
        compose.onNodeWithText("Any Bolt Drive car").performClick()
        assertEquals(listOf<Choice>(Choice.AnyOf("bolt")), toggled)
        compose.onNodeWithText("from €7.90").assertExists()
        compose.onNodeWithText("Done").assertExists()
    }

    @Test fun theButtonCountsTheTickedCars() {
        show(offers = 3, draft = setOf(Choice.Offer("bolt.vw polo"), Choice.Offer("bolt.kia picanto")))
        compose.onNodeWithText("Add 2 cars").assertExists()
    }

    @Test fun anyCarSaysSo() {
        show(offers = 3, draft = setOf(Choice.AnyOf("bolt")))
        compose.onNodeWithText("Add any Bolt Drive car").assertExists()
    }

    @Test fun filteringHidesAnyCarAndOtherModels() {
        show(offers = 7, filter = "polo")
        compose.onNodeWithText("VW Polo").assertExists()
        compose.onNodeWithText("Kia Picanto").assertDoesNotExist()
        compose.onNodeWithText("Any Bolt Drive car").assertDoesNotExist()
    }

    @Test fun aFilterWithoutMatchesSaysSo() {
        show(offers = 7, filter = "tesla")
        compose.onNodeWithText("No Bolt Drive car matches “tesla”.").assertExists()
    }
}
```

`HomeScreenTest.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.InputError
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.Coordinate
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.server.Place
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class HomeScreenTest {
    @get:Rule val compose = createComposeRule()
    private var submitted = 0
    private val picked = mutableListOf<Place>()

    private fun show(state: UiState) = compose.setContent {
        WhichrideTheme {
            HomeScreen(state, onQuery = {}, onSubmit = { submitted++ }, onSuggestion = { picked += it }, onChangeStart = {}, onLicences = {})
        }
    }

    @Test fun aPastedLinkIsOfferedForComparison() {
        show(UiState(query = "https://maps.app.goo.gl/x", linkInQuery = true))
        compose.onNodeWithText("Compare this trip").performClick()
        assertEquals(1, submitted)
    }

    @Test fun suggestionsShowNameAndArea() {
        val place = Place("Markthalle Neun, Eisenbahnstraße 42, Berlin", Coordinate(525_022_870, 134_316_050))
        show(UiState(query = "Markth", suggestions = listOf(place)))
        compose.onNodeWithText("Eisenbahnstraße 42, Berlin").assertExists()
        compose.onNodeWithText("Markthalle Neun").performClick()
        assertEquals(listOf(place), picked)
    }

    @Test fun inputErrorsAreShown() {
        show(UiState(inputError = InputError.NOT_A_MAPS_LINK))
        compose.onNodeWithText("That link isn't from Google Maps.").assertExists()
    }

    @Test fun theTipShowsWhenIdle() {
        show(UiState())
        compose.onNodeWithText("Tip: in Google Maps, tap Share on a place and pick whichride.").assertExists()
    }
}
```

- [ ] **Step 4: Run, verify failure.**

- [ ] **Step 5: Implement** — `ui/Components.kt`

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Close
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import io.github.locadani.whichride.R
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.server.Place

private val Space = WhichrideTokens.Space
private val Size = WhichrideTokens.Size

/** The main action of a screen: full width, 56 high; disabled in the canvas's grey (decision 51). */
@Composable
fun PrimaryButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, enabled: Boolean = true, icon: ImageVector? = null) {
    Button(
        onClick = onClick,
        enabled = enabled,
        modifier = modifier.fillMaxWidth().heightIn(min = Size.controlLg),
        shape = MaterialTheme.shapes.medium,
        colors = ButtonDefaults.buttonColors(
            disabledContainerColor = WhichrideTheme.colors.disabled,
            disabledContentColor = WhichrideTheme.colors.onDisabled,
        ),
    ) {
        icon?.let {
            Icon(it, contentDescription = null)
            Spacer(Modifier.width(Space.xs))
        }
        Text(text, style = MaterialTheme.typography.labelLarge)
    }
}

/** A secondary action, outlined in the muted colour (≥ 3:1, decision 51). */
@Composable
fun SecondaryButton(text: String, onClick: () -> Unit, modifier: Modifier = Modifier, icon: ImageVector? = null) {
    OutlinedButton(
        onClick = onClick,
        modifier = modifier.fillMaxWidth().heightIn(min = Size.controlMd),
        shape = MaterialTheme.shapes.medium,
        border = BorderStroke(Size.stroke, WhichrideTheme.colors.outline),
        colors = ButtonDefaults.outlinedButtonColors(contentColor = WhichrideTheme.colors.onSurface),
    ) {
        icon?.let {
            Icon(it, contentDescription = null)
            Spacer(Modifier.width(Space.xs))
        }
        Text(text, style = MaterialTheme.typography.labelLarge)
    }
}

/** A card: the canvas's surface panels (R22: not Material's Card). */
@Composable
fun Panel(modifier: Modifier = Modifier, content: @Composable ColumnScope.() -> Unit) {
    Surface(modifier.fillMaxWidth(), shape = MaterialTheme.shapes.large, color = WhichrideTheme.colors.surface) {
        Column(Modifier.padding(Space.xs), content = content)
    }
}

/** The provider's initial in a circle. */
@Composable
fun Monogram(name: String, large: Boolean = false) {
    Box(
        Modifier.size(if (large) Size.controlMd else Size.monogram).clip(CircleShape)
            .background(WhichrideTheme.colors.surfaceHighest),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            name.take(1).uppercase(),
            style = if (large) MaterialTheme.typography.titleMedium else MaterialTheme.typography.labelMedium,
            color = WhichrideTheme.colors.onSurface,
        )
    }
}

/** A small accent pill, e.g. "2 added". */
@Composable
fun Badge(text: String) {
    Surface(shape = RoundedCornerShape(WhichrideTokens.Radius.full), color = WhichrideTheme.colors.accent, contentColor = WhichrideTheme.colors.onAccent) {
        Text(text, style = MaterialTheme.typography.labelSmall, modifier = Modifier.padding(horizontal = Space.sm, vertical = Space.xxs))
    }
}

/** Back (or close) and whatever the screen puts next to it. */
@Composable
fun TopBar(onBack: () -> Unit, close: Boolean = false, content: @Composable RowScope.() -> Unit = {}) {
    Row(
        Modifier.fillMaxWidth().padding(horizontal = Space.xs, vertical = Space.xxs),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(Space.xxs),
    ) {
        IconButton(onClick = onBack) {
            Icon(
                if (close) Icons.Filled.Close else Icons.AutoMirrored.Filled.ArrowBack,
                contentDescription = stringResource(if (close) R.string.close else R.string.back),
            )
        }
        content()
    }
}

/** A place from autocomplete: the name, then the rest of its address. */
@Composable
fun PlaceRow(place: Place, icon: ImageVector, onClick: () -> Unit) {
    val name = place.label.substringBefore(", ")
    val area = place.label.substringAfter(", ", missingDelimiterValue = "")
    PlainRow(icon = icon, title = name, subtitle = area.ifEmpty { null }, onClick = onClick)
}

@Composable
fun PlainRow(icon: ImageVector, title: String, subtitle: String?, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().clip(MaterialTheme.shapes.small).clickable(onClick = onClick)
            .heightIn(min = Size.controlLg).padding(horizontal = Space.sm, vertical = Space.xs),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(Space.sm),
    ) {
        Icon(icon, contentDescription = null, tint = WhichrideTheme.colors.onSurfaceMuted)
        Column {
            Text(title, style = MaterialTheme.typography.titleSmall)
            subtitle?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = WhichrideTheme.colors.onSurfaceMuted) }
        }
    }
}
```

`ui/Text.kt` (string helpers shared by screens):

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.runtime.Composable
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.withStyle
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.InputError
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.core.Quote
import io.github.locadani.whichride.core.RankedOption
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.format.formatMoney
import io.github.locadani.whichride.prices.PriceSource
import java.util.Locale

@Composable
fun locale(): Locale = LocalConfiguration.current.locales[0]

@Composable
fun money(cents: Long): String = formatMoney(cents, locale())

/** `title` with its `accent` part in the accent colour ("Where **to?**"). */
@Composable
fun accented(title: String, accent: String): AnnotatedString = buildAnnotatedString {
    val at = title.lastIndexOf(accent)
    if (at < 0) return@buildAnnotatedString append(title)
    append(title.substring(0, at))
    withStyle(SpanStyle(color = WhichrideTheme.colors.accentText)) { append(accent) }
    append(title.substring(at + accent.length))
}

/** "MILES, Bolt Drive or SIXT share". */
@Composable
fun orList(names: List<String>): String = when (names.size) {
    0 -> ""
    1 -> names[0]
    else -> stringResource(R.string.list_or, names.dropLast(1).joinToString(", "), names.last())
}

fun List<ProviderInfo>.nameOf(providerId: String) = firstOrNull { it.id == providerId }?.name ?: providerId

@Composable
fun offerLabel(option: RankedOption): String =
    if (option.choice is Choice.AnyOf) stringResource(R.string.any_car) else option.quote.offerName

/** "cheapest is Kia Picanto · 3 h / 40 km package · confirmed 2 days ago". */
@Composable
fun optionMeta(option: RankedOption, source: PriceSource?): String = buildList {
    if (option.choice is Choice.AnyOf) add(stringResource(R.string.any_car_meta, option.quote.offerName))
    if (option.quote.cardLabel != STANDARD_CARD) add(stringResource(R.string.package_meta, option.quote.cardLabel))
    add(freshness(option.quote, source))
}.joinToString(" · ")

/** §7.5 freshness label. */
@Composable
fun freshness(quote: Quote, source: PriceSource?): String {
    val days = quote.confirmedDaysAgo
    return when {
        source == PriceSource.BUILT_IN -> stringResource(R.string.built_in_price)
        days == null -> stringResource(R.string.not_confirmed)
        days == 0u -> stringResource(R.string.confirmed_today)
        else -> pluralStringResource(R.plurals.confirmed_days_ago, days.toInt(), days.toInt())
    }
}

fun InputError.message() = when (this) {
    InputError.NOT_A_MAPS_LINK -> R.string.error_not_a_maps_link
    InputError.NO_DESTINATION -> R.string.error_no_destination
    InputError.NOT_FOUND -> R.string.error_not_found
    InputError.TOO_LONG -> R.string.error_too_long
}

/** The catalogue's pay-as-you-go card label; any other label names a package. */
private const val STANDARD_CARD = "Standard"
```

`ui/HomeScreen.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Place
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.Start
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.server.Place
import io.github.locadani.whichride.designsystem.R as DesignR

private val Space = WhichrideTokens.Space

/** Canvas "Home: where to?" (spec §4 step 1). */
@Composable
fun HomeScreen(
    state: UiState,
    onQuery: (String) -> Unit,
    onSubmit: () -> Unit,
    onSuggestion: (Place) -> Unit,
    onChangeStart: () -> Unit,
    onLicences: () -> Unit,
) {
    val colors = WhichrideTheme.colors
    Column(Modifier.fillMaxSize().imePadding()) {
        Row(
            Modifier.fillMaxWidth().padding(start = Space.lg, end = Space.sm, top = Space.md),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            Image(painterResource(DesignR.drawable.whichride_logo), contentDescription = null)
            Text(stringResource(R.string.app_name), style = MaterialTheme.typography.titleLarge, modifier = Modifier.weight(1f))
            TextButton(onClick = onLicences) { Text(stringResource(R.string.licences)) }
        }
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = Space.md, vertical = Space.xl),
            verticalArrangement = Arrangement.spacedBy(Space.md),
        ) {
            Text(
                accented(stringResource(R.string.home_title), stringResource(R.string.home_title_accent)),
                style = MaterialTheme.typography.headlineLarge,
                modifier = Modifier.padding(horizontal = Space.xxs).semantics { heading() },
            )
            Panel {
                Row(verticalAlignment = Alignment.CenterVertically, modifier = Modifier.padding(start = Space.sm)) {
                    Icon(Icons.Outlined.LocationOn, contentDescription = null, tint = colors.onSurfaceMuted)
                    Text(
                        when (val start = state.start) {
                            Start.MyLocation -> stringResource(R.string.start_my_location)
                            is Start.Chosen -> start.place.label
                        },
                        style = MaterialTheme.typography.bodyLarge,
                        color = colors.onSurfaceMuted,
                        maxLines = 1,
                        modifier = Modifier.weight(1f).padding(start = Space.sm),
                    )
                    TextButton(onClick = onChangeStart) { Text(stringResource(R.string.start_change), color = colors.accentText) }
                }
                HorizontalDivider(color = colors.outlineVariant)
                val label = stringResource(R.string.destination_label)
                TextField(
                    value = state.query,
                    onValueChange = onQuery,
                    modifier = Modifier.fillMaxWidth().semantics { contentDescription = label },
                    placeholder = { Text(stringResource(R.string.destination_hint)) },
                    leadingIcon = { Icon(Icons.Filled.Place, contentDescription = null, tint = colors.accent) },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Uri, imeAction = ImeAction.Search),
                    keyboardActions = KeyboardActions(onSearch = { onSubmit() }),
                    colors = TextFieldDefaults.colors(
                        focusedContainerColor = colors.surface,
                        unfocusedContainerColor = colors.surface,
                        unfocusedIndicatorColor = colors.surface,
                    ),
                )
            }
            state.inputError?.let {
                Text(stringResource(it.message()), color = colors.error, style = MaterialTheme.typography.bodyMedium)
            }
            when {
                state.linkInQuery -> Surface(
                    shape = MaterialTheme.shapes.large,
                    color = colors.background,
                    border = BorderStroke(WhichrideTokens.Size.stroke, colors.accent),
                ) {
                    Column(Modifier.padding(Space.md), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
                        Text(stringResource(R.string.maps_link_found), style = MaterialTheme.typography.labelMedium, color = colors.accentText)
                        PrimaryButton(stringResource(R.string.compare_this_trip), onSubmit)
                    }
                }
                state.suggestions.isNotEmpty() -> Panel {
                    state.suggestions.forEach { place -> PlaceRow(place, Icons.Outlined.Place) { onSuggestion(place) } }
                }
                state.suggestionsUnavailable -> Text(
                    stringResource(R.string.suggestions_unavailable),
                    style = MaterialTheme.typography.bodySmall,
                    color = colors.onSurfaceMuted,
                )
                else -> Text(
                    stringResource(R.string.home_tip),
                    style = MaterialTheme.typography.bodySmall,
                    color = colors.onSurfaceMuted,
                    modifier = Modifier.padding(horizontal = Space.xxs),
                )
            }
        }
    }
}
```

`ui/ResolvingScreen.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.LiveRegionMode
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.liveRegion
import androidx.compose.ui.semantics.semantics
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.ResolvingStep
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens

/** Canvas "Shared from Maps": the place, then what the app is doing. */
@Composable
fun ResolvingScreen(state: UiState, onCancel: () -> Unit) {
    val colors = WhichrideTheme.colors
    Column(Modifier.fillMaxSize()) {
        TopBar(onCancel, close = true)
        Column(
            Modifier.weight(1f).padding(horizontal = WhichrideTokens.Space.xxl),
            verticalArrangement = Arrangement.spacedBy(WhichrideTokens.Space.md, androidx.compose.ui.Alignment.CenterVertically),
        ) {
            Text(stringResource(R.string.resolving_from_maps), style = WhichrideTheme.numbers.small, color = colors.onSurfaceMuted)
            Text(
                state.target?.label ?: stringResource(R.string.resolving_unnamed),
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.semantics { heading() },
            )
            Text(
                stringResource(
                    when (state.step) {
                        ResolvingStep.FINDING_LOCATION -> R.string.resolving_location
                        ResolvingStep.ROUTING -> R.string.resolving_route
                        ResolvingStep.FINDING_PLACE, null -> R.string.resolving_place
                    },
                ),
                style = MaterialTheme.typography.bodyLarge,
                modifier = Modifier.semantics { liveRegion = LiveRegionMode.Polite },
            )
            LinearProgressIndicator(Modifier.fillMaxWidth(), color = colors.accent, trackColor = colors.surfaceHighest)
        }
    }
}
```

`ui/ManualTripScreen.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Place
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.ManualReason
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens

private val Space = WhichrideTokens.Space

/** Canvas "Route service busy": km and minutes typed from Google Maps (§8). */
@Composable
fun ManualTripScreen(
    state: UiState,
    onBack: () -> Unit,
    onKm: (String) -> Unit,
    onMinutes: (String) -> Unit,
    onSubmit: () -> Unit,
    onOpenMaps: () -> Unit,
    onChooseStart: () -> Unit,
) {
    Column(Modifier.fillMaxSize().imePadding()) {
        TopBar(onBack)
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = Space.md, vertical = Space.sm),
            verticalArrangement = Arrangement.spacedBy(Space.md),
        ) {
            Text(
                stringResource(
                    when (state.manualReason) {
                        ManualReason.BUSY -> R.string.manual_title_busy
                        ManualReason.NO_ROUTE -> R.string.manual_title_no_route
                        ManualReason.NO_LOCATION -> R.string.manual_title_no_location
                        ManualReason.EDIT -> R.string.manual_title_edit
                        ManualReason.OFFLINE, null -> R.string.manual_title_offline
                    },
                ),
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.semantics { heading() },
            )
            state.target?.label?.let { Text(stringResource(R.string.trip_to, it), style = MaterialTheme.typography.titleSmall) }
            Text(stringResource(R.string.manual_body), style = MaterialTheme.typography.bodyMedium, color = WhichrideTheme.colors.onSurfaceMuted)
            Row(horizontalArrangement = Arrangement.spacedBy(Space.sm)) {
                NumberField(
                    state.manualKm, onKm, stringResource(R.string.manual_km),
                    error = stringResource(R.string.manual_km_invalid).takeIf { state.manualKmInvalid },
                    keyboard = KeyboardType.Decimal, imeAction = ImeAction.Next, onDone = {},
                    modifier = Modifier.weight(1f),
                )
                NumberField(
                    state.manualMinutes, onMinutes, stringResource(R.string.manual_minutes),
                    error = stringResource(R.string.manual_minutes_invalid).takeIf { state.manualMinutesInvalid },
                    keyboard = KeyboardType.Number, imeAction = ImeAction.Done, onDone = onSubmit,
                    modifier = Modifier.weight(1f),
                )
            }
            if (state.manualReason == ManualReason.NO_LOCATION) {
                SecondaryButton(stringResource(R.string.manual_choose_start), onChooseStart, icon = Icons.Outlined.LocationOn)
            }
            if (state.target != null) SecondaryButton(stringResource(R.string.open_in_maps), onOpenMaps, icon = Icons.Filled.Place)
        }
        PrimaryButton(stringResource(R.string.compare), onSubmit, Modifier.padding(Space.md))
    }
}

@Composable
private fun NumberField(
    value: String,
    onChange: (String) -> Unit,
    label: String,
    error: String?,
    keyboard: KeyboardType,
    imeAction: ImeAction,
    onDone: () -> Unit,
    modifier: Modifier,
) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        modifier = modifier,
        label = { Text(label) },
        isError = error != null,
        supportingText = error?.let { { Text(it) } },
        textStyle = WhichrideTheme.numbers.medium,
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = keyboard, imeAction = imeAction),
        keyboardActions = KeyboardActions(onDone = { onDone() }),
    )
}
```

`ui/ResultScreen.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.selection.selectable
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Place
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedIconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.RankedOption
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.format.formatKilometres
import io.github.locadani.whichride.format.formatMinutes
import io.github.locadani.whichride.prices.PriceSource

private val Space = WhichrideTokens.Space
private val Size = WhichrideTokens.Size

/** Canvas "Result + add the cars near you" (spec §4 steps 6–7, §4.1). */
@Composable
fun ResultScreen(
    state: UiState,
    onBack: () -> Unit,
    onEdit: () -> Unit,
    onAddCars: () -> Unit,
    onRemove: (Choice) -> Unit,
    onSelect: (Choice) -> Unit,
    onOpen: () -> Unit,
    onOpenMaps: () -> Unit,
) {
    val colors = WhichrideTheme.colors
    val ranked = state.options?.ranked.orEmpty()
    val chosen = ranked.firstOrNull { it.choice == state.selected } ?: ranked.firstOrNull()
    Column(Modifier.fillMaxSize()) {
        TopBar(onBack) {
            Column(Modifier.weight(1f)) {
                Text(
                    stringResource(R.string.trip_to, state.target?.label ?: stringResource(R.string.resolving_unnamed)),
                    style = MaterialTheme.typography.titleMedium, maxLines = 1, overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.semantics { heading() },
                )
                state.trip?.let { trip ->
                    val line = stringResource(R.string.trip_line, formatKilometres(trip.metres, locale()), formatMinutes(trip.seconds))
                    val builtIn = if (state.priceSource == PriceSource.BUILT_IN) " · " + stringResource(R.string.built_in_prices) else ""
                    Text(line + builtIn, style = WhichrideTheme.numbers.small, color = colors.onSurfaceMuted)
                }
            }
            TextButton(onClick = onEdit) { Text(stringResource(R.string.edit), color = colors.accentText) }
        }
        val options = state.options
        if (options == null) {
            Column(Modifier.weight(1f).padding(Space.md), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
                Text(stringResource(R.string.loading_prices), style = MaterialTheme.typography.bodyMedium)
                LinearProgressIndicator(Modifier.fillMaxWidth(), color = colors.accent, trackColor = colors.surfaceHighest)
            }
        } else {
            LazyColumn(
                Modifier.weight(1f),
                contentPadding = PaddingValues(horizontal = Space.md, vertical = Space.sm),
                verticalArrangement = Arrangement.spacedBy(Space.xs),
            ) {
                if (ranked.isEmpty()) {
                    item { Intro(state) }
                } else {
                    item { Hero(ranked.first(), state, selected = chosen == ranked.first() && ranked.size > 1, onSelect, onRemove) }
                    items(ranked.drop(1), key = { it.choice.toString() }) { option ->
                        OptionRow(option, ranked.first().quote.totalCents, state, selected = chosen == option, onSelect, onRemove)
                    }
                }
                item {
                    if (ranked.isEmpty()) PrimaryButton(stringResource(R.string.add_cars), onAddCars, icon = Icons.Filled.Add)
                    else SecondaryButton(stringResource(R.string.add_more_cars), onAddCars, icon = Icons.Filled.Add)
                }
                if (ranked.isNotEmpty()) item { CheaperLine(state) }
            }
        }
        Row(
            Modifier.padding(horizontal = Space.md, vertical = Space.sm),
            horizontalArrangement = Arrangement.spacedBy(Space.sm),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OutlinedIconButton(
                onClick = onOpenMaps,
                modifier = Modifier.size(Size.controlLg),
                shape = MaterialTheme.shapes.medium,
                border = BorderStroke(Size.stroke, colors.outline),
                colors = IconButtonDefaults.outlinedIconButtonColors(containerColor = colors.surface),
            ) { Icon(Icons.Filled.Place, contentDescription = stringResource(R.string.open_in_maps)) }
            PrimaryButton(
                text = chosen?.let { stringResource(R.string.open_provider, state.providers.nameOf(it.quote.providerId)) }
                    ?: stringResource(R.string.add_cars_to_compare),
                onClick = onOpen,
                enabled = chosen != null,
                modifier = Modifier.weight(1f),
            )
        }
    }
}

@Composable
private fun Intro(state: UiState) {
    Panel {
        Column(Modifier.padding(Space.md), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
            Text(
                accented(stringResource(R.string.intro_title), stringResource(R.string.intro_title_accent)),
                style = MaterialTheme.typography.headlineSmall,
            )
            Text(
                stringResource(R.string.intro_body, orList(state.providers.map { it.name })),
                style = MaterialTheme.typography.bodyMedium,
                color = WhichrideTheme.colors.onSurfaceMuted,
            )
        }
    }
}

@Composable
private fun Hero(option: RankedOption, state: UiState, selected: Boolean, onSelect: (Choice) -> Unit, onRemove: (Choice) -> Unit) {
    val colors = WhichrideTheme.colors
    val provider = state.providers.nameOf(option.quote.providerId)
    Surface(
        shape = MaterialTheme.shapes.large,
        color = colors.hero,
        contentColor = colors.onHero,
        border = if (selected) BorderStroke(Size.strokeStrong, colors.onSurface) else null,
    ) {
        Row {
            Column(
                Modifier.weight(1f).selectable(selected, onClick = { onSelect(option.choice) }, role = Role.RadioButton)
                    .padding(start = Space.lg, top = Space.md, bottom = Space.md),
                verticalArrangement = Arrangement.spacedBy(Space.xs),
            ) {
                Text(stringResource(R.string.cheapest_of_options).uppercase(locale()), style = MaterialTheme.typography.labelSmall)
                Row(verticalAlignment = Alignment.Bottom) {
                    Column(Modifier.weight(1f)) {
                        Text(provider, style = MaterialTheme.typography.titleLarge)
                        Text("${offerLabel(option)} · ${optionMeta(option, state.priceSource)}", style = MaterialTheme.typography.bodySmall)
                    }
                    Text(money(option.quote.totalCents), style = WhichrideTheme.numbers.large)
                }
            }
            RemoveButton(option, provider, onRemove)
        }
    }
}

@Composable
private fun OptionRow(option: RankedOption, best: Long, state: UiState, selected: Boolean, onSelect: (Choice) -> Unit, onRemove: (Choice) -> Unit) {
    val colors = WhichrideTheme.colors
    val provider = state.providers.nameOf(option.quote.providerId)
    Surface(
        shape = MaterialTheme.shapes.small,
        color = colors.surface,
        border = if (selected) BorderStroke(Size.strokeStrong, colors.accent) else null,
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Row(
                Modifier.weight(1f).heightIn(min = Size.controlLg)
                    .selectable(selected, onClick = { onSelect(option.choice) }, role = Role.RadioButton)
                    .padding(start = Space.md, top = Space.xs, bottom = Space.xs),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(Space.sm),
            ) {
                Monogram(provider)
                Column(Modifier.weight(1f)) {
                    Text("$provider ${offerLabel(option)}", style = MaterialTheme.typography.titleSmall)
                    Text(optionMeta(option, state.priceSource), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
                }
                Column(horizontalAlignment = Alignment.End) {
                    Text(money(option.quote.totalCents), style = WhichrideTheme.numbers.medium)
                    Text(
                        stringResource(R.string.price_difference, money(option.quote.totalCents - best)),
                        style = WhichrideTheme.numbers.small,
                        color = colors.onSurfaceMuted,
                    )
                }
            }
            RemoveButton(option, provider, onRemove)
        }
    }
}

@Composable
private fun RemoveButton(option: RankedOption, provider: String, onRemove: (Choice) -> Unit) {
    IconButton(onClick = { onRemove(option.choice) }) {
        Icon(Icons.Filled.Close, contentDescription = stringResource(R.string.remove_option, provider, offerLabel(option)))
    }
}

@Composable
private fun CheaperLine(state: UiState) {
    val cheaper = state.options?.cheaperNotAdded.orEmpty()
    val text = cheaper.firstOrNull()?.let { first ->
        pluralStringResource(
            R.plurals.cheaper_not_added, cheaper.size, cheaper.size, money(first.totalCents),
            "${state.providers.nameOf(first.providerId)} ${first.offerName}",
        )
    } ?: stringResource(R.string.nothing_cheaper)
    Text(text, style = MaterialTheme.typography.bodySmall, color = WhichrideTheme.colors.onSurfaceMuted, modifier = Modifier.padding(Space.xxs))
}
```

`ui/ProvidersScreen.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.options.forProvider

private val Space = WhichrideTokens.Space

/** §4.1 step 1: which provider? One large button each, with its offers, lowest price and how many are added. */
@Composable
fun ProvidersScreen(state: UiState, onClose: () -> Unit, onPick: (String) -> Unit) {
    Column(Modifier.fillMaxSize()) {
        TopBar(onClose, close = true) {
            Text(stringResource(R.string.step_1), style = WhichrideTheme.numbers.small, color = WhichrideTheme.colors.onSurfaceMuted)
        }
        Text(
            stringResource(R.string.which_provider),
            style = MaterialTheme.typography.headlineMedium,
            modifier = Modifier.padding(horizontal = Space.lg, vertical = Space.sm).semantics { heading() },
        )
        LazyColumn(contentPadding = PaddingValues(Space.md), verticalArrangement = Arrangement.spacedBy(Space.sm)) {
            items(state.providers, key = { it.id }) { provider ->
                val quotes = state.quotes.filter { it.providerId == provider.id }
                val chosen = state.choices.forProvider(provider.id, provider.offers.mapTo(mutableSetOf()) { it.id })
                val added = if (chosen.any { it is Choice.AnyOf }) provider.offers.size else chosen.size
                Surface(
                    onClick = { onPick(provider.id) },
                    shape = MaterialTheme.shapes.large,
                    color = WhichrideTheme.colors.surface,
                ) {
                    Row(
                        Modifier.fillMaxWidth().heightIn(min = WhichrideTokens.Size.controlLg).padding(Space.md),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Space.md),
                    ) {
                        Monogram(provider.name, large = true)
                        Column(Modifier.weight(1f)) {
                            Text(provider.name, style = MaterialTheme.typography.titleLarge)
                            quotes.minOfOrNull { it.totalCents }?.let { from ->
                                Text(
                                    pluralStringResource(R.plurals.provider_offers, provider.offers.size, provider.offers.size, money(from)),
                                    style = MaterialTheme.typography.bodySmall,
                                    color = WhichrideTheme.colors.onSurfaceMuted,
                                )
                            }
                        }
                        if (added > 0) Badge(stringResource(R.string.added_count, added))
                        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
                    }
                }
            }
        }
    }
}
```

`ui/CarsScreen.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.selection.toggleable
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowForward
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.Checkbox
import androidx.compose.material3.CheckboxDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens

private val Space = WhichrideTokens.Space

/** Offers above this get a search box (§4.1, decision 50). */
private const val SEARCH_ABOVE = 6

/** §4.1 step 2: tick every car of this provider that is nearby. */
@Composable
fun CarsScreen(
    state: UiState,
    providerId: String,
    onBack: () -> Unit,
    onCheckIn: (String) -> Unit,
    onToggle: (Choice) -> Unit,
    onFilter: (String) -> Unit,
    onCommit: () -> Unit,
) {
    val colors = WhichrideTheme.colors
    val provider = state.providers.firstOrNull { it.id == providerId } ?: return
    val quotes = state.quotes.filter { it.providerId == providerId }
    val anyCar = Choice.AnyOf(providerId)
    val anyOn = anyCar in state.draft
    val terms = state.carFilter.lowercase().split(Regex("\\s+")).filter(String::isNotBlank)
    val matching = quotes.filter { q -> terms.all { q.offerName.lowercase().contains(it) } }.sortedBy { it.offerName }
    Column(Modifier.fillMaxSize().imePadding()) {
        TopBar(onBack) {
            Text(stringResource(R.string.step_2), style = WhichrideTheme.numbers.small, color = colors.onSurfaceMuted, modifier = Modifier.weight(1f))
            OutlinedButton(
                onClick = { onCheckIn(providerId) },
                shape = MaterialTheme.shapes.medium,
                border = BorderStroke(WhichrideTokens.Size.stroke, colors.outline),
            ) {
                Text(stringResource(R.string.check_in, provider.name), color = colors.onSurface)
                Icon(Icons.AutoMirrored.Filled.ArrowForward, contentDescription = null, tint = colors.onSurface)
            }
        }
        Column(Modifier.padding(horizontal = Space.lg, vertical = Space.xs), verticalArrangement = Arrangement.spacedBy(Space.xxs)) {
            Text(
                stringResource(R.string.which_cars, provider.name),
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.semantics { heading() },
            )
            Text(stringResource(R.string.which_cars_body), style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceMuted)
        }
        if (provider.offers.size > SEARCH_ABOVE) {
            OutlinedTextField(
                value = state.carFilter,
                onValueChange = onFilter,
                modifier = Modifier.fillMaxWidth().padding(horizontal = Space.md, vertical = Space.xs),
                placeholder = { Text(stringResource(R.string.filter_cars, provider.offers.size)) },
                leadingIcon = { Icon(Icons.Filled.Search, contentDescription = null) },
                singleLine = true,
                shape = MaterialTheme.shapes.small,
            )
        }
        LazyColumn(
            Modifier.weight(1f),
            contentPadding = PaddingValues(horizontal = Space.md, vertical = Space.xs),
            verticalArrangement = Arrangement.spacedBy(Space.xs),
        ) {
            if (terms.isEmpty()) {
                item {
                    CarRow(
                        label = stringResource(R.string.any_provider_car, provider.name),
                        sub = stringResource(R.string.any_car_sub),
                        price = quotes.minOfOrNull { it.totalCents }?.let { stringResource(R.string.from_price, money(it)) }.orEmpty(),
                        checked = anyOn,
                        strong = true,
                        onToggle = { onToggle(anyCar) },
                    )
                }
            }
            items(matching, key = { it.offerId }) { quote ->
                val choice = Choice.Offer(quote.offerId)
                CarRow(quote.offerName, null, money(quote.totalCents), anyOn || choice in state.draft, strong = false) { onToggle(choice) }
            }
            if (terms.isNotEmpty() && matching.isEmpty()) {
                item {
                    Text(
                        stringResource(R.string.no_car_matches, provider.name, state.carFilter.trim()),
                        style = MaterialTheme.typography.bodyLarge,
                        color = colors.onSurfaceMuted,
                        modifier = Modifier.padding(Space.lg),
                    )
                }
            }
        }
        val label = when {
            state.draft.isEmpty() -> stringResource(R.string.done)
            anyOn -> stringResource(R.string.add_any_car, provider.name)
            else -> pluralStringResource(R.plurals.add_n_cars, state.draft.size, state.draft.size)
        }
        PrimaryButton(label, onCommit, Modifier.padding(Space.md))
    }
}

@Composable
private fun CarRow(label: String, sub: String?, price: String, checked: Boolean, strong: Boolean, onToggle: () -> Unit) {
    val colors = WhichrideTheme.colors
    Surface(
        shape = MaterialTheme.shapes.small,
        color = colors.surface,
        border = if (checked) BorderStroke(WhichrideTokens.Size.strokeStrong, colors.accent) else null,
    ) {
        Row(
            Modifier.fillMaxWidth().heightIn(min = WhichrideTokens.Size.controlLg)
                .toggleable(value = checked, role = Role.Checkbox, onValueChange = { onToggle() })
                .padding(horizontal = Space.sm, vertical = Space.xs),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            Checkbox(
                checked = checked,
                onCheckedChange = null,
                colors = CheckboxDefaults.colors(checkedColor = colors.accent, uncheckedColor = colors.outline, checkmarkColor = colors.onAccent),
            )
            Column(Modifier.weight(1f)) {
                Text(label, style = if (strong) MaterialTheme.typography.titleMedium else MaterialTheme.typography.titleSmall)
                sub?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted) }
            }
            Text(price, style = WhichrideTheme.numbers.medium)
        }
    }
}
```

`ui/StartScreen.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.outlined.LocationOn
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.server.Place

private val Space = WhichrideTokens.Space

/** Change the start (§4 step 3, R20). */
@Composable
fun StartScreen(state: UiState, onBack: () -> Unit, onQuery: (String) -> Unit, onPick: (Place) -> Unit, onMyLocation: () -> Unit) {
    Column(Modifier.fillMaxSize().imePadding()) {
        TopBar(onBack)
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = Space.md),
            verticalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            Text(stringResource(R.string.start_title), style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
            OutlinedTextField(
                value = state.startQuery,
                onValueChange = onQuery,
                modifier = Modifier.fillMaxWidth(),
                placeholder = { Text(stringResource(R.string.start_hint)) },
                leadingIcon = { Icon(Icons.Filled.Search, contentDescription = null) },
                singleLine = true,
                shape = MaterialTheme.shapes.small,
            )
            Panel {
                PlainRow(Icons.Outlined.LocationOn, stringResource(R.string.use_my_location), null, onMyLocation)
                state.startSuggestions.forEach { place -> PlaceRow(place, Icons.Outlined.Place) { onPick(place) } }
            }
        }
    }
}
```

`ui/LicencesScreen.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import io.github.locadani.whichride.R
import io.github.locadani.whichride.designsystem.WhichrideTokens

private val FILES = listOf("whichride-LICENSE.txt", "Sora-OFL.txt", "JetBrainsMono-OFL.txt")

/** The licences that travel with the app (R29); copied from the repository at build time. */
@Composable
fun LicencesScreen(onBack: () -> Unit) {
    val assets = LocalContext.current.assets
    val texts = remember { FILES.map { name -> name to assets.open("licences/$name").bufferedReader().use { it.readText() } } }
    Column(Modifier.fillMaxSize()) {
        TopBar(onBack)
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = WhichrideTokens.Space.md),
            verticalArrangement = Arrangement.spacedBy(WhichrideTokens.Space.md),
        ) {
            Text(stringResource(R.string.licences), style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
            Text(stringResource(R.string.licences_body), style = MaterialTheme.typography.bodyMedium)
            texts.forEach { (name, text) ->
                Text(name, style = MaterialTheme.typography.titleSmall, modifier = Modifier.semantics { heading() })
                Text(text, style = MaterialTheme.typography.bodySmall)
            }
        }
    }
}
```

`ui/WhichrideNav.kt`:

```kotlin
package io.github.locadani.whichride.ui

import android.Manifest
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.locadani.whichride.compare.CompareViewModel
import io.github.locadani.whichride.compare.Screen
import io.github.locadani.whichride.designsystem.WhichrideTheme

/** Shows the top of the ViewModel's back stack (decision 63) and asks for location when it says so. */
@Composable
fun WhichrideNav(viewModel: CompareViewModel) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    BackHandler(enabled = state.stack.size > 1) { viewModel.back() }
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestMultiplePermissions()) {
        viewModel.onLocationAnswered()
    }
    LaunchedEffect(state.askLocation) {
        if (state.askLocation) {
            permission.launch(arrayOf(Manifest.permission.ACCESS_FINE_LOCATION, Manifest.permission.ACCESS_COARSE_LOCATION))
        }
    }
    Surface(Modifier.fillMaxSize(), color = WhichrideTheme.colors.background, contentColor = WhichrideTheme.colors.onSurface) {
        Box(Modifier.safeDrawingPadding()) {
            when (val screen = state.screen) {
                Screen.Home -> HomeScreen(
                    state, viewModel::onQuery, viewModel::submitQuery, viewModel::pickSuggestion,
                    viewModel::changeStart, viewModel::openLicences,
                )
                Screen.Resolving -> ResolvingScreen(state, onCancel = { viewModel.back() })
                Screen.ManualTrip -> ManualTripScreen(
                    state, onBack = { viewModel.back() }, viewModel::onManualKm, viewModel::onManualMinutes,
                    viewModel::submitManual, viewModel::openMaps, viewModel::changeStart,
                )
                Screen.Result -> ResultScreen(
                    state, onBack = { viewModel.back() }, viewModel::editTrip, viewModel::addCars, viewModel::remove,
                    viewModel::select, viewModel::openChosen, viewModel::openMaps,
                )
                Screen.Providers -> ProvidersScreen(state, onClose = { viewModel.back() }, viewModel::pickProvider)
                is Screen.Cars -> CarsScreen(
                    state, screen.providerId, onBack = { viewModel.back() }, viewModel::checkIn, viewModel::toggle,
                    viewModel::onCarFilter, viewModel::commitCars,
                )
                Screen.Start -> StartScreen(state, onBack = { viewModel.back() }, viewModel::onStartQuery, viewModel::pickStart, viewModel::useMyLocation)
                Screen.Licences -> LicencesScreen(onBack = { viewModel.back() })
            }
        }
    }
}
```

`MainActivity` from Task 9 calls `WhichrideNav(viewModel)` (no `onExit`: with one screen left
the BackHandler is disabled and the system finishes the activity).

- [ ] **Step 6: Run**

```sh
cd android && ./gradlew :app:testDebugUnitTest :app:lintDebug :app:assembleDebug
cd ../design && npm test
```

Expected: all green; lint has no errors (fix warnings that point at real problems: missing
`contentDescription`, hard-coded text). The single-source guard reports nothing.

- [ ] **Step 7: Commit** — `feat(android): result, provider and car screens from the design canvas`

---

### Task 11: CI, docs, decisions, device check, review

**Files:**
- Modify: `.github/workflows/ci.yml`, `docs/DECISIONS.md`, `docs/plans/ROADMAP.md`, `README.md` (Android line, if it lists components)
- Create: `android/README.md`

- [ ] **Step 1: CI** — the android job's last step becomes:

```yaml
      - run: >-
          ./gradlew --no-daemon
          :designsystem:testDebugUnitTest :designsystem:lintDebug :designsystem:assembleRelease
          :core:testDebugUnitTest
          :app:testDebugUnitTest :app:lintDebug :app:assembleDebug :app:assembleRelease
```

- [ ] **Step 2: Decisions 58–64** appended to `docs/DECISIONS.md` (date 2026-10-20), one row
  each, from rulings R2+R4+R7 (58), R9 (59), R11 (60), R13 (61), R14 (62), R17 (63), R3 (64),
  with the alternatives and reasons from the ledger.

- [ ] **Step 3: ROADMAP** — row 5 becomes two rows:

| 5a | **Android app v1: core flow** | UniFFI bindings built from Gradle; `:app`; share-sheet and `geo:` entry, pasted links, typed addresses with server autocomplete; location; result screen; adding the cars near you (§4.1); Open <provider> (SIXT through Jelbi) and Open in Google Maps; manual km/min fallback; install registration (Play Integrity) for routing | §4, §4.1, §7.5, §8, §10.3 | `05a-android-core-flow.md` ✅ |
| 5b | **Android app v1: personal and crowd** | Personal discounts and passes (§6), price reports and confirmations with Play Integrity (§7), "Car not listed? Add it" (§5.1), per-user choice of which app opens a provider, EN/IT/DE/TR, accessibility pass (TalkBack, 200 % font) | §5.1, §6, §7, §11 | — |

(The ✅ goes in only after Step 6 passes.)

- [ ] **Step 4: `android/README.md`**

```markdown
# whichride for Android

Gradle project with three modules: `:designsystem` (theme generated from `design/tokens.json`),
`:core` (the Rust core through UniFFI, built by Gradle) and `:app`.

## Build

Needs JDK 21, the Android SDK with NDK 30.0.16248370, Rust (the workspace toolchain) with
`rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`, and
`cargo install cargo-ndk --version 4.1.2`.

```sh
export JAVA_HOME=/opt/homebrew/opt/openjdk@21
./gradlew :app:assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
./gradlew :core:testDebugUnitTest :app:testDebugUnitTest   # JVM tests run the real Rust (host build)
```

## Server

Without a server the app compares with the prices built into the APK and asks for km and minutes.
To use a server, set in `~/.gradle/gradle.properties` (not in the repository):

```properties
whichride.serverUrl=https://…            # or http://localhost:8080 with `adb reverse tcp:8080 tcp:8080` (debug builds)
whichride.cloudProjectNumber=123456789   # Play Integrity; registration needs it
```

Play Integrity verdicts only pass for builds installed from Google Play (milestone 6); a sideloaded
debug build can't register with a production server, so routing falls back to manual entry.
```

(Fold that last sentence into a ruling row if review finds it surprising: R9 already implies it.)

- [ ] **Step 5: Full verification**

```sh
cargo fmt --all --check && RUSTFLAGS=-D\ warnings cargo clippy --workspace --all-targets --locked && cargo test --workspace --locked
(cd design && npm test && npm run contrast)
(cd android && ./gradlew :designsystem:testDebugUnitTest :core:testDebugUnitTest :app:testDebugUnitTest :app:lintDebug :app:assembleDebug :app:assembleRelease)
```

All green; record the test counts.

- [ ] **Step 6: Device check** — `~/Library/Android/sdk/platform-tools/adb devices`. If a device
  is listed: `adb install -r android/app/build/outputs/apk/debug/app-debug.apk`, then on the phone
  share a place from Google Maps to whichride → manual entry (no server) → type km/min → add a
  MILES and a Bolt car → Open opens the provider; Open in Google Maps opens the short link. If no
  device: give the owner the install command.

- [ ] **Step 7: Commit** — `docs: decisions 58–64, roadmap split 5a/5b, Android README`; then
  `ci: build and test the Android app with the Rust core`.

- [ ] **Step 8: Review** — one fresh reviewer (most capable model) over `git diff main...HEAD`
  against this plan and the spec; fix every Critical and Important finding (one commit per fix
  or a single `fix(android): review findings`), list Minor ones as deferred in the final summary.

- [ ] **Step 9: Push** — `git push -u origin feat/m5a-android-core-flow` (SSH alias remote). The
  owner opens the PR.

---

## Self-review

- **Spec coverage:** §4 step 1 (share, `geo:`, paste, typed + autocomplete) → Tasks 4, 6, 9, 10;
  step 2 (short links, coordinates, geocoding) → Tasks 1–2, 9; step 3 (location, changeable start)
  → Tasks 8, 9, 10; step 4 (km/min from the server, manual fallback) → Tasks 6, 9, 10; step 5
  (core prices) → Tasks 2, 7; step 6 + §4.1 (cars near you, ranking, cheaper elsewhere, 15 min,
  search over 6, any car) → Tasks 1, 5, 9, 10; step 7 (Open provider + Play Store fallback, Open in
  Google Maps) → Tasks 8, 9, 10; §7.5 freshness → Tasks 2, 10; §8 manual entry always available →
  Task 9 (`editTrip`); §9 privacy → body-only coordinates (Task 6 test), no persisted locations
  (R17), backup off (Task 4); §10.3 → Tasks 3–4; §11 touch targets → tokens, Material minimum.
  Out of scope by owner: §6, §7.1–7.4, languages, accessibility pass (5b).
- **Placeholders:** none intended; two steps say how to adapt if a tool behaves differently
  (bindgen input, Robolectric intent matching) — those are contingencies with the exact fallback.
- **Type consistency:** `forProvider(providerId, offerIds)` everywhere (Task 5 interface line
  corrected in the note); `PriceLoader.offline()/load()`; `ChoiceStore`; `Routes.expand` returns
  `ApiResult<Destination>`; `Quote.totalCents: Long`, `confirmedDaysAgo: UInt?`, `Trip(metres: UInt, seconds: UInt)`.
