# Milestone 5b: Android app v1, personal and crowd — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The app applies the user's own discounts, passes, refuel credits and price corrections
(§6), sends price reports, confirmations and new-car proposals with Play Integrity (§7, §5.1),
lets the user pick which app opens a provider, speaks English, Italian, German and Turkish, and
passes an accessibility check (§11). The 5a leftovers are fixed on the way.

**Architecture:** The core gains `report` (the limits on reported values, now shared by server
and app) and providers gain alternative apps. `crates/ffi` passes personal adjustments into
`quotes`/`options`, exposes pending reports, locks and proposals from the server's catalogue,
decides when to ask the §7.2 question (`Prices.question`) and writes the exact JSON bodies of
reports and proposals. The app keeps everything personal in one JSON file (`PersonalStore`),
signs writes through `Installs.signed` (integrity token bound to install ID, path and body,
decision 44) behind a `Crowd` interface, and adds screens for settings, discounts, price
corrections, proposals and the confirmation sheet. All rules stay in Rust or in small pure Kotlin
files with JVM tests; screens stay stateless Robolectric-tested composables.

**Tech Stack:** as milestone 5a (Rust workspace toolchain, UniFFI 0.32.2; Gradle 9.8.1, AGP
9.4.1, Kotlin 2.4.21, Compose BOM 2026.09.00 with Material 3, OkHttp 5.5.0, kotlinx
serialization 1.12.0, coroutines 1.11.0, Play Integrity 1.6.0, JUnit 4.13.2, Robolectric 4.17).
No new dependency.

**Spec:** `docs/specs/2026-10-19-carsharing-compare-design.md` §5.1 (proposals), §6 (personal
adjustments), §7 (reports, confirmations, freshness), §11 (languages, accessibility); decisions
1–64 in `docs/DECISIONS.md`; this plan adds 65–73. **Design:** canvas
https://claude.ai/artifact/P2A5aUCR45ZBKhnRRqCW2b, artboards **Discounts** (list + new
discount), **PriceCheck** (ask → fix → thanks sheet) and **Main** ("Discounts · 1" pill on the
result, "Car not listed? Add it" under step 2). Read with the Artifact tool, `read` + `path`.

## Scope

**5b (this plan):** personal discounts (% and €, optional end date, "once"), passes, refuel
credits with a per-trip switch, personal price overrides; price reports, the confirmation
question after **Open <provider>**, new-car proposals and voting on them; per-user choice of the
app that opens a provider; a Settings screen; EN/IT/DE/TR; the accessibility pass; 5a's deferred
minors: location permission asked twice on rotation, Keystore save failure, trailing slash in
the server URL, Cars screen when its provider disappears, short-link expansion errors, release
APK on the phone, TalkBack details.

**Not here:** Play Console, Play Integrity link to the Cloud project, privacy policy (milestone
6). On a real phone every crowd write fails with `integrity_failed`/not registered until then
(decision of 2026-10-20 to keep the roadmap order); everything here is tested with fakes and
MockWebServer, and the app keeps working (overrides apply locally).

## Rulings ledger

Calls made without the owner, each defensible on its merits. DECISIONS.md rows are marked.

| # | Ruling | Why |
|---|---|---|
| R1 | Branch `feat/m5b-personal-and-crowd` from `main` `2f38abe`; this plan is its first commit. | All earlier PRs merged; no open PRs. |
| R2 | The limits on reported values (`MAX_CARDS` 5, label 40, name 60 characters, unlock ≤ €500, per km/min ≤ €5, minimum ≤ €50, included ≤ 20 000) and `is_safe_text` move from `crates/server/src/crowd.rs` to a new core module **`report`**; the server and the app call the same checks. **Decision 65.** | §10.1: the server validates with the code the app calculates with; the app can refuse a value before the user sends it. |
| R3 | A provider may name **other apps** that can open it: `catalogue.json` gets optional `app_name` (default: the provider's name) and `other_apps: [{name, android_package}]`. SIXT share: default **BVG Jelbi**, other **SIXT** (`com.sixt.reservation`, verified on the Play Store in Task 2). The user's choice is stored per provider on the device. `<queries>` lists every package (the 5a test already reads every `android_package` in the file). **Decision 66.** | Decision 48 promised the per-user choice; the catalogue is where provider facts live (§5.1), so a new alternative needs no app update except `<queries>`. |
| R4 | Personal data lives in **one JSON file** `files/personal.json` (atomic write, no backup: the manifest already excludes everything): deals (percent, amount, pass, refuel credit), overrides, chosen apps, the day the question was last asked. Unreadable file → empty, overwritten on the next save. **Decision 67.** | §6 "stored on the device only"; one small file is simpler than preferences for nested lists; corrupt data must not crash the comparison. |
| R5 | Deal rules (pure Kotlin, `personal/Personal.kt`): a deal applies when **enabled**, not past its **end date** (inclusive, device time zone) and, for a **once** amount, not used. A once amount is **marked used when Open <provider> is tapped** for that provider, with an **Undo** snackbar. Percent and pass values are basis points 1–10 000, amounts cents ≥ 1. **Decision 67.** | The canvas has "Once · next trip only" and "until 31 Oct"; tapping Open is the only moment the app knows a trip starts; Undo covers a trip not taken. |
| R6 | The refuel credit is a fourth **kind** in the new-discount form ("Refuel credit"); the result screen shows **"I'll refuel or charge"** only when an active credit exists; the switch is per trip, in memory, off for every new trip. | Decision 47 makes the credit per user per provider; the canvas says "turn it on per trip from the result screen". |
| R7 | Passes edit four values: % off per km, % off per minute, % off packages, and "no unlock fee" (core `Subscription`). Empty = 0; a pass with all zero and no waiver is refused. The monthly fee is not asked for (§6). | Matches the core model and the canvas's "Pass" kind. |
| R8 | ffi `Adjustments` mirrors the core's; invalid entries (percent > 10 000) are **dropped**, not errors, like the core ignores invalid overrides. `quotes`/`options` take adjustments. | The app validates input; a bad stored value must not stop the comparison. |
| R9 | **Reports** (§7.1): "Price wrong? Tell us" under the options opens the price form for the **selected option** (else the cheapest; "any car" → the resolved model). The form edits the offer's **standard card**: unlock, per km, per minute, minimum; other card fields and packages are kept as they are. Saving stores the **override at once** (decision 9) and sends the report. Values equal to the shared price send nothing and store nothing. **Decision 68.** | The canvas's correction shows these numbers; packages and rounding rarely change and would make the form long. |
| R10 | An override is **dropped** when the shared price becomes equal to it (report accepted), or when the user taps **Use the shared price**; rows priced by an override say "your price". | Otherwise a stale personal price would hide future crowd updates forever. |
| R11 | **Writes are best effort**: no outbox, no retry. Unavailable → "Saved for you; couldn't send it now". `offer_locked` → "This price is locked for now" (override kept). Status `current` → override removed ("That's already the price"). After a successful report or proposal the catalogue is reloaded. **Decision 68.** | The override already gives the reporter the right price; a queue would hold integrity tokens that expire in minutes and add a persistence layer for a rare case. |
| R12 | **Confirmation question** (§7.2): armed by **Open <provider>** (never by Check in), asked on the next `onResume` about the opened option's offer, when this session's prices came **from the server** and a server is configured. `Prices.question` (ffi) applies `should_ask`, picks the oldest pending report's values (decision 37) or the current ones, and returns nothing for locked offers. The day it is **shown** is stored (Skip counts as asked). **Decision 69.** | One rule in Rust; built-in or cached prices may be old, so asking about them would confirm stale values. |
| R13 | Answers: **Yes** → vote yes on the pending report, else confirmation yes. **No, fix it** → the price form inside the sheet; **Save correction** = a report (R9). **I don't know the numbers** → vote no on a pending report, nothing on a current price (decision 36). **Skip** → nothing. Proposals offer Yes / No / Skip (no correction). | §7.2; a correction to a proposal would be a second proposal, which decision 46 already handles by replacement. |
| R14 | **Proposals** (§5.1): "Car not listed? Add it" at the end of step 2 opens the price form with a **name** field; rounding and card label are copied from the provider's first standard card. Pending proposals appear to **every** user in step 2 under "Added by users, not confirmed yet", priced for the trip, tickable; they enter comparisons **only when added** and say "not confirmed". Their offer ID is `proposal.<report id>`; when accepted they vanish and the real offer appears (an added proposal is skipped like any unknown offer). **Decision 70.** | Other installs must see a proposal to confirm it; adding it and opening the provider is how they can (R12). Showing it unadded in "cheaper elsewhere" would advertise unconfirmed prices. |
| R15 | The install ID is stored with the bearer token (`id` + newline + token, encrypted as in 5a); a 5a value without an ID reads as missing, so the install registers again. A Keystore failure on save keeps the install **in memory** for the session instead of crashing (5a minor). | Decision 44's request hash needs the ID; 5a only kept the token. |
| R16 | **Settings** screen (gear on Home, replacing the Licences link): Discounts and passes, "Open <provider> with" for providers with more than one app, App language (opens Android's per-app language settings on Android 13+, otherwise explains it follows the phone), Licences. | One place for personal settings; Android 13 per-app languages need no in-app picker. |
| R17 | **Languages**: `values-it`, `values-de`, `values-tr` translated by the implementer with the glossary in Task 14; the owner reviews Italian; German and Turkish get a native-speaker review in the closed test (milestone 6). `generateLocaleConfig` publishes the four locales. A test fails on a missing string, a missing plural, or placeholders that differ from English. **Decision 71.** | §11; a test is the only thing that keeps four files in step. |
| R18 | **Accessibility**: a test helper checks every clickable or toggleable node has a label and a touch area ≥ 48 dp, runs on every screen; every screen is also rendered at **font scale 2.0** on a 360 × 640 dp screen and its main action must be reachable. TalkBack is walked through on the phone by the owner with the checklist in Task 16. **Decision 72.** | §11; Compose's ATF integration is unproven under Robolectric, the helper is deterministic. |
| R19 | 5a minors: the permission request is launched once (`locationRequestLaunched()` clears `askLocation` before rotation can relaunch it); the server URL loses trailing slashes; the Cars screen pops when its provider leaves the catalogue; `not_a_short_link` from expand shows "That link isn't from Google Maps", `invalid_destination` shows "That link doesn't point to a place". | Reported in the 5a final review. |
| R20 | Euro inputs accept dot or comma and up to 2 decimals, optional "€"; percentages up to 2 decimals, 0 < p ≤ 100. Integer maths only. | Same reasoning as 5a R24 (a German keyboard types `0,29`). |
| R21 | `valid until` days use the **device time zone** (`LocalDate.now()`); the question's last-asked day is the ffi's UTC `dayOf`, like the server's confirmation days. | A discount "until 31 Oct" means the user's 31 October; the weekly limit is compared with server days in the core. |

## Global Constraints

- Work on branch `feat/m5b-personal-and-crowd` (cut from `main` at `2f38abe`). Never commit to
  `main`. Push only to `git@github-locadani:locadani/whichride.git`. Never use `gh` for this repo;
  the owner opens PRs and squash-merges.
- Before every commit `git config user.name` prints `locadani` and `git config user.email` prints
  `61902944+locadani@users.noreply.github.com`. No email address in a tracked file.
- gitleaks pre-commit hook active (`git config core.hooksPath .githooks`). No secret is needed.
- Every commit message ends with a blank line and
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Conventional Commits:
  `feat(core): …`, `feat(ffi): …`, `feat(server): …`, `feat(android): …`, `fix(android): …`,
  `docs: …`.
- **Every graphic value comes from `design/tokens.json`** through `WhichrideTheme` /
  `WhichrideTokens`: no literal colour, `dp`, `sp`, `em`, font weight, `Color.X`,
  `RoundedCornerShape(<number>)` or `alpha = <number>` in hand-written Kotlin or XML under
  `android/*/src/` (tests excepted). `cd design && npm test` stays green. New colour pairs go into
  `design/contrast.json` first; a new token needs `npm run build` and the regenerated files.
- Touch targets ≥ 48 dp (`WhichrideTokens.Size.touchTarget`).
- **Privacy (§9):** coordinates only in request bodies; no location, destination or route written
  to disk or logs; no `Log.*` of user data. Personal data stays on the device; only reports,
  votes, confirmations and proposals (offer, values) are sent.
- All user-visible text in `strings.xml` (and its three translations from Task 14 on).
- Rust: `cargo fmt --all --check`, `RUSTFLAGS=-D warnings cargo clippy --workspace --all-targets
  --locked`, `cargo test --workspace --locked` stay green; no floats in the core.
- Android: `export JAVA_HOME=/opt/homebrew/opt/openjdk@21`; from `android/`:
  `./gradlew :app:testDebugUnitTest :designsystem:testDebugUnitTest :app:assembleRelease` green.
  Exact versions only; no new dependency.
- Server tests need the local Postgres: `smoke-pg` container, `TEST_DATABASE_URL` as in
  `crates/server/README.md` (`postgres://postgres@localhost:55432/postgres`).
- Disk: never `cargo clean`, `docker rmi`, `rm -rf` of build folders.
- Doc comments say what and why; no tutorial comments.

## Review Focus

1. **A once-only discount and a trip not taken.** "MILES −€5, once": tapping Open MILES uses
   it and shows Undo; Undo brings it back; opening Bolt must not use it (Task 7:
   `open uses the once discount of that provider only`, `undo restores a used discount`).
2. **"Valid until 31 Oct" at the day boundary.** Applies all of 31 October in the phone's
   time zone, not from 1 November (Task 5: `a discount applies through its last day`).
3. **A correction while offline or locked.** The override applies at once, the message says it
   wasn't sent, nothing retries; `offer_locked` keeps the override (Task 11:
   `a correction without a server still applies to me`, `a locked offer keeps my price`).
4. **The weekly question limit across restarts.** The day shown is stored in the personal
   file, so killing the app does not ask again tomorrow; built-in or cached prices never ask
   (Task 11: `the question is asked at most once a week and never after check in`,
   `no question about built-in prices or without a server`).
5. **A German keyboard in the price form.** `0,29` is 29 cents, `0,295` and `5,01` per km are
   refused with the field marked (Task 4: `euros accept a decimal comma`,
   `euros refuse three decimals and nonsense`; Task 10: `values above the limits are refused before sending`).

---

## File structure

```
crates/core/src/report.rs              NEW  limits on reported values, is_safe_text, check_cards, check_name
crates/core/src/catalogue.rs           MOD  Provider.app_name, Provider.other_apps, ProviderApp, DuplicateApp
crates/core/src/lib.rs                 MOD  pub mod report; re-exports
crates/server/src/crowd.rs             MOD  uses core::report (limits removed here)
crates/server/catalogue.json           MOD  SIXT: app_name "BVG Jelbi", other_apps SIXT
crates/ffi/src/lib.rs                  MOD  thin: module declarations and re-exports
crates/ffi/src/cards.rs                NEW  RateCard, CardKind, Rounding records; cards JSON
crates/ffi/src/adjustments.rs          NEW  Adjustments, Discount, Subscription, RefuelCredit, PriceOverride
crates/ffi/src/prices.rs               NEW  Prices (moved from lib.rs) + adjustments, pending, locks, proposals, question
crates/ffi/src/report.rs               NEW  report_body, proposal_body, report_limits, ReportError
android/app/src/main/AndroidManifest.xml          MOD  <queries> + com.sixt.reservation
android/app/build.gradle.kts                      MOD  generateLocaleConfig
android/app/src/main/res/resources.properties     NEW  unqualifiedResLocale=en
android/app/src/main/res/values{,-it,-de,-tr}/strings.xml
android/app/src/main/kotlin/io/github/locadani/whichride/
  personal/Personal.kt          NEW  Deal types, Personal, rules (active, adjustments, once, overrides)
  personal/PersonalStore.kt     NEW  JSON file store
  personal/DealForm.kt          NEW  the discount form and its validation
  crowd/Crowd.kt                NEW  Crowd interface, Answer, ReportOutcome, ServerCrowd
  crowd/PriceForm.kt            NEW  the price form (report, correction, proposal) and card building
  format/MoneyInput.kt          NEW  parseEuros, parsePercent, eurosInput, percentInput
  server/TokenStore.kt          MOD  Install(id, token)
  server/Installs.kt            MOD  install ID, signed(), in-memory fallback
  server/ServerApi.kt           MOD  writeHash, report, vote, confirm; trailing slash
  server/Routes.kt              MOD  expand error codes
  launch/Launcher.kt            MOD  openAppLanguageSettings
  compare/CompareViewModel.kt   MOD  personal + crowd + question + minors
  compare/UiState.kt, Screen.kt MOD
  AppContainer.kt               MOD  wiring
  ui/SettingsScreen.kt, ui/DealsScreen.kt, ui/DealEditorScreen.kt,
  ui/PriceFormScreen.kt, ui/QuestionSheet.kt                       NEW
  ui/ResultScreen.kt, ui/CarsScreen.kt, ui/HomeScreen.kt, ui/WhichrideNav.kt, ui/Text.kt  MOD
android/app/src/test/kotlin/io/github/locadani/whichride/
  personal/PersonalTest.kt, personal/PersonalStoreTest.kt, personal/DealFormTest.kt,
  crowd/PriceFormTest.kt, crowd/ServerCrowdTest.kt, format/MoneyInputTest.kt,
  compare/PersonalFlowTest.kt, compare/CrowdFlowTest.kt,
  ui/DealsScreenTest.kt, ui/PriceFormScreenTest.kt, ui/QuestionSheetTest.kt, ui/SettingsScreenTest.kt,
  ui/Accessibility.kt, ui/AccessibilityTest.kt, i18n/TranslationsTest.kt                   NEW
docs/DECISIONS.md (65–73), docs/plans/ROADMAP.md, android/README.md
```

`CompareViewModel` grows by the personal and crowd flows; every rule it applies lives in
`personal/`, `crowd/` or the core, so the ViewModel only moves state. Its new tests go into two
new files (`PersonalFlowTest`, `CrowdFlowTest`) sharing `Fakes.kt`.

---
## Part A — Core and FFI

### Task 0: Plan commit

- [ ] **Step 1:** On `feat/m5b-personal-and-crowd` (already created from `main` `2f38abe`), check
  `git config user.name` = `locadani`, then commit this file:
  `git add docs/plans/05b-personal-and-crowd.md && git commit -m "docs: add the milestone 5b plan (personal and crowd)"` (with the Co-Authored-By trailer).

### Task 1: Shared limits on reported values (`core::report`)

**Files:**
- Create: `crates/core/src/report.rs`
- Modify: `crates/core/src/lib.rs`, `crates/server/src/crowd.rs`

**Interfaces:**
- Produces: `carsharing_core::report::{MAX_CARDS, MAX_LABEL_CHARS, MAX_NAME_CHARS, MAX_BASE_FEE,
  MAX_PER_UNIT, MAX_MINIMUM, MAX_INCLUDED, ReportError, is_safe_text, check_cards, check_name}`;
  `check_cards(&OfferId, &[RateCard]) -> Result<(), ReportError>`,
  `check_name(&str) -> Result<&str, ReportError>` (returns the trimmed name).

- [ ] **Step 1: Write the module with its tests (tests first in the file, implementation stubbed
  with `todo!()` bodies)**

```rust
//! Limits on what users may report or propose (spec §5.1, §7.1). The server refuses anything
//! outside them and the app checks the same limits before sending, so both always agree.

use thiserror::Error;

use crate::catalogue::{OfferId, validate_cards};
use crate::money::Cents;
use crate::rate_card::RateCard;

pub const MAX_CARDS: usize = 5;
pub const MAX_LABEL_CHARS: usize = 40;
pub const MAX_NAME_CHARS: usize = 60;
/// Unlock fee of a standard tariff, or the price of a package.
pub const MAX_BASE_FEE: Cents = Cents(50_000);
/// Per km and per minute.
pub const MAX_PER_UNIT: Cents = Cents(500);
pub const MAX_MINIMUM: Cents = Cents(5_000);
pub const MAX_INCLUDED: u32 = 20_000;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ReportError {
    #[error("a report has 1 to {MAX_CARDS} rate cards")]
    CardCount,
    #[error("the rate cards are not valid")]
    InvalidCards,
    #[error("a value is outside what any provider charges")]
    Implausible,
    #[error("the name is empty, too long or has hidden characters")]
    BadName,
}

/// Text shown to other users must not carry control characters or bidirectional formatting
/// (which can make a label read differently from what it is).
pub fn is_safe_text(text: &str) -> bool {
    !text.chars().any(|c| {
        c.is_control()
            || matches!(c, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
    })
}

/// The rate cards of a report or proposal: valid for the core, labelled, and believable.
pub fn check_cards(offer: &OfferId, cards: &[RateCard]) -> Result<(), ReportError> {
    if cards.is_empty() || cards.len() > MAX_CARDS {
        return Err(ReportError::CardCount);
    }
    validate_cards(offer, cards).map_err(|_| ReportError::InvalidCards)?;
    for card in cards {
        let label = card.label.trim();
        let plausible = label == card.label
            && !label.is_empty()
            && label.chars().count() <= MAX_LABEL_CHARS
            && is_safe_text(label)
            && card.base_fee <= MAX_BASE_FEE
            && card.per_km <= MAX_PER_UNIT
            && card.per_minute <= MAX_PER_UNIT
            && card.minimum <= MAX_MINIMUM
            && card.included_km <= MAX_INCLUDED
            && card.included_minutes <= MAX_INCLUDED;
        if !plausible {
            return Err(ReportError::Implausible);
        }
    }
    Ok(())
}

/// The name of a proposed offer, trimmed.
pub fn check_name(name: &str) -> Result<&str, ReportError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS || !is_safe_text(name) {
        return Err(ReportError::BadName);
    }
    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rate_card::fixtures::per_minute_and_km_card;

    fn offer() -> OfferId {
        OfferId("bolt.corsa".into())
    }

    #[test]
    fn safe_text_has_no_control_or_bidi_characters() {
        assert!(is_safe_text("Škoda Enyaq — 10 €/h"));
        for c in ['\u{0}', '\n', '\u{9f}', '\u{200e}', '\u{202c}', '\u{2067}'] {
            assert!(!is_safe_text(&format!("a{c}b")), "{c:?}");
        }
        assert!(
            is_safe_text("a\u{200d}b"),
            "a zero-width joiner is fine (emoji)"
        );
    }

    #[test]
    fn values_at_the_limits_are_accepted() {
        let mut card = per_minute_and_km_card();
        card.base_fee = MAX_BASE_FEE;
        card.per_km = MAX_PER_UNIT;
        card.per_minute = MAX_PER_UNIT;
        card.minimum = MAX_MINIMUM;
        card.included_km = MAX_INCLUDED;
        assert_eq!(check_cards(&offer(), &[card]), Ok(()));
    }

    #[test]
    fn values_above_the_limits_are_implausible() {
        let mut per_km = per_minute_and_km_card();
        per_km.per_km = Cents(MAX_PER_UNIT.0 + 1);
        let mut minimum = per_minute_and_km_card();
        minimum.minimum = Cents(MAX_MINIMUM.0 + 1);
        for card in [per_km, minimum] {
            assert_eq!(
                check_cards(&offer(), &[card]),
                Err(ReportError::Implausible)
            );
        }
    }

    #[test]
    fn labels_must_be_trimmed_short_and_safe() {
        for label in [" Standard", "", &"x".repeat(MAX_LABEL_CHARS + 1), "a\u{202e}b"] {
            let mut card = per_minute_and_km_card();
            card.label = label.into();
            assert_eq!(
                check_cards(&offer(), &[card]),
                Err(ReportError::Implausible),
                "{label:?}"
            );
        }
    }

    #[test]
    fn card_count_and_core_validity_are_checked() {
        assert_eq!(check_cards(&offer(), &[]), Err(ReportError::CardCount));
        let six: Vec<_> = (0..6)
            .map(|i| {
                let mut card = per_minute_and_km_card();
                card.label = format!("Card {i}");
                card
            })
            .collect();
        assert_eq!(check_cards(&offer(), &six), Err(ReportError::CardCount));
        let mut negative = per_minute_and_km_card();
        negative.per_km = Cents(-1);
        assert_eq!(
            check_cards(&offer(), &[negative]),
            Err(ReportError::InvalidCards)
        );
    }

    #[test]
    fn names_are_trimmed_and_checked() {
        assert_eq!(check_name("  Fiat 500e "), Ok("Fiat 500e"));
        assert_eq!(check_name("   "), Err(ReportError::BadName));
        assert_eq!(
            check_name(&"a".repeat(MAX_NAME_CHARS + 1)),
            Err(ReportError::BadName)
        );
        assert_eq!(check_name("a\u{202e}b"), Err(ReportError::BadName));
    }
}
```

  Check that `crate::rate_card::fixtures::per_minute_and_km_card` exists and is `pub` (it is used
  by `adjustments.rs` tests); its label is `"Standard"`.

- [ ] **Step 2: Register the module** — in `crates/core/src/lib.rs` add `pub mod report;` after
  `pub mod rate_card;` and `pub use report::ReportError;` after the `rate_card` re-export.

- [ ] **Step 3: Run** `cargo test -p carsharing-core report --locked` — Expected: 6 passed.

- [ ] **Step 4: Make the server use it.** In `crates/server/src/crowd.rs`:
  - delete the constants `MAX_CARDS`, `MAX_LABEL_CHARS`, `MAX_NAME_CHARS`, `MAX_BASE_FEE`,
    `MAX_PER_UNIT`, `MAX_MINIMUM`, `MAX_INCLUDED` (keep `MAX_ID_SLUG_CHARS`), the function
    `is_safe_text`, and the test `safe_text_has_no_control_or_bidi_characters` (now in the core);
  - replace the body of `check_cards` with
    ```rust
    /// The rate cards of a report: the core's limits (shared with the app).
    pub fn check_cards(offer: &OfferId, cards: &[RateCard]) -> Result<(), ApiError> {
        carsharing_core::report::check_cards(offer, cards).map_err(|_| ApiError::BadRequest)
    }
    ```
  - in `post_report`, replace the proposal arm's name check with
    ```rust
        (None, Some(provider), Some(name)) => {
            let name =
                carsharing_core::report::check_name(&name).map_err(|_| ApiError::BadRequest)?;
            if state.providers.provider(&provider).is_none() {
                return Err(ApiError::NotFound);
            }
            check_cards(&OfferId(offer_id_for(&provider.0, name)), &body.cards)?;
            propose(&state, &install, &provider, name, &body.cards, today).await
        }
    ```
  - remove imports that became unused (`Cents` if unused; the compiler says).

- [ ] **Step 5: Run** `cargo fmt --all && RUSTFLAGS=-D\ warnings cargo clippy --workspace --all-targets --locked && TEST_DATABASE_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace --locked`
  — Expected: all green; `crates/server/tests/crowd.rs` and `proposals.rs` (which send
  implausible values and bad names) still pass unchanged.

- [ ] **Step 6: Commit** — `feat(core): share the limits on reported values with the server`.

### Task 2: Other apps that can open a provider

**Files:**
- Modify: `crates/core/src/catalogue.rs`, `crates/server/catalogue.json`,
  `android/app/src/main/AndroidManifest.xml`, every `Provider { … }` literal (`grep -rn "Provider {" crates`)

**Interfaces:**
- Produces: `Provider.app_name: Option<String>`, `Provider.other_apps: Vec<ProviderApp>`,
  `ProviderApp { name: String, android_package: String }`, `CatalogueError::DuplicateApp(String)`,
  `Provider::apps(&self) -> Vec<ProviderApp>` (default app first).

- [ ] **Step 1: Verify the SIXT package.** Fetch
  `https://play.google.com/store/apps/details?id=com.sixt.reservation` (WebFetch). Expected: the
  page is SIXT's app ("SIXT rent, share, ride & plus" or similar, publisher Sixt). If it is not,
  stop and ask the owner for the package; do not guess another.

- [ ] **Step 2: Write the failing tests** in `catalogue.rs`'s test module:

```rust
    #[test]
    fn apps_list_the_default_first_named_after_the_provider_unless_named() {
        let json = r#"{"providers":[{"id":"sixt","name":"SIXT share","android_package":"com.trafi.whitelabel.bvg",
            "app_name":"BVG Jelbi","other_apps":[{"name":"SIXT","android_package":"com.sixt.reservation"}],
            "offers":[{"id":"sixt.a","name":"A","cards":[]}]}]}"#;
        // Cards are irrelevant here; build the provider directly from JSON without validation.
        let catalogue: Catalogue = serde_json::from_str(json).unwrap();
        let apps = catalogue.providers[0].apps();
        assert_eq!(apps[0], ProviderApp { name: "BVG Jelbi".into(), android_package: "com.trafi.whitelabel.bvg".into() });
        assert_eq!(apps[1].android_package, "com.sixt.reservation");
        let miles = &fixtures::catalogue().providers[0];
        assert_eq!(miles.apps(), vec![ProviderApp { name: "MILES".into(), android_package: "com.driveby.app".into() }]);
    }

    #[test]
    fn an_app_listed_twice_is_refused() {
        let mut catalogue = fixtures::catalogue();
        catalogue.providers[0].other_apps = vec![ProviderApp {
            name: "Again".into(),
            android_package: catalogue.providers[0].android_package.clone(),
        }];
        assert!(matches!(catalogue.validate(), Err(CatalogueError::DuplicateApp(_))));
    }

    #[test]
    fn older_catalogues_without_apps_still_load() {
        let catalogue = fixtures::catalogue();
        let json = serde_json::to_string(&catalogue).unwrap();
        assert!(!json.contains("other_apps"), "empty lists are not written");
        assert_eq!(Catalogue::from_json(&json).unwrap(), catalogue);
    }
```

- [ ] **Step 3: Run** `cargo test -p carsharing-core catalogue` — Expected: compile errors
  (`ProviderApp`, `apps`, `DuplicateApp` unknown).

- [ ] **Step 4: Implement** in `catalogue.rs`:

```rust
/// An app that can open a provider (decision 66), e.g. BVG Jelbi or SIXT for SIXT share.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderApp {
    pub name: String,
    pub android_package: String,
}
```

  In `Provider` after `android_package`:

```rust
    /// The name of the app `android_package` opens, when it isn't the provider's own
    /// (SIXT share opens BVG Jelbi, decision 48).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,
    /// Other apps the user may choose to open this provider with.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other_apps: Vec<ProviderApp>,
```

  `impl Provider`:

```rust
impl Provider {
    /// The default app first, then the others.
    pub fn apps(&self) -> Vec<ProviderApp> {
        let default = ProviderApp {
            name: self.app_name.clone().unwrap_or_else(|| self.name.clone()),
            android_package: self.android_package.clone(),
        };
        std::iter::once(default).chain(self.other_apps.iter().cloned()).collect()
    }
}
```

  `CatalogueError`: `#[error("app `{0}` is listed twice for one provider")] DuplicateApp(String),`.
  In `validate`, after the `NoOffers` check:

```rust
            let mut packages = HashSet::new();
            for app in provider.apps() {
                if !packages.insert(app.android_package.clone()) {
                    return Err(CatalogueError::DuplicateApp(app.android_package));
                }
            }
```

  Add `app_name: None, other_apps: Vec::new(),` to every `Provider { … }` literal the compiler
  reports (core fixtures, server tests).

- [ ] **Step 5: Catalogue and manifest.** In `crates/server/catalogue.json`, provider `sixt`,
  after `"android_package": "com.trafi.whitelabel.bvg",` add
  `"app_name": "BVG Jelbi", "other_apps": [{"name": "SIXT", "android_package": "com.sixt.reservation"}],`
  (formatted like the file). In `AndroidManifest.xml` `<queries>` add
  `<package android:name="com.sixt.reservation" />` and extend the comment: "and the other apps
  a user may choose for a provider (decision 66)".

- [ ] **Step 6: Run** `cargo test --workspace --locked` (with `TEST_DATABASE_URL`) and
  `cd android && ./gradlew :app:testDebugUnitTest --tests '*ManifestQueriesTest*'` — Expected:
  all pass (the manifest test reads every `android_package`, including `other_apps`).

- [ ] **Step 7: Commit** — `feat(core): providers can name other apps that open them (SIXT app for SIXT share)`.

### Task 3: FFI for adjustments, rate cards, crowd data and report bodies

`crates/ffi/src/lib.rs` is split: rate cards, adjustments, prices and reports get their own
files; `lib.rs` keeps links, coordinates, `Trip`, `DataError` and the module list. Kotlin call
sites change in the same task so the Android build stays green.

**Files:**
- Create: `crates/ffi/src/cards.rs`, `crates/ffi/src/adjustments.rs`, `crates/ffi/src/prices.rs`,
  `crates/ffi/src/report.rs`
- Modify: `crates/ffi/src/lib.rs`; Kotlin: `compare/CompareViewModel.kt` (`reprice`),
  test fixtures `ui/ResultScreenTest.kt` (`quote`, `PROVIDERS`), every other `ProviderInfo(` /
  `Quote(` literal in `android/app/src/test` (`grep -rn "ProviderInfo(\|Quote(" android/app/src/test`)

**Interfaces (Kotlin names as UniFFI generates them):**
- Consumes: `carsharing_core::report` (Task 1), `Provider::apps` (Task 2).
- Produces:
  - `RateCard(label, kind: CardKind, baseFee: Long, perKm: Long, perMinute: Long, includedKm: UInt, includedMinutes: UInt, minimum: Long, kmRounding: Rounding, minuteRounding: Rounding)`;
    `CardKind.STANDARD|PACKAGE`; `Rounding.STARTED_UNIT|EXACT`;
    `cardsToJson(List<RateCard>): String`; `cardsFromJson(String): List<RateCard>` (throws `DataException`).
  - `Adjustments(overrides: List<PriceOverride>, subscriptions: List<Subscription>, discounts: List<Discount>, refuelCredits: List<RefuelCredit>, refuel: Boolean)`;
    `PriceOverride(offerId, cards)`; `Subscription(providerId, distanceOff: UShort, timeOff: UShort, waiveBaseFee: Boolean, packageOff: UShort)` (basis points);
    `Discount.Percent(providerId, off: UShort)`, `Discount.Amount(providerId, cents: Long)`; `RefuelCredit(providerId, cents: Long)`.
  - `ProviderInfo(id, name, androidPackage, offers: List<OfferInfo>, apps: List<AppInfo>)`; `AppInfo(name, androidPackage)`.
  - `Quote(providerId, offerId, offerName, cardLabel, totalCents, overridden, confirmedDaysAgo: UInt?, proposed: Boolean)`.
  - `Prices.quotes(trip, adjustments, todayUnixSeconds)`, `Prices.options(trip, choices, adjustments, todayUnixSeconds)`,
    `Prices.cards(offerId): List<RateCard>`, `Prices.isLocked(offerId): Boolean`,
    `Prices.question(offerId, todayUnixSeconds, lastAskedDay: Int?): Question?`;
    `Question(offerId, reportId: String?, cards: List<RateCard>)`; `dayOf(unixSeconds): Int`;
    `proposalPrefix(): String` = `"proposal."` (Kotlin repeats it as a constant in Task 10, checked by a test).
  - `reportBody(offerId, cards): String`, `proposalBody(providerId, name, cards): String` (throw `ReportException.CardCount|InvalidCards|Implausible|BadName`);
    `reportLimits(): ReportLimits(maxBaseFee: Long, maxPerUnit: Long, maxMinimum: Long, maxNameChars: UInt)`.

- [ ] **Step 1: `cards.rs`**

```rust
//! Rate cards across the boundary: the app shows and edits them, and stores its overrides as the
//! core's JSON.

use carsharing_core as cc;

use crate::{DataError, invalid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CardKind {
    Standard,
    Package,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Rounding {
    StartedUnit,
    Exact,
}

/// One tariff of an offer; money in cents (see `carsharing_core::RateCard`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RateCard {
    pub label: String,
    pub kind: CardKind,
    pub base_fee: i64,
    pub per_km: i64,
    pub per_minute: i64,
    pub included_km: u32,
    pub included_minutes: u32,
    pub minimum: i64,
    pub km_rounding: Rounding,
    pub minute_rounding: Rounding,
}

fn rounding_from(r: cc::Rounding) -> Rounding {
    match r {
        cc::Rounding::StartedUnit => Rounding::StartedUnit,
        cc::Rounding::Exact => Rounding::Exact,
    }
}

fn rounding_to(r: Rounding) -> cc::Rounding {
    match r {
        Rounding::StartedUnit => cc::Rounding::StartedUnit,
        Rounding::Exact => cc::Rounding::Exact,
    }
}

impl From<&cc::RateCard> for RateCard {
    fn from(c: &cc::RateCard) -> Self {
        RateCard {
            label: c.label.clone(),
            kind: match c.kind {
                cc::CardKind::Standard => CardKind::Standard,
                cc::CardKind::Package => CardKind::Package,
            },
            base_fee: c.base_fee.0,
            per_km: c.per_km.0,
            per_minute: c.per_minute.0,
            included_km: c.included_km,
            included_minutes: c.included_minutes,
            minimum: c.minimum.0,
            km_rounding: rounding_from(c.km_rounding),
            minute_rounding: rounding_from(c.minute_rounding),
        }
    }
}

impl From<&RateCard> for cc::RateCard {
    fn from(c: &RateCard) -> Self {
        cc::RateCard {
            label: c.label.clone(),
            kind: match c.kind {
                CardKind::Standard => cc::CardKind::Standard,
                CardKind::Package => cc::CardKind::Package,
            },
            base_fee: cc::Cents(c.base_fee),
            per_km: cc::Cents(c.per_km),
            per_minute: cc::Cents(c.per_minute),
            included_km: c.included_km,
            included_minutes: c.included_minutes,
            minimum: cc::Cents(c.minimum),
            km_rounding: rounding_to(c.km_rounding),
            minute_rounding: rounding_to(c.minute_rounding),
        }
    }
}

pub(crate) fn to_core(cards: &[RateCard]) -> Vec<cc::RateCard> {
    cards.iter().map(Into::into).collect()
}

pub(crate) fn from_core(cards: &[cc::RateCard]) -> Vec<RateCard> {
    cards.iter().map(Into::into).collect()
}

/// The core's JSON of these cards (how the app stores its overrides).
#[uniffi::export]
pub fn cards_to_json(cards: Vec<RateCard>) -> String {
    serde_json::to_string(&to_core(&cards)).expect("rate cards always serialise")
}

#[uniffi::export]
pub fn cards_from_json(json: String) -> Result<Vec<RateCard>, DataError> {
    let cards: Vec<cc::RateCard> = serde_json::from_str(&json).map_err(invalid)?;
    Ok(from_core(&cards))
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn standard(per_km: i64) -> RateCard {
        RateCard {
            label: "Standard".into(),
            kind: CardKind::Standard,
            base_fee: 100,
            per_km,
            per_minute: 0,
            included_km: 0,
            included_minutes: 0,
            minimum: 0,
            km_rounding: Rounding::StartedUnit,
            minute_rounding: Rounding::StartedUnit,
        }
    }

    #[test]
    fn cards_survive_a_json_round_trip_in_the_cores_format() {
        let cards = vec![standard(99)];
        let json = cards_to_json(cards.clone());
        assert!(json.contains(r#""km_rounding":"started_unit""#), "{json}");
        assert_eq!(cards_from_json(json), Ok(cards));
        assert!(cards_from_json("[{}]".into()).is_err());
    }
}
```

- [ ] **Step 2: `adjustments.rs`**

```rust
//! The user's own price changes (spec §6) across the boundary. Entries the core can't use (a
//! percentage above 100 %) are dropped instead of failing the comparison (plan R8).

use carsharing_core as cc;

use crate::cards::{RateCard, to_core};

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PriceOverride {
    pub offer_id: String,
    pub cards: Vec<RateCard>,
}

/// A pass; percentages in basis points (1 500 = 15 %).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Subscription {
    pub provider_id: String,
    pub distance_off: u16,
    pub time_off: u16,
    pub waive_base_fee: bool,
    pub package_off: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum Discount {
    /// `off` in basis points.
    Percent { provider_id: String, off: u16 },
    Amount { provider_id: String, cents: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct RefuelCredit {
    pub provider_id: String,
    pub cents: i64,
}

/// What applies to this trip; the app passes only deals valid today (plan R5).
#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct Adjustments {
    pub overrides: Vec<PriceOverride>,
    pub subscriptions: Vec<Subscription>,
    pub discounts: Vec<Discount>,
    pub refuel_credits: Vec<RefuelCredit>,
    pub refuel: bool,
}

fn percent(basis_points: u16) -> Option<cc::Percent> {
    cc::Percent::from_basis_points(basis_points).ok()
}

fn provider(id: &str) -> cc::ProviderId {
    cc::ProviderId(id.to_owned())
}

impl From<&Adjustments> for cc::Adjustments {
    fn from(a: &Adjustments) -> Self {
        cc::Adjustments {
            overrides: a
                .overrides
                .iter()
                .map(|o| cc::PriceOverride {
                    offer: cc::OfferId(o.offer_id.clone()),
                    cards: to_core(&o.cards),
                })
                .collect(),
            subscriptions: a
                .subscriptions
                .iter()
                .filter_map(|s| {
                    Some(cc::Subscription {
                        provider: provider(&s.provider_id),
                        distance_off: percent(s.distance_off)?,
                        time_off: percent(s.time_off)?,
                        waive_base_fee: s.waive_base_fee,
                        package_off: percent(s.package_off)?,
                    })
                })
                .collect(),
            discounts: a
                .discounts
                .iter()
                .filter_map(|d| match d {
                    Discount::Percent { provider_id, off } => Some(cc::Discount::Percent {
                        provider: provider(provider_id),
                        off: percent(*off)?,
                    }),
                    Discount::Amount { provider_id, cents } => Some(cc::Discount::Amount {
                        provider: provider(provider_id),
                        amount: cc::Cents(*cents),
                    }),
                })
                .collect(),
            refuel_credits: a
                .refuel_credits
                .iter()
                .map(|c| cc::adjustments::RefuelCredit {
                    provider: provider(&c.provider_id),
                    amount: cc::Cents(c.cents),
                })
                .collect(),
            refuel: a.refuel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentages_above_100_are_dropped() {
        let adjustments = Adjustments {
            discounts: vec![
                Discount::Percent { provider_id: "bolt".into(), off: 10_001 },
                Discount::Percent { provider_id: "bolt".into(), off: 2_000 },
                Discount::Amount { provider_id: "miles".into(), cents: 500 },
            ],
            subscriptions: vec![Subscription {
                provider_id: "miles".into(),
                distance_off: 1_000,
                time_off: 20_000,
                waive_base_fee: true,
                package_off: 0,
            }],
            ..Adjustments::default()
        };
        let core: cc::Adjustments = (&adjustments).into();
        assert_eq!(core.discounts.len(), 2);
        assert!(core.subscriptions.is_empty());
    }
}
```

- [ ] **Step 3: `report.rs`**

```rust
//! Bodies of `POST /v1/reports` (a price report or a new-offer proposal), checked with the
//! core's limits before the app sends them. The app sends these exact bytes and signs them
//! (decision 44), so they are built here, once.

use carsharing_core as cc;

use crate::cards::{RateCard, to_core};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum ReportError {
    #[error("a report has 1 to 5 rate cards")]
    CardCount,
    #[error("the rate cards are not valid")]
    InvalidCards,
    #[error("a value is outside what any provider charges")]
    Implausible,
    #[error("the name is empty, too long or has hidden characters")]
    BadName,
}

impl From<cc::ReportError> for ReportError {
    fn from(e: cc::ReportError) -> Self {
        match e {
            cc::ReportError::CardCount => ReportError::CardCount,
            cc::ReportError::InvalidCards => ReportError::InvalidCards,
            cc::ReportError::Implausible => ReportError::Implausible,
            cc::ReportError::BadName => ReportError::BadName,
        }
    }
}

/// The largest values a user may enter, in cents (`carsharing_core::report`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ReportLimits {
    pub max_base_fee: i64,
    pub max_per_unit: i64,
    pub max_minimum: i64,
    pub max_name_chars: u32,
}

#[uniffi::export]
pub fn report_limits() -> ReportLimits {
    ReportLimits {
        max_base_fee: cc::report::MAX_BASE_FEE.0,
        max_per_unit: cc::report::MAX_PER_UNIT.0,
        max_minimum: cc::report::MAX_MINIMUM.0,
        max_name_chars: cc::report::MAX_NAME_CHARS as u32,
    }
}

#[uniffi::export]
pub fn report_body(offer_id: String, cards: Vec<RateCard>) -> Result<String, ReportError> {
    let cards = to_core(&cards);
    cc::report::check_cards(&cc::OfferId(offer_id.clone()), &cards)?;
    Ok(serde_json::json!({ "offer": offer_id, "cards": cards }).to_string())
}

#[uniffi::export]
pub fn proposal_body(
    provider_id: String,
    name: String,
    cards: Vec<RateCard>,
) -> Result<String, ReportError> {
    let name = cc::report::check_name(&name)?;
    let cards = to_core(&cards);
    cc::report::check_cards(&cc::OfferId(format!("{provider_id}.proposal")), &cards)?;
    Ok(serde_json::json!({ "provider": provider_id, "name": name, "cards": cards }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{CardKind, Rounding};

    fn card(per_km: i64) -> RateCard {
        RateCard {
            label: "Standard".into(),
            kind: CardKind::Standard,
            base_fee: 100,
            per_km,
            per_minute: 0,
            included_km: 0,
            included_minutes: 0,
            minimum: 0,
            km_rounding: Rounding::StartedUnit,
            minute_rounding: Rounding::StartedUnit,
        }
    }

    #[test]
    fn a_report_body_is_what_the_server_reads() {
        let body = report_body("miles.s".into(), vec![card(89)]).unwrap();
        let value: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(value["offer"], "miles.s");
        assert_eq!(value["cards"][0]["per_km"], 89);
        assert_eq!(value["cards"][0]["kind"], "standard");
    }

    #[test]
    fn implausible_values_are_refused_before_sending() {
        assert_eq!(
            report_body("miles.s".into(), vec![card(501)]),
            Err(ReportError::Implausible)
        );
        assert_eq!(report_body("miles.s".into(), vec![]), Err(ReportError::CardCount));
    }

    #[test]
    fn a_proposal_body_has_the_trimmed_name() {
        let body = proposal_body("bolt".into(), "  Fiat 500e ".into(), vec![card(29)]).unwrap();
        let value: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(value["provider"], "bolt");
        assert_eq!(value["name"], "Fiat 500e");
        assert_eq!(
            proposal_body("bolt".into(), " ".into(), vec![card(29)]),
            Err(ReportError::BadName)
        );
    }

    #[test]
    fn limits_are_the_cores() {
        assert_eq!(report_limits().max_per_unit, 500);
        assert_eq!(report_limits().max_name_chars, 60);
    }
}
```

- [ ] **Step 4: `prices.rs`.** Move `OfferInfo`, `ProviderInfo`, `Quote`, `Choice`,
  `RankedOption`, `Options`, `ServerCatalogue`, `ServerOfferStatus`, `Prices` and the tests about
  them (`the_seed_catalogue_loads_with_every_v1_provider`, `seed_quotes_have_no_confirmation_date`,
  `server_quotes_say_how_long_ago_they_were_confirmed`, `options_cross_the_boundary`,
  `invalid_catalogues_are_errors_not_panics`, with `SEED`, `TODAY`, `TRIP`, `server_response`)
  out of `lib.rs` into `prices.rs`, then make it read as follows (the `Choice` conversions and
  `RankedOption`/`Options` are unchanged from `lib.rs`):

```rust
//! The catalogue the app compares with: prices, freshness, pending reports, locks and proposals
//! from the server (or the seed in the APK), priced with the user's adjustments.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use carsharing_core as cc;
use serde::Deserialize;

use crate::adjustments::Adjustments;
use crate::cards::{RateCard, from_core};
use crate::{DataError, Trip, invalid};

/// Offer IDs of pending proposals: this prefix and the report ID (plan R14).
pub const PROPOSAL_PREFIX: &str = "proposal.";

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct OfferInfo {
    pub id: String,
    pub name: String,
}

/// An app that can open a provider; the provider's default first (decision 66).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct AppInfo {
    pub name: String,
    pub android_package: String,
}

/// A provider and its accepted offers (pending proposals are only in `quotes`).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub android_package: String,
    pub offers: Vec<OfferInfo>,
    pub apps: Vec<AppInfo>,
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
    /// A car proposed by a user and not confirmed yet (§5.1).
    pub proposed: bool,
}

/// The §7.2 question: the values to confirm, and the pending report they belong to (if any).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Question {
    pub offer_id: String,
    pub report_id: Option<String>,
    pub cards: Vec<RateCard>,
}

// Choice, its two From impls, RankedOption and Options: unchanged, moved from lib.rs.

/// The fields of `GET /v1/catalogue` the app uses.
#[derive(Deserialize)]
struct ServerCatalogue {
    catalogue: cc::Catalogue,
    offers: Vec<ServerOfferStatus>,
    #[serde(default)]
    proposals: Vec<ServerProposal>,
}

#[derive(Deserialize)]
struct ServerOfferStatus {
    offer: cc::OfferId,
    confirmed_on: Option<cc::Day>,
    #[serde(default)]
    locked: bool,
    #[serde(default)]
    pending: Option<ServerPending>,
}

#[derive(Deserialize)]
struct ServerPending {
    report: String,
    cards: Vec<cc::RateCard>,
}

#[derive(Deserialize)]
struct ServerProposal {
    report: String,
    provider: cc::ProviderId,
    name: String,
    cards: Vec<cc::RateCard>,
}

struct Pending {
    report: String,
    cards: Vec<cc::RateCard>,
}

/// A report ID goes into a URL path, so only lowercase hyphenated UUIDs are accepted.
fn is_report_id(id: &str) -> bool {
    id.len() == 36
        && id.chars().enumerate().all(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                c == '-'
            } else {
                c.is_ascii_digit() || ('a'..='f').contains(&c)
            }
        })
}

fn is_proposal(offer: &cc::OfferId) -> bool {
    offer.0.starts_with(PROPOSAL_PREFIX)
}

/// `PROPOSAL_PREFIX` for Kotlin (UniFFI does not export constants); the app checks its copy against it.
#[uniffi::export]
pub fn proposal_prefix() -> String {
    PROPOSAL_PREFIX.into()
}

/// The day number (days since 1970-01-01 UTC) the server and `question` use.
#[uniffi::export]
pub fn day_of(unix_seconds: i64) -> i32 {
    cc::Day::from_unix_seconds(unix_seconds).0
}

/// A validated catalogue with everything the crowd said about it.
#[derive(uniffi::Object)]
pub struct Prices {
    /// Accepted offers only.
    catalogue: cc::Catalogue,
    /// `catalogue` plus pending proposals as offers.
    with_proposals: cc::Catalogue,
    confirmed_on: HashMap<cc::OfferId, cc::Day>,
    pending: HashMap<cc::OfferId, Pending>,
    locked: HashSet<cc::OfferId>,
}

#[uniffi::export]
impl Prices {
    /// The body of the server's `GET /v1/catalogue`. Pending reports and proposals that fail the
    /// core's checks are left out, not errors.
    #[uniffi::constructor]
    pub fn from_server_response(json: String) -> Result<Arc<Prices>, DataError> {
        let response: ServerCatalogue = serde_json::from_str(&json).map_err(invalid)?;
        response.catalogue.validate().map_err(invalid)?;
        let mut confirmed_on = HashMap::new();
        let mut pending = HashMap::new();
        let mut locked = HashSet::new();
        for status in response.offers {
            if let Some(day) = status.confirmed_on {
                confirmed_on.insert(status.offer.clone(), day);
            }
            if status.locked {
                locked.insert(status.offer.clone());
            }
            if let Some(report) = status.pending
                && is_report_id(&report.report)
                && cc::catalogue::validate_cards(&status.offer, &report.cards).is_ok()
            {
                pending.insert(
                    status.offer,
                    Pending {
                        report: report.report,
                        cards: report.cards,
                    },
                );
            }
        }
        let mut with_proposals = response.catalogue.clone();
        for proposal in response.proposals {
            let id = cc::OfferId(format!("{PROPOSAL_PREFIX}{}", proposal.report));
            let usable = is_report_id(&proposal.report)
                && cc::catalogue::validate_cards(&id, &proposal.cards).is_ok();
            if usable
                && let Some(provider) = with_proposals
                    .providers
                    .iter_mut()
                    .find(|p| p.id == proposal.provider)
            {
                provider.offers.push(cc::Offer {
                    id,
                    name: proposal.name,
                    cards: proposal.cards,
                });
            }
        }
        Ok(Arc::new(Prices {
            catalogue: response.catalogue,
            with_proposals,
            confirmed_on,
            pending,
            locked,
        }))
    }

    /// The seed catalogue shipped in the APK (`crates/server/catalogue.json`); nothing is confirmed.
    #[uniffi::constructor]
    pub fn from_seed(json: String) -> Result<Arc<Prices>, DataError> {
        let catalogue = cc::Catalogue::from_json(&json).map_err(invalid)?;
        Ok(Arc::new(Prices {
            with_proposals: catalogue.clone(),
            catalogue,
            confirmed_on: HashMap::new(),
            pending: HashMap::new(),
            locked: HashSet::new(),
        }))
    }

    pub fn providers(&self) -> Vec<ProviderInfo> {
        self.catalogue
            .providers
            .iter()
            .map(|p| ProviderInfo {
                id: p.id.0.clone(),
                name: p.name.clone(),
                android_package: p.android_package.clone(),
                offers: p
                    .offers
                    .iter()
                    .map(|o| OfferInfo {
                        id: o.id.0.clone(),
                        name: o.name.clone(),
                    })
                    .collect(),
                apps: p
                    .apps()
                    .into_iter()
                    .map(|a| AppInfo {
                        name: a.name,
                        android_package: a.android_package,
                    })
                    .collect(),
            })
            .collect()
    }

    /// Every offer's best price with the user's adjustments, cheapest first, pending proposals
    /// included (marked `proposed`).
    pub fn quotes(&self, trip: Trip, adjustments: Adjustments, today_unix_seconds: i64) -> Vec<Quote> {
        let today = cc::Day::from_unix_seconds(today_unix_seconds);
        self.core_quotes(trip, &adjustments)
            .iter()
            .map(|q| self.quote(q, today))
            .collect()
    }

    /// The user's added cars ranked, and the cheaper ones they did not add (spec §4.1). A
    /// proposal counts only when the user added it (plan R14).
    pub fn options(
        &self,
        trip: Trip,
        choices: Vec<Choice>,
        adjustments: Adjustments,
        today_unix_seconds: i64,
    ) -> Options {
        let today = cc::Day::from_unix_seconds(today_unix_seconds);
        let added: HashSet<&str> = choices
            .iter()
            .filter_map(|c| match c {
                Choice::Offer { offer_id } => Some(offer_id.as_str()),
                Choice::AnyOf { .. } => None,
            })
            .collect();
        let quotes: Vec<cc::Quote> = self
            .core_quotes(trip, &adjustments)
            .into_iter()
            .filter(|q| !is_proposal(&q.offer) || added.contains(q.offer.0.as_str()))
            .collect();
        let choices: Vec<cc::Choice> = choices.iter().map(Into::into).collect();
        let options = cc::options(&quotes, &choices);
        Options {
            ranked: options
                .ranked
                .into_iter()
                .map(|o| RankedOption {
                    choice: o.choice.into(),
                    quote: self.quote(&o.quote, today),
                })
                .collect(),
            cheaper_not_added: options
                .cheaper_not_added
                .iter()
                .map(|q| self.quote(q, today))
                .collect(),
        }
    }

    /// The shared rate cards of an offer or proposal; empty if unknown.
    pub fn cards(&self, offer_id: String) -> Vec<RateCard> {
        self.offer(&cc::OfferId(offer_id))
            .map(|o| from_core(&o.cards))
            .unwrap_or_default()
    }

    /// The owner locked or hid this offer: no reports, no questions.
    pub fn is_locked(&self, offer_id: String) -> bool {
        self.locked.contains(&cc::OfferId(offer_id))
    }

    /// The §7.2 question for an offer the user just opened, if one is due (plan R12): about a
    /// proposal itself, else about the oldest pending report (decision 37), else about the
    /// current price when it was not confirmed recently. `last_asked_day` is a `day_of` value.
    pub fn question(
        &self,
        offer_id: String,
        today_unix_seconds: i64,
        last_asked_day: Option<i32>,
    ) -> Option<Question> {
        let offer = cc::OfferId(offer_id);
        if self.locked.contains(&offer) {
            return None;
        }
        let current = self.offer(&offer)?;
        let (report_id, cards) = if let Some(report) = offer.0.strip_prefix(PROPOSAL_PREFIX) {
            (Some(report.to_owned()), &current.cards)
        } else if let Some(pending) = self.pending.get(&offer) {
            (Some(pending.report.clone()), &pending.cards)
        } else {
            (None, &current.cards)
        };
        let ask = cc::should_ask(
            cc::Day::from_unix_seconds(today_unix_seconds),
            last_asked_day.map(cc::Day),
            report_id.is_some(),
            self.confirmed_on.get(&offer).copied(),
        );
        ask.then(|| Question {
            offer_id: offer.0.clone(),
            report_id,
            cards: from_core(cards),
        })
    }
}

impl Prices {
    fn offer(&self, id: &cc::OfferId) -> Option<&cc::Offer> {
        self.with_proposals
            .providers
            .iter()
            .flat_map(|p| &p.offers)
            .find(|o| &o.id == id)
    }

    fn core_quotes(&self, trip: Trip, adjustments: &Adjustments) -> Vec<cc::Quote> {
        cc::compare(&self.with_proposals, &trip.into(), &adjustments.into())
    }

    fn quote(&self, q: &cc::Quote, today: cc::Day) -> Quote {
        Quote {
            provider_id: q.provider.0.clone(),
            offer_id: q.offer.0.clone(),
            offer_name: q.offer_name.clone(),
            card_label: q.card_label.clone(),
            total_cents: q.total.0,
            overridden: q.overridden,
            confirmed_days_ago: cc::confirmed_days_ago(
                today,
                self.confirmed_on.get(&q.offer).copied(),
            ),
            proposed: is_proposal(&q.offer),
        }
    }
}
```

  Test module of `prices.rs`: the moved tests call `quotes(TRIP, Adjustments::default(), TODAY)`
  and `options(TRIP, …, Adjustments::default(), TODAY)`; `the_seed_catalogue_loads_with_every_v1_provider`
  also asserts `prices.providers()[2].apps.iter().map(|a| a.name.as_str()).collect::<Vec<_>>() == ["BVG Jelbi", "SIXT"]`.
  Replace `server_response()` and add the new tests:

```rust
    const PENDING: &str = "11111111-2222-4333-8444-555555555555";
    const PROPOSAL: &str = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";

    fn card_json(per_km: i64) -> serde_json::Value {
        serde_json::json!({"label":"Standard","kind":"standard","base_fee":100,"per_km":per_km,
            "per_minute":0,"km_rounding":"started_unit","minute_rounding":"started_unit"})
    }

    fn server_response() -> String {
        let catalogue: serde_json::Value = serde_json::from_str(SEED).unwrap();
        serde_json::json!({
            "catalogue": catalogue,
            "offers": [
                {"offer": "miles.s", "confirmed_on": 20_744, "locked": false,
                 "pending": {"report": PENDING, "cards": [card_json(89)]}},
                {"offer": "miles.m", "confirmed_on": 20_746, "locked": false, "pending": null},
                {"offer": "bolt.vw-polo", "confirmed_on": null, "locked": true, "pending": null}
            ],
            "proposals": [
                {"report": PROPOSAL, "provider": "bolt", "name": "Fiat 500e", "cards": [card_json(1)]},
                {"report": "../../etc", "provider": "bolt", "name": "Bad id", "cards": [card_json(1)]},
                {"report": PROPOSAL, "provider": "nobody", "name": "Unknown provider", "cards": [card_json(1)]}
            ]
        })
        .to_string()
    }

    fn server() -> Arc<Prices> {
        Prices::from_server_response(server_response()).unwrap()
    }

    #[test]
    fn proposals_are_quoted_but_count_only_when_added() {
        let prices = server();
        let proposal_id = format!("{PROPOSAL_PREFIX}{PROPOSAL}");
        let quotes = prices.quotes(TRIP, Adjustments::default(), TODAY);
        let proposed: Vec<_> = quotes.iter().filter(|q| q.proposed).collect();
        assert_eq!(proposed.len(), 1, "bad report IDs and unknown providers are left out");
        assert_eq!(proposed[0].offer_id, proposal_id);
        assert!(prices.providers()[1].offers.iter().all(|o| !o.id.starts_with(PROPOSAL_PREFIX)));

        let any_bolt = vec![Choice::AnyOf { provider_id: "bolt".into() }];
        let options = prices.options(TRIP, any_bolt, Adjustments::default(), TODAY);
        assert!(!options.ranked[0].quote.proposed, "any car never resolves to a proposal");
        assert!(options.cheaper_not_added.iter().all(|q| !q.proposed));

        let added = vec![Choice::Offer { offer_id: proposal_id.clone() }];
        let options = prices.options(TRIP, added, Adjustments::default(), TODAY);
        assert_eq!(options.ranked[0].quote.offer_id, proposal_id);
    }

    #[test]
    fn adjustments_change_the_quotes() {
        let prices = Prices::from_seed(SEED.into()).unwrap();
        let plain = prices.quotes(TRIP, Adjustments::default(), TODAY);
        let miles_s = |quotes: &[Quote]| quotes.iter().find(|q| q.offer_id == "miles.s").unwrap().clone();
        let half = Adjustments {
            discounts: vec![crate::adjustments::Discount::Percent { provider_id: "miles".into(), off: 5_000 }],
            ..Adjustments::default()
        };
        let discounted = miles_s(&prices.quotes(TRIP, half, TODAY));
        assert_eq!(discounted.total_cents, (miles_s(&plain).total_cents + 1) / 2);
        let mine = Adjustments {
            overrides: vec![crate::adjustments::PriceOverride {
                offer_id: "miles.s".into(),
                cards: crate::cards::cards_from_json(format!("[{}]", card_json(10))).unwrap(),
            }],
            ..Adjustments::default()
        };
        assert!(miles_s(&prices.quotes(TRIP, mine, TODAY)).overridden);
    }

    #[test]
    fn the_question_is_about_the_pending_report_first() {
        let prices = server();
        let question = prices.question("miles.s".into(), TODAY, None).unwrap();
        assert_eq!(question.report_id.as_deref(), Some(PENDING));
        assert_eq!(question.cards[0].per_km, 89);
    }

    #[test]
    fn no_question_about_locked_unknown_or_recently_confirmed_offers() {
        let prices = server();
        assert_eq!(prices.question("bolt.vw-polo".into(), TODAY, None), None);
        assert_eq!(prices.question("nothing.here".into(), TODAY, None), None);
        assert_eq!(prices.question("miles.m".into(), TODAY, None), None, "confirmed today");
        assert!(prices.is_locked("bolt.vw-polo".into()));
    }

    #[test]
    fn the_question_waits_a_week_after_the_last_one() {
        let prices = server();
        let today = day_of(TODAY);
        assert_eq!(prices.question("miles.s".into(), TODAY, Some(today - 6)), None);
        assert!(prices.question("miles.s".into(), TODAY, Some(today - 7)).is_some());
    }

    #[test]
    fn a_proposal_question_votes_on_the_proposal() {
        let prices = server();
        let question = prices
            .question(format!("{PROPOSAL_PREFIX}{PROPOSAL}"), TODAY, None)
            .unwrap();
        assert_eq!(question.report_id.as_deref(), Some(PROPOSAL));
        assert_eq!(question.cards[0].per_km, 1);
    }

    #[test]
    fn cards_of_unknown_offers_are_empty() {
        let prices = server();
        assert_eq!(prices.cards("miles.s".into())[0].per_km, 99);
        assert!(prices.cards("nope".into()).is_empty());
    }
```

  (`miles.s` costs 99 cents per km in the seed; check `crates/server/catalogue.json` and adjust
  the expected `99` if the owner's seed differs. `miles.m` was confirmed on day 20 746 = `TODAY`.)

- [ ] **Step 5: `lib.rs`** keeps `setup_scaffolding!`, `Coordinate`, `Destination`, `LinkError`,
  `DataError`, `invalid` (now `pub(crate)`), `parse_destination`, `shared_label`,
  `destination_from_json`, `Trip` and their tests, and adds:

```rust
mod adjustments;
mod cards;
mod prices;
mod report;

pub use adjustments::{Adjustments, Discount, PriceOverride, RefuelCredit, Subscription};
pub use cards::{CardKind, RateCard, Rounding, cards_from_json, cards_to_json};
pub use prices::{
    AppInfo, Choice, OfferInfo, Options, PROPOSAL_PREFIX, Prices, ProviderInfo, Question, Quote,
    RankedOption, day_of, proposal_prefix,
};
pub use report::{ReportError, ReportLimits, proposal_body, report_body, report_limits};
```

  Update the crate doc comment's first line to "The core's functions for the Android app, through
  UniFFI." (unchanged) and add "Personal adjustments, crowd data and report bodies are in their
  own modules."

- [ ] **Step 6: Run** `cargo fmt --all && RUSTFLAGS=-D\ warnings cargo clippy -p whichride-ffi --all-targets --locked && cargo test -p whichride-ffi --locked`
  — Expected: all pass (old and new tests).

- [ ] **Step 7: Kotlin call sites.** In `CompareViewModel.reprice()` pass
  `NO_ADJUSTMENTS` to both calls and add at the bottom of the file
  `private val NO_ADJUSTMENTS = Adjustments(emptyList(), emptyList(), emptyList(), emptyList(), false)`
  (Task 7 replaces it with the user's). In test fixtures: `quote(...)` in `ResultScreenTest.kt`
  becomes `Quote(provider, offer, name, "Standard", cents, false, days, false)`; every
  `ProviderInfo(id, name, package, offers)` gets a fifth argument
  `listOf(AppInfo(name, package))`. Run
  `cd android && ./gradlew :app:testDebugUnitTest` — Expected: all pass.

- [ ] **Step 8: Commit** — `feat(ffi): adjustments, rate cards, crowd data, questions and report bodies for the app`.

## Part B — Personal adjustments

### Task 4: Typing euros and percentages

**Files:**
- Create: `android/app/src/main/kotlin/io/github/locadani/whichride/format/MoneyInput.kt`
- Test: `android/app/src/test/kotlin/io/github/locadani/whichride/format/MoneyInputTest.kt`

**Interfaces:**
- Produces: `parseEuros(text: String): Long?` (cents ≥ 0), `parsePercent(text: String): Int?`
  (basis points 1–10 000), `eurosInput(cents: Long, locale: Locale): String`,
  `percentInput(basisPoints: Int, locale: Locale): String`.

- [ ] **Step 1: Write the failing test**

```kotlin
package io.github.locadani.whichride.format

import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class MoneyInputTest {
    @Test fun `euros accept a decimal comma`() {
        assertEquals(29L, parseEuros("0,29"))
        assertEquals(29L, parseEuros("0.29"))
        assertEquals(150L, parseEuros("1,5"))
        assertEquals(500L, parseEuros("5"))
        assertEquals(500L, parseEuros("€ 5,00"))
        assertEquals(500L, parseEuros("5,00 €"))
        assertEquals(0L, parseEuros("0"))
        assertEquals(29L, parseEuros(",29"))
    }

    @Test fun `euros refuse three decimals and nonsense`() {
        for (text in listOf("", " ", "0,295", "1.2.3", "-1", "abc", "1e3", "12345", "1,", "€")) {
            assertNull(text, parseEuros(text))
        }
    }

    @Test fun `percentages are basis points between 0 and 100`() {
        assertEquals(1_500, parsePercent("15"))
        assertEquals(1_550, parsePercent("15,5"))
        assertEquals(1_525, parsePercent("15.25 %"))
        assertEquals(10_000, parsePercent("100"))
        for (text in listOf("0", "100,01", "101", "", "15,555", "-5")) assertNull(text, parsePercent(text))
    }

    @Test fun `inputs are prefilled with the locale's separator and no needless zeros`() {
        assertEquals("0,29", eurosInput(29, Locale.GERMANY))
        assertEquals("0.29", eurosInput(29, Locale.UK))
        assertEquals("5", eurosInput(500, Locale.ITALY))
        assertEquals("1,50", eurosInput(150, Locale.ITALY))
        assertEquals("15", percentInput(1_500, Locale.GERMANY))
        assertEquals("15,5", percentInput(1_550, Locale.GERMANY))
        assertEquals("15.25", percentInput(1_525, Locale.UK))
    }
}
```

- [ ] **Step 2: Run** `cd android && ./gradlew :app:testDebugUnitTest --tests '*MoneyInputTest*'` — Expected: FAIL, unresolved references.

- [ ] **Step 3: Implement**

```kotlin
package io.github.locadani.whichride.format

import java.text.DecimalFormatSymbols
import java.util.Locale

private val DECIMAL = Regex("""(\d{0,4})(?:[.,](\d{1,2}))?""")

/** Whole units and up to two decimals as hundredths, with integer maths; null if not a number. */
private fun hundredths(text: String, unit: Char): Long? {
    val bare = text.trim().removePrefix(unit.toString()).removeSuffix(unit.toString()).trim()
    val match = DECIMAL.matchEntire(bare) ?: return null
    val (whole, fraction) = match.destructured
    if (whole.isEmpty() && fraction.isEmpty()) return null
    return whole.ifEmpty { "0" }.toLong() * 100 + fraction.padEnd(2, '0').ifEmpty { "0" }.toLong()
}

/** Euros typed with a dot or comma ("0,29", "5", "5,00 €") as cents (plan R20). */
fun parseEuros(text: String): Long? = hundredths(text, '€')

/** A percentage above 0 and at most 100, up to two decimals, as basis points. */
fun parsePercent(text: String): Int? = hundredths(text, '%')?.takeIf { it in 1..10_000 }?.toInt()

/** Cents for an input field: "5" for whole euros, else two decimals with the locale's separator. */
fun eurosInput(cents: Long, locale: Locale): String =
    if (cents % 100 == 0L) "${cents / 100}"
    else "${cents / 100}${separator(locale)}${(cents % 100).toString().padStart(2, '0')}"

/** Basis points for an input field: "15", "15,5", "15,25". */
fun percentInput(basisPoints: Int, locale: Locale): String {
    val fraction = (basisPoints % 100).toString().padStart(2, '0').trimEnd('0')
    return if (fraction.isEmpty()) "${basisPoints / 100}" else "${basisPoints / 100}${separator(locale)}$fraction"
}

private fun separator(locale: Locale) = DecimalFormatSymbols.getInstance(locale).decimalSeparator
```

- [ ] **Step 4: Run** the test again — Expected: PASS (4 tests).

- [ ] **Step 5: Commit** — `feat(android): parse typed euros and percentages`.

### Task 5: Personal data: deals, overrides, chosen apps

**Files:**
- Create: `android/app/src/main/kotlin/io/github/locadani/whichride/personal/Personal.kt`,
  `android/app/src/main/kotlin/io/github/locadani/whichride/personal/PersonalStore.kt`
- Test: `android/app/src/test/kotlin/io/github/locadani/whichride/personal/PersonalTest.kt`,
  `android/app/src/test/kotlin/io/github/locadani/whichride/personal/PersonalStoreTest.kt`

**Interfaces:**
- Consumes: ffi `Adjustments`, `Discount`, `Subscription`, `RefuelCredit`, `PriceOverride`,
  `RateCard`, `cardsToJson`, `cardsFromJson` (Task 3).
- Produces (package `io.github.locadani.whichride.personal`):
  - `sealed interface Deal { id; providerId; enabled }` with `PercentOff(id, providerId, basisPoints: Int, until: Long?, enabled)`,
    `AmountOff(id, providerId, cents: Long, once: Boolean, until: Long?, used: Boolean, enabled)`,
    `Pass(id, providerId, distanceOff: Int, timeOff: Int, packageOff: Int, waiveUnlock: Boolean, enabled)`,
    `RefuelCreditDeal(id, providerId, cents: Long, enabled)`; days are epoch days (`LocalDate.toEpochDay()`).
  - `data class Personal(deals, overrides: List<StoredOverride>, apps: Map<String, String>, lastAskedDay: Int?)`,
    `StoredOverride(offerId, cardsJson)`.
  - `fun Deal.isActive(today: Long): Boolean`; `fun Personal.adjustments(today: Long, refuel: Boolean): Adjustments`;
    `fun Personal.hasActiveRefuelCredit(today: Long): Boolean`;
    `fun Personal.useOnce(providerId: String, today: Long): Pair<Personal, List<String>>` (ids used);
    `fun Personal.restore(ids: List<String>): Personal`;
    `fun Personal.withDeal(deal: Deal)` (replace by id or append), `withoutDeal(id)`, `toggled(id)`;
    `fun Personal.withOverride(offerId: String, cards: List<RateCard>)`, `withoutOverride(offerId)`,
    `fun Personal.overrideCards(offerId: String): List<RateCard>?`,
    `fun Personal.droppingOverridesEqualTo(shared: (offerId: String) -> List<RateCard>): Personal`;
    `fun Personal.appFor(provider: ProviderInfo): String` (chosen package if still offered, else default).
  - `interface PersonalStore { fun load(): Personal; fun save(personal: Personal) }`, `FilePersonalStore(file: File)`.

- [ ] **Step 1: Write the failing tests** — `PersonalTest.kt`:

```kotlin
package io.github.locadani.whichride.personal

import io.github.locadani.whichride.core.AppInfo
import io.github.locadani.whichride.core.CardKind
import io.github.locadani.whichride.core.Discount
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.core.RateCard
import io.github.locadani.whichride.core.Rounding
import java.time.LocalDate
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

fun card(perKm: Long) = RateCard("Standard", CardKind.STANDARD, 100, perKm, 0, 0u, 0u, 0, Rounding.STARTED_UNIT, Rounding.STARTED_UNIT)

class PersonalTest {
    private val oct31 = LocalDate.of(2026, 10, 31).toEpochDay()
    private val bolt20 = PercentOff("a", "bolt", 2_000, until = oct31, enabled = true)
    private val milesOnce = AmountOff("b", "miles", 500, once = true, until = null, used = false, enabled = true)
    private val milesPass = Pass("c", "miles", distanceOff = 1_000, timeOff = 0, packageOff = 0, waiveUnlock = true, enabled = true)
    private val milesFuel = RefuelCreditDeal("d", "miles", 300, enabled = true)
    private val personal = Personal(deals = listOf(bolt20, milesOnce, milesPass, milesFuel))

    @Test fun `a discount applies through its last day`() {
        assertTrue(bolt20.isActive(oct31))
        assertFalse(bolt20.isActive(oct31 + 1))
        assertFalse(bolt20.copy(enabled = false).isActive(oct31))
        assertFalse(milesOnce.copy(used = true).isActive(oct31))
    }

    @Test fun `only active deals reach the core`() {
        val adjustments = personal.adjustments(today = oct31 + 1, refuel = false)
        assertEquals(listOf(Discount.Amount("miles", 500)), adjustments.discounts)
        assertEquals(1, adjustments.subscriptions.size)
        assertEquals(1_000.toUShort(), adjustments.subscriptions[0].distanceOff)
        assertTrue(adjustments.subscriptions[0].waiveBaseFee)
        assertEquals(1, adjustments.refuelCredits.size)
        assertFalse(adjustments.refuel)
        assertTrue(personal.adjustments(oct31, refuel = true).refuel)
        assertEquals(2, personal.adjustments(oct31, false).discounts.size)
    }

    @Test fun `a refuel credit shows the switch only while active`() {
        assertTrue(personal.hasActiveRefuelCredit(oct31))
        assertFalse(personal.toggled("d").hasActiveRefuelCredit(oct31))
    }

    @Test fun `opening a provider uses its once discounts and undo restores them`() {
        val (afterBolt, none) = personal.useOnce("bolt", oct31)
        assertEquals(emptyList<String>(), none)
        assertEquals(personal, afterBolt)
        val (afterMiles, used) = personal.useOnce("miles", oct31)
        assertEquals(listOf("b"), used)
        assertTrue((afterMiles.deals.first { it.id == "b" } as AmountOff).used)
        assertEquals(personal, afterMiles.restore(used))
    }

    @Test fun `deals are added, replaced, toggled and removed by id`() {
        val changed = personal.withDeal(bolt20.copy(basisPoints = 2_500))
        assertEquals(2_500, (changed.deals.first { it.id == "a" } as PercentOff).basisPoints)
        assertEquals(4, changed.deals.size)
        assertEquals(5, personal.withDeal(bolt20.copy(id = "e")).deals.size)
        assertFalse(personal.toggled("a").deals.first { it.id == "a" }.enabled)
        assertEquals(3, personal.withoutDeal("a").deals.size)
    }

    @Test fun `overrides are stored as the core's json and dropped once shared`() {
        val mine = Personal().withOverride("miles.s", listOf(card(89)))
        assertEquals(listOf(card(89)), mine.overrideCards("miles.s"))
        assertEquals(1, mine.adjustments(oct31, false).overrides.size)
        assertEquals(mine, mine.withOverride("miles.s", listOf(card(89))))
        assertEquals(mine, mine.droppingOverridesEqualTo { listOf(card(99)) })
        assertNull(mine.droppingOverridesEqualTo { listOf(card(89)) }.overrideCards("miles.s"))
        assertNull(mine.withoutOverride("miles.s").overrideCards("miles.s"))
    }

    @Test fun `a corrupt override is ignored, not fatal`() {
        val broken = Personal(overrides = listOf(StoredOverride("miles.s", "{")))
        assertNull(broken.overrideCards("miles.s"))
        assertTrue(broken.adjustments(oct31, false).overrides.isEmpty())
    }

    @Test fun `the chosen app is used while the provider still offers it`() {
        val sixt = ProviderInfo("sixt", "SIXT share", "com.trafi.whitelabel.bvg", emptyList(),
            listOf(AppInfo("BVG Jelbi", "com.trafi.whitelabel.bvg"), AppInfo("SIXT", "com.sixt.reservation")))
        assertEquals("com.trafi.whitelabel.bvg", Personal().appFor(sixt))
        assertEquals("com.sixt.reservation", Personal(apps = mapOf("sixt" to "com.sixt.reservation")).appFor(sixt))
        assertEquals("com.trafi.whitelabel.bvg", Personal(apps = mapOf("sixt" to "gone.app")).appFor(sixt))
    }
}
```

  `PersonalStoreTest.kt`:

```kotlin
package io.github.locadani.whichride.personal

import java.io.File
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class PersonalStoreTest {
    @get:Rule val folder = TemporaryFolder()

    @Test fun `personal data survives a restart`() {
        val file = File(folder.root, "personal.json")
        val personal = Personal(
            deals = listOf(
                PercentOff("a", "bolt", 2_000, until = 20_757, enabled = true),
                AmountOff("b", "miles", 500, once = true, until = null, used = false, enabled = true),
                Pass("c", "miles", 1_000, 0, 500, waiveUnlock = true, enabled = false),
                RefuelCreditDeal("d", "miles", 300, enabled = true),
            ),
            apps = mapOf("sixt" to "com.sixt.reservation"),
            lastAskedDay = 20_746,
        ).withOverride("miles.s", listOf(card(89)))
        FilePersonalStore(file).save(personal)
        assertEquals(personal, FilePersonalStore(file).load())
    }

    @Test fun `a missing or unreadable file is an empty start`() {
        val file = File(folder.root, "personal.json")
        assertEquals(Personal(), FilePersonalStore(file).load())
        file.writeText("{not json")
        assertEquals(Personal(), FilePersonalStore(file).load())
        FilePersonalStore(file).save(Personal(lastAskedDay = 1))
        assertEquals(Personal(lastAskedDay = 1), FilePersonalStore(file).load())
    }
}
```

- [ ] **Step 2: Run** `./gradlew :app:testDebugUnitTest --tests '*personal*'` — Expected: FAIL, unresolved references.

- [ ] **Step 3: Implement `Personal.kt`**

```kotlin
package io.github.locadani.whichride.personal

import io.github.locadani.whichride.core.Adjustments
import io.github.locadani.whichride.core.DataException
import io.github.locadani.whichride.core.Discount
import io.github.locadani.whichride.core.PriceOverride
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.core.RateCard
import io.github.locadani.whichride.core.RefuelCredit
import io.github.locadani.whichride.core.Subscription
import io.github.locadani.whichride.core.cardsFromJson
import io.github.locadani.whichride.core.cardsToJson
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * One of the user's own price changes (spec §6, plan R5–R7). Kept on this phone only. Days are
 * epoch days in the phone's time zone; percentages are basis points.
 */
@Serializable
sealed interface Deal {
    val id: String
    val providerId: String
    val enabled: Boolean
}

@Serializable @SerialName("percent")
data class PercentOff(
    override val id: String,
    override val providerId: String,
    val basisPoints: Int,
    val until: Long?,
    override val enabled: Boolean = true,
) : Deal

@Serializable @SerialName("amount")
data class AmountOff(
    override val id: String,
    override val providerId: String,
    val cents: Long,
    val once: Boolean,
    val until: Long?,
    val used: Boolean = false,
    override val enabled: Boolean = true,
) : Deal

/** A subscription: its monthly fee is not counted per trip (§6). */
@Serializable @SerialName("pass")
data class Pass(
    override val id: String,
    override val providerId: String,
    val distanceOff: Int,
    val timeOff: Int,
    val packageOff: Int,
    val waiveUnlock: Boolean,
    override val enabled: Boolean = true,
) : Deal

/** What the provider credits this user for refuelling or charging (decision 47). */
@Serializable @SerialName("refuel")
data class RefuelCreditDeal(
    override val id: String,
    override val providerId: String,
    val cents: Long,
    override val enabled: Boolean = true,
) : Deal

/** The user's own rate cards for an offer, as the core's JSON (decision 9). */
@Serializable
data class StoredOverride(val offerId: String, val cardsJson: String)

@Serializable
data class Personal(
    val deals: List<Deal> = emptyList(),
    val overrides: List<StoredOverride> = emptyList(),
    /** Provider ID → the Android package the user chose to open it with (decision 66). */
    val apps: Map<String, String> = emptyMap(),
    /** `dayOf` of the last §7.2 question shown (UTC day, as the server counts). */
    val lastAskedDay: Int? = null,
)

/** Enabled, not past its last day, and not a once discount already used. */
fun Deal.isActive(today: Long): Boolean = enabled && when (this) {
    is PercentOff -> until == null || today <= until
    is AmountOff -> !used && (until == null || today <= until)
    is Pass, is RefuelCreditDeal -> true
}

/** What applies to a trip today; `refuel` is the result screen's per-trip switch (R6). */
fun Personal.adjustments(today: Long, refuel: Boolean): Adjustments {
    val active = deals.filter { it.isActive(today) }
    return Adjustments(
        overrides = overrides.mapNotNull { stored -> overrideCards(stored.offerId)?.let { PriceOverride(stored.offerId, it) } },
        subscriptions = active.filterIsInstance<Pass>().map {
            Subscription(it.providerId, it.distanceOff.toUShort(), it.timeOff.toUShort(), it.waiveUnlock, it.packageOff.toUShort())
        },
        discounts = active.mapNotNull {
            when (it) {
                is PercentOff -> Discount.Percent(it.providerId, it.basisPoints.toUShort())
                is AmountOff -> Discount.Amount(it.providerId, it.cents)
                else -> null
            }
        },
        refuelCredits = active.filterIsInstance<RefuelCreditDeal>().map { RefuelCredit(it.providerId, it.cents) },
        refuel = refuel,
    )
}

fun Personal.hasActiveRefuelCredit(today: Long): Boolean = deals.any { it is RefuelCreditDeal && it.isActive(today) }

/** Marks the provider's active once discounts used (plan R5); returns their IDs for Undo. */
fun Personal.useOnce(providerId: String, today: Long): Pair<Personal, List<String>> {
    val used = deals.filter { it is AmountOff && it.once && it.providerId == providerId && it.isActive(today) }.map { it.id }
    if (used.isEmpty()) return this to used
    return copy(deals = deals.map { if (it.id in used) (it as AmountOff).copy(used = true) else it }) to used
}

fun Personal.restore(ids: List<String>): Personal =
    copy(deals = deals.map { if (it.id in ids && it is AmountOff) it.copy(used = false) else it })

fun Personal.withDeal(deal: Deal): Personal =
    if (deals.any { it.id == deal.id }) copy(deals = deals.map { if (it.id == deal.id) deal else it })
    else copy(deals = deals + deal)

fun Personal.withoutDeal(id: String): Personal = copy(deals = deals.filterNot { it.id == id })

fun Personal.toggled(id: String): Personal = copy(
    deals = deals.map {
        if (it.id != id) it
        else when (it) {
            is PercentOff -> it.copy(enabled = !it.enabled)
            is AmountOff -> it.copy(enabled = !it.enabled)
            is Pass -> it.copy(enabled = !it.enabled)
            is RefuelCreditDeal -> it.copy(enabled = !it.enabled)
        }
    },
)

fun Personal.withOverride(offerId: String, cards: List<RateCard>): Personal =
    copy(overrides = overrides.filterNot { it.offerId == offerId } + StoredOverride(offerId, cardsToJson(cards)))

fun Personal.withoutOverride(offerId: String): Personal = copy(overrides = overrides.filterNot { it.offerId == offerId })

/** The user's cards for an offer; null when there are none or they can't be read. */
fun Personal.overrideCards(offerId: String): List<RateCard>? {
    val stored = overrides.firstOrNull { it.offerId == offerId } ?: return null
    return try {
        cardsFromJson(stored.cardsJson)
    } catch (_: DataException) {
        null
    }
}

/** Overrides the crowd has caught up with are no longer needed (plan R10). */
fun Personal.droppingOverridesEqualTo(shared: (offerId: String) -> List<RateCard>): Personal =
    copy(overrides = overrides.filterNot { overrideCards(it.offerId) == shared(it.offerId) })

/** The package that opens this provider: the user's choice while the catalogue still offers it. */
fun Personal.appFor(provider: ProviderInfo): String =
    apps[provider.id]?.takeIf { chosen -> provider.apps.any { it.androidPackage == chosen } } ?: provider.androidPackage
```

  Note the `ProviderInfo` constructor order from Task 3 is `(id, name, androidPackage, offers, apps)`.

- [ ] **Step 4: Implement `PersonalStore.kt`**

```kotlin
package io.github.locadani.whichride.personal

import java.io.File
import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json

/** Where [Personal] lives between launches; the ViewModel's tests fake it. */
interface PersonalStore {
    fun load(): Personal
    fun save(personal: Personal)
}

/**
 * One JSON file in app storage (plan R4, decision 67). Backups are off for the whole app, so it
 * never leaves the phone. An unreadable file starts empty rather than stopping the comparison.
 */
class FilePersonalStore(private val file: File) : PersonalStore {
    private val json = Json { ignoreUnknownKeys = true }

    override fun load(): Personal = try {
        if (file.exists()) json.decodeFromString<Personal>(file.readText()) else Personal()
    } catch (_: SerializationException) {
        Personal()
    } catch (_: IllegalArgumentException) {
        Personal()
    }

    override fun save(personal: Personal) {
        val temporary = File(file.parentFile, "${file.name}.tmp")
        temporary.writeText(json.encodeToString(personal))
        temporary.renameTo(file)
    }
}
```

- [ ] **Step 5: Run** `./gradlew :app:testDebugUnitTest --tests '*personal*'` — Expected: PASS (10 tests).

- [ ] **Step 6: Commit** — `feat(android): personal deals, overrides and chosen apps, kept on the phone`.

### Task 6: The discount form

**Files:**
- Create: `android/app/src/main/kotlin/io/github/locadani/whichride/personal/DealForm.kt`
- Test: `android/app/src/test/kotlin/io/github/locadani/whichride/personal/DealFormTest.kt`

**Interfaces:**
- Consumes: `parseEuros`, `parsePercent`, `eurosInput`, `percentInput` (Task 4); `Deal` types (Task 5).
- Produces: `enum class DealKind { PERCENT, AMOUNT, PASS, REFUEL }`, `enum class DealField { PROVIDER, AMOUNT, TIME_OFF, PACKAGE_OFF }`,
  `data class DealForm(editingId: String?, providerId: String?, kind: DealKind, amount: String, timeOff: String, packageOff: String, waiveUnlock: Boolean, once: Boolean, until: Long?, invalid: Set<DealField>)`,
  `fun DealForm.validated(newId: () -> String): Pair<Deal?, Set<DealField>>`, `fun Deal.toForm(locale: Locale): DealForm`.

`amount` holds the percentage (PERCENT, and PASS's per-km value) or euros (AMOUNT, REFUEL).

- [ ] **Step 1: Write the failing test**

```kotlin
package io.github.locadani.whichride.personal

import java.util.Locale
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class DealFormTest {
    private val id = { "new" }

    @Test fun `a percent discount needs a provider and 0 to 100 percent`() {
        val form = DealForm(providerId = "bolt", kind = DealKind.PERCENT, amount = "20", until = 20_757)
        assertEquals(PercentOff("new", "bolt", 2_000, 20_757) to emptySet<DealField>(), form.validated(id))
        assertEquals(setOf(DealField.PROVIDER), form.copy(providerId = null).validated(id).second)
        assertEquals(setOf(DealField.AMOUNT), form.copy(amount = "120").validated(id).second)
    }

    @Test fun `an amount is euros above zero and may be once`() {
        val form = DealForm(providerId = "miles", kind = DealKind.AMOUNT, amount = "5,00", once = true)
        assertEquals(AmountOff("new", "miles", 500, once = true, until = null), form.validated(id).first)
        assertEquals(setOf(DealField.AMOUNT), form.copy(amount = "0").validated(id).second)
    }

    @Test fun `a pass needs at least one saving`() {
        val form = DealForm(providerId = "miles", kind = DealKind.PASS, amount = "10", waiveUnlock = true)
        assertEquals(Pass("new", "miles", 1_000, 0, 0, waiveUnlock = true), form.validated(id).first)
        val empty = form.copy(amount = "", waiveUnlock = false)
        assertNull(empty.validated(id).first)
        assertEquals(setOf(DealField.AMOUNT), empty.validated(id).second)
        assertEquals(setOf(DealField.TIME_OFF), form.copy(timeOff = "abc").validated(id).second)
    }

    @Test fun `a refuel credit is euros above zero`() {
        val form = DealForm(providerId = "miles", kind = DealKind.REFUEL, amount = "3")
        assertEquals(RefuelCreditDeal("new", "miles", 300), form.validated(id).first)
    }

    @Test fun `editing keeps the id and the enabled state`() {
        val deal = AmountOff("b", "miles", 550, once = true, until = 20_757, enabled = false)
        val form = deal.toForm(Locale.GERMANY)
        assertEquals("5,50", form.amount)
        assertEquals("b", form.editingId)
        assertEquals(deal, form.validated(id).first)
    }
}
```

- [ ] **Step 2: Run** — Expected: FAIL, unresolved references.

- [ ] **Step 3: Implement**

```kotlin
package io.github.locadani.whichride.personal

import io.github.locadani.whichride.format.eurosInput
import io.github.locadani.whichride.format.parseEuros
import io.github.locadani.whichride.format.parsePercent
import io.github.locadani.whichride.format.percentInput
import java.util.Locale

enum class DealKind { PERCENT, AMOUNT, PASS, REFUEL }

enum class DealField { PROVIDER, AMOUNT, TIME_OFF, PACKAGE_OFF }

/**
 * The canvas's "New discount" form. `amount` is the percentage (percent, and a pass's per-km
 * saving) or euros (amount, refuel credit). `enabled` survives editing.
 */
data class DealForm(
    val editingId: String? = null,
    val providerId: String? = null,
    val kind: DealKind = DealKind.PERCENT,
    val amount: String = "",
    val timeOff: String = "",
    val packageOff: String = "",
    val waiveUnlock: Boolean = false,
    val once: Boolean = false,
    val until: Long? = null,
    val enabled: Boolean = true,
    val invalid: Set<DealField> = emptySet(),
)

/** The deal this form describes, or the fields to fix. */
fun DealForm.validated(newId: () -> String): Pair<Deal?, Set<DealField>> {
    val invalid = mutableSetOf<DealField>()
    if (providerId == null) invalid += DealField.PROVIDER
    val id = editingId ?: newId()
    val provider = providerId.orEmpty()
    val deal: Deal? = when (kind) {
        DealKind.PERCENT -> parsePercent(amount)?.let { PercentOff(id, provider, it, until, enabled) }
            .also { if (it == null) invalid += DealField.AMOUNT }
        DealKind.AMOUNT -> parseEuros(amount)?.takeIf { it > 0 }?.let { AmountOff(id, provider, it, once, until, enabled = enabled) }
            .also { if (it == null) invalid += DealField.AMOUNT }
        DealKind.REFUEL -> parseEuros(amount)?.takeIf { it > 0 }?.let { RefuelCreditDeal(id, provider, it, enabled) }
            .also { if (it == null) invalid += DealField.AMOUNT }
        DealKind.PASS -> {
            fun optional(text: String, field: DealField): Int? =
                if (text.isBlank()) 0 else parsePercent(text) ?: run { invalid += field; null }
            val km = optional(amount, DealField.AMOUNT)
            val minutes = optional(timeOff, DealField.TIME_OFF)
            val packages = optional(packageOff, DealField.PACKAGE_OFF)
            if (km == 0 && minutes == 0 && packages == 0 && !waiveUnlock) invalid += DealField.AMOUNT
            if (km != null && minutes != null && packages != null) Pass(id, provider, km, minutes, packages, waiveUnlock, enabled) else null
        }
    }
    return if (invalid.isEmpty()) deal to invalid else null to invalid
}

fun Deal.toForm(locale: Locale): DealForm = when (this) {
    is PercentOff -> DealForm(id, providerId, DealKind.PERCENT, percentInput(basisPoints, locale), until = until, enabled = enabled)
    is AmountOff -> DealForm(id, providerId, DealKind.AMOUNT, eurosInput(cents, locale), once = once, until = until, enabled = enabled)
    is RefuelCreditDeal -> DealForm(id, providerId, DealKind.REFUEL, eurosInput(cents, locale), enabled = enabled)
    is Pass -> DealForm(
        id, providerId, DealKind.PASS,
        amount = if (distanceOff == 0) "" else percentInput(distanceOff, locale),
        timeOff = if (timeOff == 0) "" else percentInput(timeOff, locale),
        packageOff = if (packageOff == 0) "" else percentInput(packageOff, locale),
        waiveUnlock = waiveUnlock, enabled = enabled,
    )
}
```

  (An edited `AmountOff` keeps `used = false`: saving a used once-discount re-arms it, which is
  what editing it means.)

- [ ] **Step 4: Run** — Expected: PASS (5 tests).

- [ ] **Step 5: Commit** — `feat(android): the discount form and its validation`.

### Task 7: Personal adjustments in the compare flow

**Files:**
- Modify: `compare/CompareViewModel.kt`, `compare/UiState.kt`, `compare/Screen.kt`, `AppContainer.kt`
- Test: `android/app/src/test/kotlin/io/github/locadani/whichride/compare/PersonalFlowTest.kt` (new),
  `compare/Fakes.kt`, `compare/CompareViewModelTest.kt` (factory only)

**Interfaces:**
- Consumes: Task 5 (`Personal`, rules, `PersonalStore`), Task 6 (`DealForm`, `validated`, `toForm`).
- Produces (used by the screens in Task 8):
  - `Screen.Settings`, `Screen.Deals`, `Screen.DealEditor`.
  - `UiState.personal: Personal`, `UiState.refuel: Boolean`, `UiState.dealForm: DealForm?`,
    `UiState.usedOnce: List<String>`, `UiState.today: Long`.
  - `CompareViewModel(…, personal: PersonalStore, today: () -> Long = { LocalDate.now().toEpochDay() }, newId: () -> String = { UUID.randomUUID().toString() })`
    — the three new parameters go after `hasLocationPermission`, before `now`.
  - Methods: `openSettings()`, `openDeals()`, `newDeal()`, `editDeal(id)`, `onDealForm(form)`,
    `saveDeal()`, `deleteDeal()`, `toggleDeal(id)`, `setRefuel(on)`, `chooseApp(providerId, androidPackage)`,
    `undoOnce()`, `onceMessageShown()`.

- [ ] **Step 1: Fakes and factory.** Append to `Fakes.kt`:

```kotlin
class FakePersonalStore(var saved: Personal = Personal()) : PersonalStore {
    override fun load() = saved
    override fun save(personal: Personal) { saved = personal }
}

/** 2026-10-20 12:00 UTC, as `now` in the ViewModel tests. */
const val NOW = 1_792_497_600_000L
val TODAY: Long = java.time.LocalDate.of(2026, 10, 20).toEpochDay()
const val COORDINATE_LINK = "https://www.google.com/maps/search/?api=1&query=52.502287,13.431605"
```

  (imports `io.github.locadani.whichride.personal.Personal`, `…personal.PersonalStore`). In
  `CompareViewModelTest.viewModel()` add `personal = FakePersonalStore(), today = { TODAY },`
  after `hasLocationPermission`, and use `now = { NOW }`.

- [ ] **Step 2: Write the failing test** `PersonalFlowTest.kt`:

```kotlin
package io.github.locadani.whichride.compare

import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.personal.AmountOff
import io.github.locadani.whichride.personal.DealField
import io.github.locadani.whichride.personal.DealForm
import io.github.locadani.whichride.personal.DealKind
import io.github.locadani.whichride.personal.Personal
import io.github.locadani.whichride.personal.RefuelCreditDeal
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class PersonalFlowTest {
    private val dispatcher = StandardTestDispatcher()
    private val launcher = FakeLauncher()
    private var personal = FakePersonalStore()

    @Before fun main() = Dispatchers.setMain(dispatcher)
    @After fun reset() = Dispatchers.resetMain()

    private fun TestScope.trip(): CompareViewModel = CompareViewModel(
        routes = FakeRoutes(),
        location = { ALEX },
        launcher = launcher,
        store = FakeStore(),
        priceLoader = SeedPrices(),
        hasLocationPermission = { true },
        personal = personal,
        today = { TODAY },
        newId = { "new" },
        now = { NOW },
    ).also {
        advanceUntilIdle()
        it.onShared(COORDINATE_LINK)
        advanceUntilIdle()
    }

    private fun CompareViewModel.cheapest(provider: String) =
        state.value.quotes.filter { it.providerId == provider }.minOf { it.totalCents }

    private fun CompareViewModel.add(choice: Choice, provider: String) {
        addCars(); pickProvider(provider); toggle(choice); commitCars()
    }

    @Test fun `a saved discount lowers that provider's prices only`() = runTest(dispatcher) {
        val vm = trip()
        val bolt = vm.cheapest("bolt")
        val miles = vm.cheapest("miles")
        vm.openSettings(); vm.openDeals(); vm.newDeal()
        vm.onDealForm(DealForm(providerId = "bolt", kind = DealKind.PERCENT, amount = "50"))
        vm.saveDeal()
        assertEquals(Screen.Deals, vm.state.value.screen)
        assertEquals((bolt + 1) / 2, vm.cheapest("bolt"))
        assertEquals(miles, vm.cheapest("miles"))
        assertEquals(1, personal.saved.deals.size)
    }

    @Test fun `an invalid discount keeps the form open with its fields marked`() = runTest(dispatcher) {
        val vm = trip()
        vm.openSettings(); vm.openDeals(); vm.newDeal()
        vm.onDealForm(DealForm(providerId = null, kind = DealKind.PERCENT, amount = "x"))
        vm.saveDeal()
        assertEquals(Screen.DealEditor, vm.state.value.screen)
        assertEquals(setOf(DealField.PROVIDER, DealField.AMOUNT), vm.state.value.dealForm!!.invalid)
        assertTrue(personal.saved.deals.isEmpty())
    }

    @Test fun `open uses the once discount of that provider only`() = runTest(dispatcher) {
        personal = FakePersonalStore(Personal(deals = listOf(AmountOff("m", "miles", 100, once = true, until = null))))
        val vm = trip()
        vm.add(Choice.AnyOf("bolt"), "bolt")
        vm.openChosen()
        assertEquals(listOf("ee.mtakso.client"), launcher.providers)
        assertTrue(vm.state.value.usedOnce.isEmpty())
        vm.add(Choice.AnyOf("miles"), "miles")
        vm.select(Choice.AnyOf("miles"))
        vm.openChosen()
        assertEquals("com.driveby.app", launcher.providers.last())
        assertEquals(listOf("m"), vm.state.value.usedOnce)
        assertTrue((personal.saved.deals.single() as AmountOff).used)
    }

    @Test fun `undo restores a used discount`() = runTest(dispatcher) {
        personal = FakePersonalStore(Personal(deals = listOf(AmountOff("m", "miles", 100, once = true, until = null))))
        val vm = trip()
        val discounted = vm.cheapest("miles")
        vm.add(Choice.AnyOf("miles"), "miles")
        vm.openChosen()
        assertEquals(discounted + 100, vm.cheapest("miles"))
        vm.undoOnce()
        assertFalse((personal.saved.deals.single() as AmountOff).used)
        assertTrue(vm.state.value.usedOnce.isEmpty())
        assertEquals(discounted, vm.cheapest("miles"))
    }

    @Test fun `the refuel switch is per trip`() = runTest(dispatcher) {
        personal = FakePersonalStore(Personal(deals = listOf(RefuelCreditDeal("f", "miles", 300))))
        val vm = trip()
        val miles = vm.cheapest("miles")
        vm.setRefuel(true)
        assertEquals(miles - 300, vm.cheapest("miles"))
        vm.onShared(COORDINATE_LINK)
        advanceUntilIdle()
        assertFalse(vm.state.value.refuel)
        assertEquals(miles, vm.cheapest("miles"))
    }

    @Test fun `the chosen app opens the provider`() = runTest(dispatcher) {
        val vm = trip()
        vm.chooseApp("sixt", "com.sixt.reservation")
        assertEquals(mapOf("sixt" to "com.sixt.reservation"), personal.saved.apps)
        vm.add(Choice.AnyOf("sixt"), "sixt")
        vm.openChosen()
        assertEquals("com.sixt.reservation", launcher.providers.last())
        vm.pickProvider("sixt")
        vm.checkIn("sixt")
        assertEquals("com.sixt.reservation", launcher.providers.last())
    }

    @Test fun `deleting and toggling a deal`() = runTest(dispatcher) {
        personal = FakePersonalStore(Personal(deals = listOf(AmountOff("m", "miles", 100, once = false, until = null))))
        val vm = trip()
        val discounted = vm.cheapest("miles")
        vm.toggleDeal("m")
        assertEquals(discounted + 100, vm.cheapest("miles"))
        vm.openSettings(); vm.openDeals(); vm.editDeal("m")
        assertEquals("1", vm.state.value.dealForm!!.amount)
        vm.deleteDeal()
        assertTrue(personal.saved.deals.isEmpty())
        assertEquals(Screen.Deals, vm.state.value.screen)
    }
}
```

  (MILES's seed price for 7.4 km is well above €3, so the refuel credit and the €1 discount are
  never cut by the zero floor.)

- [ ] **Step 3: Run** `./gradlew :app:testDebugUnitTest --tests '*PersonalFlowTest*'` — Expected: FAIL, unresolved references.

- [ ] **Step 4: State and screens.** `Screen.kt` add `data object Settings : Screen`,
  `data object Deals : Screen`, `data object DealEditor : Screen`. `UiState.kt`, after
  `carFilter`:

```kotlin
    // Personal (§6)
    val personal: Personal = Personal(),
    /** Epoch day in the phone's time zone, for "expired" labels. */
    val today: Long = 0,
    /** "I'll refuel or charge" for this trip (R6). */
    val refuel: Boolean = false,
    val dealForm: DealForm? = null,
    /** Once discounts just used by Open <provider>, for the Undo snackbar (R5). */
    val usedOnce: List<String> = emptyList(),
```

- [ ] **Step 5: ViewModel.** Constructor gains, after `hasLocationPermission`:

```kotlin
    private val personal: PersonalStore,
    private val today: () -> Long = { LocalDate.now().toEpochDay() },
    private val newId: () -> String = { UUID.randomUUID().toString() },
```

  Name it `personal` (the tests pass `personal = …`):
  `private val personal: PersonalStore,`; inside the class `personal` is the store and
  `state.value.personal` the data. The initial state becomes
  `UiState(choices = store.load(), personal = personal.load(), today = today())`.

  In `startTrip`'s `copy(...)` add `refuel = false`. Replace `reprice()`'s two core calls:

```kotlin
        val adjustments = state.value.personal.adjustments(today(), state.value.refuel)
        val options = prices.options(trip, state.value.choices.toList(), adjustments, nowSeconds)
        mutableState.update {
            it.copy(
                quotes = prices.quotes(trip, adjustments, nowSeconds),
                options = options,
                today = today(),
                selected = it.selected?.takeIf { s -> options.ranked.any { o -> o.choice == s } },
            )
        }
```

  with `val nowSeconds = now() / 1_000` replacing the old `today` local; delete
  `NO_ADJUSTMENTS`. Replace `openChosen()` and `checkIn()`'s launcher calls:

```kotlin
    /** Open <provider> for the selected option, else the cheapest; uses its once discounts (R5). */
    fun openChosen() {
        val ranked = state.value.options?.ranked.orEmpty()
        val chosen = ranked.firstOrNull { it.choice == state.value.selected } ?: ranked.firstOrNull() ?: return
        val provider = providerOf(chosen.quote.providerId) ?: return
        val (next, used) = state.value.personal.useOnce(provider.id, today())
        if (used.isNotEmpty()) {
            savePersonal(next)
            mutableState.update { it.copy(usedOnce = used) }
        }
        launcher.openProvider(state.value.personal.appFor(provider))
    }
```

  and in `checkIn`: `providerOf(providerId)?.let { launcher.openProvider(state.value.personal.appFor(it)) }`.
  Replace `packageOf` with
  `private fun providerOf(providerId: String) = state.value.providers.firstOrNull { it.id == providerId }`.
  Add a section:

```kotlin
    // --- Personal (§6) -----------------------------------------------------------------

    fun openSettings() = push(Screen.Settings)

    fun openDeals() = push(Screen.Deals)

    fun newDeal() {
        mutableState.update { it.copy(dealForm = DealForm(providerId = it.providers.singleOrNull()?.id)) }
        push(Screen.DealEditor)
    }

    fun editDeal(id: String) {
        val deal = state.value.personal.deals.firstOrNull { it.id == id } ?: return
        mutableState.update { it.copy(dealForm = deal.toForm(Locale.getDefault())) }
        push(Screen.DealEditor)
    }

    /** Every edit of the form; marks are cleared until the next save. */
    fun onDealForm(form: DealForm) = mutableState.update { it.copy(dealForm = form.copy(invalid = emptySet())) }

    fun saveDeal() {
        val form = state.value.dealForm ?: return
        val (deal, invalid) = form.validated(newId)
        if (deal == null) return mutableState.update { it.copy(dealForm = form.copy(invalid = invalid)) }
        savePersonal(state.value.personal.withDeal(deal))
        mutableState.update { it.copy(dealForm = null) }
        pop()
    }

    fun deleteDeal() {
        val id = state.value.dealForm?.editingId ?: return
        savePersonal(state.value.personal.withoutDeal(id))
        mutableState.update { it.copy(dealForm = null) }
        pop()
    }

    fun toggleDeal(id: String) = savePersonal(state.value.personal.toggled(id))

    fun setRefuel(on: Boolean) {
        mutableState.update { it.copy(refuel = on) }
        reprice()
    }

    fun chooseApp(providerId: String, androidPackage: String) =
        savePersonal(state.value.personal.let { it.copy(apps = it.apps + (providerId to androidPackage)) })

    fun undoOnce() {
        savePersonal(state.value.personal.restore(state.value.usedOnce))
        mutableState.update { it.copy(usedOnce = emptyList()) }
    }

    fun onceMessageShown() = mutableState.update { it.copy(usedOnce = emptyList()) }

    private fun savePersonal(next: Personal) {
        personal.save(next)
        mutableState.update { it.copy(personal = next) }
        reprice()
    }
```

  `back()` from `Screen.DealEditor` also clears `dealForm` (add
  `dealForm = if (stack.last() == Screen.DealEditor) null else it.dealForm` to its `copy`).

- [ ] **Step 6: Wiring.** In `AppContainer`, pass
  `personal = FilePersonalStore(File(app.filesDir, "personal.json")),` to the ViewModel.

- [ ] **Step 7: Run** `./gradlew :app:testDebugUnitTest` — Expected: all pass (7 new).

- [ ] **Step 8: Commit** — `feat(android): apply discounts, passes, refuel credits and the chosen app`.

### Task 8: Settings, discounts and the result screen's personal controls

**Files:**
- Create: `ui/SettingsScreen.kt`, `ui/DealsScreen.kt`, `ui/DealEditorScreen.kt`
- Modify: `ui/ResultScreen.kt` (Discounts pill, refuel switch), `ui/HomeScreen.kt` (settings
  gear instead of the Licences link), `ui/WhichrideNav.kt` (routes, Undo snackbar),
  `launch/Launcher.kt` (`openAppLanguageSettings`), `res/values/strings.xml`
- Test: `ui/DealsScreenTest.kt`, `ui/SettingsScreenTest.kt`, `ui/ResultScreenTest.kt`, `launch/LauncherTest.kt`

**Interfaces:**
- Consumes: Task 7's state and methods.
- Produces: `SettingsScreen(state, onBack, onDeals, onChooseApp: (providerId, androidPackage) -> Unit, onLanguage, onLicences)`,
  `DealsScreen(state, onBack, onAdd, onEdit: (id) -> Unit, onToggle: (id) -> Unit)`,
  `DealEditorScreen(state, onBack, onChange: (DealForm) -> Unit, onSave, onDelete)`,
  `Launcher.openAppLanguageSettings()`, `fun Deal.title(providers): String`, `fun Deal.subtitle(today): String` (composables in `ui/Text.kt`).

- [ ] **Step 1: Strings.** Add to `values/strings.xml`:

```xml
    <string name="settings">Settings</string>
    <string name="settings_deals">Discounts and passes</string>
    <plurals name="settings_deals_count">
        <item quantity="one">%d active</item>
        <item quantity="other">%d active</item>
    </plurals>
    <string name="settings_open_with">Open %1$s with</string>
    <string name="settings_language">App language</string>
    <string name="settings_language_body">whichride follows your phone\'s language: English, Italiano, Deutsch or Türkçe.</string>
    <string name="settings_language_change">Change in Android settings</string>

    <string name="deals_title">Discounts and passes</string>
    <string name="deals_body">Applied to every price you see. Kept only on this phone.</string>
    <string name="deals_empty">No discounts yet. Add a promo code\'s discount, a pass you pay for, or what a provider credits you for refuelling.</string>
    <string name="deals_add">Add a discount or pass</string>
    <string name="deals_refuel_title">Refuelling or charging?</string>
    <string name="deals_refuel_body">Some providers give credit when you refuel. Add it as a refuel credit, then turn it on per trip from the result screen.</string>
    <string name="deal_on">%1$s on</string>
    <string name="deal_percent">%1$s −%2$s%%</string>
    <string name="deal_amount">%1$s −%2$s</string>
    <string name="deal_pass">%1$s pass</string>
    <string name="deal_refuel">%1$s refuel credit</string>
    <string name="deal_until">until %1$s</string>
    <string name="deal_no_end">no end date</string>
    <string name="deal_once">once · next trip only</string>
    <string name="deal_used">used</string>
    <string name="deal_expired">expired</string>
    <string name="deal_pass_km">%1$s%% off per km</string>
    <string name="deal_pass_minute">%1$s%% off per minute</string>
    <string name="deal_pass_package">%1$s%% off packages</string>
    <string name="deal_pass_unlock">no unlock fee</string>
    <string name="deal_refuel_sub">when you refuel or charge</string>

    <string name="deal_new">New discount</string>
    <string name="deal_edit">Edit discount</string>
    <string name="deal_provider">Provider</string>
    <string name="deal_provider_missing">Choose a provider</string>
    <string name="deal_kind">Kind</string>
    <string name="deal_kind_percent">% off</string>
    <string name="deal_kind_amount">€ off</string>
    <string name="deal_kind_pass">Pass</string>
    <string name="deal_kind_refuel">Refuel credit</string>
    <string name="deal_field_percent">Discount (%)</string>
    <string name="deal_field_amount">Amount (€)</string>
    <string name="deal_field_pass_km">Off per km (%)</string>
    <string name="deal_field_pass_minute">Off per minute (%)</string>
    <string name="deal_field_pass_package">Off packages (%)</string>
    <string name="deal_field_refuel">Credit (€)</string>
    <string name="deal_field_unlock">No unlock fee</string>
    <string name="deal_field_once">Once (next trip only)</string>
    <string name="deal_field_until">Valid until</string>
    <string name="deal_field_until_none">No end date</string>
    <string name="deal_field_until_clear">Remove end date</string>
    <string name="deal_invalid_percent">Between 0.01 and 100</string>
    <string name="deal_invalid_amount">More than €0, e.g. 5 or 2,50</string>
    <string name="deal_invalid_pass">Enter at least one saving</string>
    <string name="deal_hint_percent">Taken off the provider\'s total for every trip until it expires.</string>
    <string name="deal_hint_amount">Taken off the trip total, never below €0.</string>
    <string name="deal_hint_pass">A pass changes the provider\'s rates for good, e.g. cheaper km or no unlock fee. Its monthly fee is not counted per trip.</string>
    <string name="deal_hint_refuel">Taken off the trip total when you turn on \"I\'ll refuel or charge\" for a trip.</string>
    <string name="deal_save">Save discount</string>
    <string name="deal_delete">Delete</string>
    <string name="ok">OK</string>
    <string name="cancel">Cancel</string>

    <plurals name="discounts_pill">
        <item quantity="one">Discounts · %d</item>
        <item quantity="other">Discounts · %d</item>
    </plurals>
    <string name="discounts_pill_none">Discounts</string>
    <string name="refuel_switch">I\'ll refuel or charge</string>
    <string name="once_used">%1$s used</string>
    <string name="undo">Undo</string>
```

- [ ] **Step 2: Labels** — append to `ui/Text.kt`:

```kotlin
/** "Bolt Drive −20%", "MILES pass", … (canvas Discounts). */
@Composable
fun Deal.title(providers: List<ProviderInfo>): String {
    val provider = providers.nameOf(providerId)
    return when (this) {
        is PercentOff -> stringResource(R.string.deal_percent, provider, percentInput(basisPoints, locale()))
        is AmountOff -> stringResource(R.string.deal_amount, provider, money(cents))
        is Pass -> stringResource(R.string.deal_pass, provider)
        is RefuelCreditDeal -> stringResource(R.string.deal_refuel, provider)
    }
}

/** "until 31 Oct 2026", "once · next trip only", "10% off per km · no unlock fee", "expired". */
@Composable
fun Deal.subtitle(today: Long): String {
    if (this is AmountOff && used) return stringResource(R.string.deal_used)
    if (!isActive(today) && enabled) return stringResource(R.string.deal_expired)
    val loc = locale()
    fun until(day: Long?) = day?.let {
        stringResource(R.string.deal_until, LocalDate.ofEpochDay(it).format(DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM).withLocale(loc)))
    } ?: stringResource(R.string.deal_no_end)
    return when (this) {
        is PercentOff -> until(until)
        is AmountOff -> if (once) stringResource(R.string.deal_once) else until(until)
        is Pass -> buildList {
            if (distanceOff > 0) add(stringResource(R.string.deal_pass_km, percentInput(distanceOff, loc)))
            if (timeOff > 0) add(stringResource(R.string.deal_pass_minute, percentInput(timeOff, loc)))
            if (packageOff > 0) add(stringResource(R.string.deal_pass_package, percentInput(packageOff, loc)))
            if (waiveUnlock) add(stringResource(R.string.deal_pass_unlock))
        }.joinToString(" · ")
        is RefuelCreditDeal -> "${money(cents)} · ${stringResource(R.string.deal_refuel_sub)}"
    }
}
```

  (imports: `java.time.LocalDate`, `java.time.format.DateTimeFormatter`, `java.time.format.FormatStyle`,
  the deal types, `isActive`, `percentInput`.) A once discount shows "once" rather than a date
  even when it has one; the editor still shows its date.

- [ ] **Step 3: Write the failing UI tests.** `DealsScreenTest.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.ui.test.assertIsOff
import androidx.compose.ui.test.assertIsOn
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextReplacement
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.Screen
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.personal.AmountOff
import io.github.locadani.whichride.personal.DealField
import io.github.locadani.whichride.personal.DealForm
import io.github.locadani.whichride.personal.DealKind
import io.github.locadani.whichride.personal.PercentOff
import io.github.locadani.whichride.personal.Personal
import java.time.LocalDate
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class DealsScreenTest {
    @get:Rule val rule = createComposeRule()
    private val today = LocalDate.of(2026, 10, 20).toEpochDay()
    private val deals = listOf(
        PercentOff("a", "bolt", 2_000, until = today + 11),
        AmountOff("b", "miles", 500, once = true, until = null, enabled = false),
        PercentOff("c", "bolt", 1_000, until = today - 1),
    )
    private val state = UiState(stack = listOf(Screen.Home, Screen.Settings, Screen.Deals), providers = PROVIDERS, personal = Personal(deals), today = today)

    @Test fun `deals show their state and switch`() {
        val toggled = mutableListOf<String>()
        rule.setContent { WhichrideTheme { DealsScreen(state, {}, {}, {}, { toggled += it }) } }
        rule.onNodeWithText("Bolt Drive −20%").assertExists()
        rule.onNodeWithText("once · next trip only").assertExists()
        rule.onNodeWithText("expired").assertExists()
        rule.onNodeWithContentDescription("Bolt Drive −20% on").assertIsOn()
        rule.onNodeWithContentDescription("MILES −€5.00 on").assertIsOff().performClick()
        assertEquals(listOf("b"), toggled)
    }

    @Test fun `the editor shows fields for the kind and marks what to fix`() {
        var form = DealForm(providerId = null, kind = DealKind.PASS, invalid = setOf(DealField.PROVIDER, DealField.AMOUNT))
        rule.setContent {
            WhichrideTheme { DealEditorScreen(state.copy(dealForm = form), {}, { form = it }, {}, {}) }
        }
        rule.onNodeWithText("Choose a provider").assertExists()
        rule.onNodeWithText("Enter at least one saving").assertExists()
        rule.onNodeWithText("Off per minute (%)").assertExists()
        rule.onNodeWithText("Valid until").assertDoesNotExist()
        rule.onNodeWithText("Bolt Drive").performClick()
        assertEquals("bolt", form.providerId)
        rule.onNodeWithText("Off per km (%)").performTextReplacement("10")
        assertEquals("10", form.amount)
    }
}
```

  (Robolectric runs with the `en-rUS` locale by default, so money reads "€5.00"; if your
  Robolectric config differs, use the string `money(500)` produces in the test's locale.)

  `SettingsScreenTest.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.AppInfo
import io.github.locadani.whichride.core.ProviderInfo
import io.github.locadani.whichride.designsystem.WhichrideTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class SettingsScreenTest {
    @get:Rule val rule = createComposeRule()
    private val sixt = ProviderInfo("sixt", "SIXT share", "com.trafi.whitelabel.bvg", emptyList(),
        listOf(AppInfo("BVG Jelbi", "com.trafi.whitelabel.bvg"), AppInfo("SIXT", "com.sixt.reservation")))

    @Test fun `only providers with a choice of apps are listed, the current one selected`() {
        val chosen = mutableListOf<Pair<String, String>>()
        rule.setContent {
            WhichrideTheme { SettingsScreen(UiState(providers = PROVIDERS + sixt), {}, {}, { p, a -> chosen += p to a }, {}, {}) }
        }
        rule.onNodeWithText("Open SIXT share with").assertExists()
        rule.onNodeWithText("Open MILES with").assertDoesNotExist()
        rule.onNodeWithText("BVG Jelbi").assertIsSelected()
        rule.onNodeWithText("SIXT").performClick()
        assertEquals(listOf("sixt" to "com.sixt.reservation"), chosen)
    }
}
```

  In `ResultScreenTest.kt` add:

```kotlin
    @Test fun `the refuel switch shows only with an active credit`() {
        var refuel: Boolean? = null
        val withCredit = resultState().copy(personal = Personal(deals = listOf(RefuelCreditDeal("f", "miles", 300))))
        rule.setContent { WhichrideTheme { ResultScreen(withCredit, resultActions(onRefuel = { refuel = it })) } }
        rule.onNodeWithText("I'll refuel or charge").performClick()
        assertEquals(true, refuel)
        rule.onNodeWithText("Discounts · 1").assertExists()
    }
```

  This needs `ResultScreen`'s many callbacks grouped: change its signature to
  `ResultScreen(state: UiState, actions: ResultActions)` with

```kotlin
/** Everything the result screen can ask the ViewModel to do. */
data class ResultActions(
    val onBack: () -> Unit = {},
    val onEdit: () -> Unit = {},
    val onChangeStart: () -> Unit = {},
    val onAddCars: () -> Unit = {},
    val onRemove: (Choice) -> Unit = {},
    val onSelect: (Choice) -> Unit = {},
    val onOpen: () -> Unit = {},
    val onOpenMaps: () -> Unit = {},
    val onDeals: () -> Unit = {},
    val onRefuel: (Boolean) -> Unit = {},
)
```

  defined in `ResultScreen.kt`; add `fun resultActions(onRefuel: (Boolean) -> Unit = {}) = ResultActions(onRefuel = onRefuel)`
  to the test file and update the existing `ResultScreenTest` calls to
  `ResultScreen(state, ResultActions(onOpen = …, onRemove = …))` keeping their behaviour. Task 12
  adds `onReport` to `ResultActions`.

  In `LauncherTest.kt` add (Robolectric, like the existing launcher tests):

```kotlin
    @Test @Config(sdk = [33]) fun `app language settings open on Android 13`() {
        AndroidLauncher(context).openAppLanguageSettings()
        val started = shadowOf(context as Application).nextStartedActivity
        assertEquals(Settings.ACTION_APP_LOCALE_SETTINGS, started.action)
        assertEquals("package:${context.packageName}", started.dataString)
    }
```

  (match the existing file's imports and `context` field.)

- [ ] **Step 4: Run** `./gradlew :app:testDebugUnitTest --tests '*DealsScreenTest*' --tests '*SettingsScreenTest*' --tests '*ResultScreenTest*' --tests '*LauncherTest*'` — Expected: FAIL to compile.

- [ ] **Step 5: Launcher.** In `Launcher` add `fun openAppLanguageSettings()` with doc
  "Android 13+'s per-app language screen for whichride (R16); older versions follow the phone."
  `AndroidLauncher`:

```kotlin
    override fun openAppLanguageSettings() {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return
        start(Intent(Settings.ACTION_APP_LOCALE_SETTINGS, "package:${context.packageName}".toUri()))
    }
```

  `FakeLauncher` gets `var languageSettings = 0` and `override fun openAppLanguageSettings() { languageSettings++ }`.
  ViewModel: `fun openLanguageSettings() = launcher.openAppLanguageSettings()`.

- [ ] **Step 6: `SettingsScreen.kt`**

```kotlin
package io.github.locadani.whichride.ui

import android.os.Build
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.RadioButtonDefaults
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
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.personal.appFor
import io.github.locadani.whichride.personal.isActive

private val Space = WhichrideTokens.Space

/** Personal settings (R16): discounts, which app opens a provider, language, licences. */
@Composable
fun SettingsScreen(
    state: UiState,
    onBack: () -> Unit,
    onDeals: () -> Unit,
    onChooseApp: (providerId: String, androidPackage: String) -> Unit,
    onLanguage: () -> Unit,
    onLicences: () -> Unit,
) {
    val colors = WhichrideTheme.colors
    Column(Modifier.fillMaxSize()) {
        TopBar(onBack)
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = Space.md, vertical = Space.xs),
            verticalArrangement = Arrangement.spacedBy(Space.md),
        ) {
            Text(
                stringResource(R.string.settings),
                style = MaterialTheme.typography.headlineMedium,
                modifier = Modifier.padding(horizontal = Space.xxs).semantics { heading() },
            )
            Panel {
                val active = state.personal.deals.count { it.isActive(state.today) }
                NavRow(
                    title = stringResource(R.string.settings_deals),
                    subtitle = pluralStringResource(R.plurals.settings_deals_count, active, active),
                    onClick = onDeals,
                )
            }
            state.providers.filter { it.apps.size > 1 }.forEach { provider ->
                val current = state.personal.appFor(provider)
                Panel {
                    Text(
                        stringResource(R.string.settings_open_with, provider.name),
                        style = MaterialTheme.typography.titleSmall,
                        modifier = Modifier.padding(Space.sm).semantics { heading() },
                    )
                    Column(Modifier.selectableGroup()) {
                        provider.apps.forEach { app ->
                            Row(
                                Modifier.fillMaxWidth().heightIn(min = WhichrideTokens.Size.touchTarget)
                                    .selectable(app.androidPackage == current, role = Role.RadioButton) {
                                        onChooseApp(provider.id, app.androidPackage)
                                    }
                                    .padding(horizontal = Space.sm),
                                verticalAlignment = Alignment.CenterVertically,
                                horizontalArrangement = Arrangement.spacedBy(Space.sm),
                            ) {
                                RadioButton(
                                    selected = app.androidPackage == current,
                                    onClick = null,
                                    colors = RadioButtonDefaults.colors(selectedColor = colors.accent, unselectedColor = colors.outline),
                                )
                                Text(app.name, style = MaterialTheme.typography.bodyLarge)
                            }
                        }
                    }
                }
            }
            Panel {
                Column(Modifier.padding(Space.sm), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
                    Text(stringResource(R.string.settings_language), style = MaterialTheme.typography.titleSmall)
                    Text(stringResource(R.string.settings_language_body), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
                    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                        SecondaryButton(stringResource(R.string.settings_language_change), onLanguage)
                    }
                }
            }
            Panel { NavRow(title = stringResource(R.string.licences), subtitle = null, onClick = onLicences) }
        }
    }
}

/** A row that opens another screen. */
@Composable
fun NavRow(title: String, subtitle: String?, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().heightIn(min = WhichrideTokens.Size.controlLg)
            .selectable(false, role = Role.Button, onClick = onClick).padding(horizontal = Space.sm),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.titleSmall)
            subtitle?.let { Text(it, style = MaterialTheme.typography.bodySmall, color = WhichrideTheme.colors.onSurfaceMuted) }
        }
        Icon(Icons.AutoMirrored.Filled.KeyboardArrowRight, contentDescription = null)
    }
}
```

  `NavRow` uses `selectable(false, role = Role.Button)` so TalkBack says "button" without a
  selection state; if the reviewer prefers, `clickable(role = Role.Button)` is equivalent.

- [ ] **Step 7: `DealsScreen.kt`**

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens

private val Space = WhichrideTokens.Space

/** Canvas "Discounts and passes": every deal with an on/off switch; tap a row to edit it. */
@Composable
fun DealsScreen(state: UiState, onBack: () -> Unit, onAdd: () -> Unit, onEdit: (String) -> Unit, onToggle: (String) -> Unit) {
    val colors = WhichrideTheme.colors
    Column(Modifier.fillMaxSize()) {
        TopBar(onBack)
        LazyColumn(
            Modifier.weight(1f),
            contentPadding = PaddingValues(horizontal = Space.md, vertical = Space.xs),
            verticalArrangement = Arrangement.spacedBy(Space.sm),
        ) {
            item {
                Column(Modifier.padding(horizontal = Space.xxs), verticalArrangement = Arrangement.spacedBy(Space.xxs)) {
                    Text(stringResource(R.string.deals_title), style = MaterialTheme.typography.headlineMedium, modifier = Modifier.semantics { heading() })
                    Text(stringResource(R.string.deals_body), style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceMuted)
                }
            }
            if (state.personal.deals.isEmpty()) {
                item { Text(stringResource(R.string.deals_empty), style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceMuted) }
            }
            items(state.personal.deals, key = { it.id }) { deal ->
                val title = deal.title(state.providers)
                Surface(shape = MaterialTheme.shapes.large, color = colors.surface) {
                    Row(
                        Modifier.clickable { onEdit(deal.id) }.padding(start = Space.md, end = Space.sm, top = Space.sm, bottom = Space.sm),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(Space.sm),
                    ) {
                        Monogram(state.providers.nameOf(deal.providerId), large = true)
                        Column(Modifier.weight(1f)) {
                            Text(title, style = MaterialTheme.typography.titleSmall)
                            Text(deal.subtitle(state.today), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
                        }
                        val description = stringResource(R.string.deal_on, title)
                        Switch(
                            checked = deal.enabled,
                            onCheckedChange = { onToggle(deal.id) },
                            modifier = Modifier.semantics { contentDescription = description },
                            colors = SwitchDefaults.colors(
                                checkedTrackColor = colors.accent,
                                checkedThumbColor = colors.onAccent,
                                uncheckedTrackColor = colors.surfaceHighest,
                                uncheckedThumbColor = colors.onSurfaceMuted,
                                uncheckedBorderColor = colors.outline,
                            ),
                        )
                    }
                }
            }
            item { SecondaryButton(stringResource(R.string.deals_add), onAdd, icon = Icons.Filled.Add) }
            item {
                Surface(shape = MaterialTheme.shapes.large, color = colors.background, border = BorderStroke(WhichrideTokens.Size.stroke, colors.outlineVariant)) {
                    Column(Modifier.padding(Space.md), verticalArrangement = Arrangement.spacedBy(Space.xxs)) {
                        Text(stringResource(R.string.deals_refuel_title), style = MaterialTheme.typography.titleSmall)
                        Text(stringResource(R.string.deals_refuel_body), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
                    }
                }
            }
        }
    }
}
```

  Before writing it, add the switch's colour pairs to `design/contrast.json` (checked thumb on
  checked track = `on-accent`/`accent`; unchecked thumb on unchecked track =
  `on-surface-muted`/`surface-highest`, the unchecked border `outline` against `surface`, both
  themes) and run `cd design && npm test`. If a pair fails WCAG 1.4.11 (3:1), use the pair the
  check accepts (e.g. thumb `onSurface`) — decision 51 already rejected the canvas's light
  off-switch for this reason.

- [ ] **Step 8: `DealEditorScreen.kt`**

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DatePicker
import androidx.compose.material3.DatePickerDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberDatePickerState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardType
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens
import io.github.locadani.whichride.personal.DealField
import io.github.locadani.whichride.personal.DealForm
import io.github.locadani.whichride.personal.DealKind
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.time.format.FormatStyle

private val Space = WhichrideTokens.Space
private const val MILLIS_PER_DAY = 86_400_000L

/** Canvas "New discount": provider, kind, the kind's values, end date or "once". */
@OptIn(ExperimentalLayoutApi::class, ExperimentalMaterial3Api::class)
@Composable
fun DealEditorScreen(state: UiState, onBack: () -> Unit, onChange: (DealForm) -> Unit, onSave: () -> Unit, onDelete: () -> Unit) {
    val form = state.dealForm ?: return
    val colors = WhichrideTheme.colors
    var picking by rememberSaveable { mutableStateOf(false) }
    Column(Modifier.fillMaxSize().imePadding()) {
        TopBar(onBack, close = true) {
            Text(
                stringResource(if (form.editingId == null) R.string.deal_new else R.string.deal_edit),
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.weight(1f).semantics { heading() },
            )
            if (form.editingId != null) TextButton(onClick = onDelete) { Text(stringResource(R.string.deal_delete), color = colors.error) }
        }
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = Space.md, vertical = Space.xs),
            verticalArrangement = Arrangement.spacedBy(Space.lg),
        ) {
            Section(stringResource(R.string.deal_provider)) {
                FlowRow(Modifier.selectableGroup(), horizontalArrangement = Arrangement.spacedBy(Space.xs), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
                    state.providers.forEach { provider ->
                        Choice(provider.name, form.providerId == provider.id) { onChange(form.copy(providerId = provider.id)) }
                    }
                }
                if (DealField.PROVIDER in form.invalid) ErrorText(stringResource(R.string.deal_provider_missing))
            }
            Section(stringResource(R.string.deal_kind)) {
                FlowRow(Modifier.selectableGroup(), horizontalArrangement = Arrangement.spacedBy(Space.xs), verticalArrangement = Arrangement.spacedBy(Space.xs)) {
                    DealKind.entries.forEach { kind ->
                        Choice(stringResource(kind.label()), form.kind == kind) { onChange(form.copy(kind = kind, invalid = emptySet())) }
                    }
                }
            }
            when (form.kind) {
                DealKind.PERCENT -> Amount(R.string.deal_field_percent, form.amount, DealField.AMOUNT in form.invalid, R.string.deal_invalid_percent) { onChange(form.copy(amount = it)) }
                DealKind.AMOUNT -> Amount(R.string.deal_field_amount, form.amount, DealField.AMOUNT in form.invalid, R.string.deal_invalid_amount) { onChange(form.copy(amount = it)) }
                DealKind.REFUEL -> Amount(R.string.deal_field_refuel, form.amount, DealField.AMOUNT in form.invalid, R.string.deal_invalid_amount) { onChange(form.copy(amount = it)) }
                DealKind.PASS -> {
                    Amount(R.string.deal_field_pass_km, form.amount, DealField.AMOUNT in form.invalid, R.string.deal_invalid_pass) { onChange(form.copy(amount = it)) }
                    Amount(R.string.deal_field_pass_minute, form.timeOff, DealField.TIME_OFF in form.invalid, R.string.deal_invalid_percent) { onChange(form.copy(timeOff = it)) }
                    Amount(R.string.deal_field_pass_package, form.packageOff, DealField.PACKAGE_OFF in form.invalid, R.string.deal_invalid_percent) { onChange(form.copy(packageOff = it)) }
                    SwitchRow(stringResource(R.string.deal_field_unlock), form.waiveUnlock) { onChange(form.copy(waiveUnlock = it)) }
                }
            }
            if (form.kind == DealKind.AMOUNT) SwitchRow(stringResource(R.string.deal_field_once), form.once) { onChange(form.copy(once = it)) }
            if (form.kind == DealKind.PERCENT || form.kind == DealKind.AMOUNT) {
                Section(stringResource(R.string.deal_field_until)) {
                    val until = form.until?.let { LocalDate.ofEpochDay(it).format(DateTimeFormatter.ofLocalizedDate(FormatStyle.MEDIUM).withLocale(locale())) }
                    Row(horizontalArrangement = Arrangement.spacedBy(Space.xs), verticalAlignment = Alignment.CenterVertically) {
                        SecondaryButton(until ?: stringResource(R.string.deal_field_until_none), { picking = true }, Modifier.weight(1f))
                        if (form.until != null) TextButton(onClick = { onChange(form.copy(until = null)) }) { Text(stringResource(R.string.deal_field_until_clear)) }
                    }
                }
            }
            Text(stringResource(form.kind.hint()), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
        }
        PrimaryButton(stringResource(R.string.deal_save), onSave, Modifier.padding(Space.md))
    }
    if (picking) {
        val picker = rememberDatePickerState(initialSelectedDateMillis = form.until?.times(MILLIS_PER_DAY))
        DatePickerDialog(
            onDismissRequest = { picking = false },
            confirmButton = {
                TextButton(onClick = {
                    picker.selectedDateMillis?.let { onChange(form.copy(until = it / MILLIS_PER_DAY)) }
                    picking = false
                }) { Text(stringResource(R.string.ok)) }
            },
            dismissButton = { TextButton(onClick = { picking = false }) { Text(stringResource(R.string.cancel)) } },
        ) { DatePicker(picker) }
    }
}

@Composable
private fun Section(title: String, content: @Composable () -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(Space.xs)) {
        Text(title, style = MaterialTheme.typography.labelMedium, color = WhichrideTheme.colors.onSurfaceMuted)
        content()
    }
}

@Composable
private fun Choice(label: String, selected: Boolean, onClick: () -> Unit) {
    val colors = WhichrideTheme.colors
    FilterChip(
        selected = selected,
        onClick = onClick,
        label = { Text(label, style = MaterialTheme.typography.labelLarge) },
        modifier = Modifier.heightIn(min = WhichrideTokens.Size.touchTarget),
        shape = MaterialTheme.shapes.extraLarge,
        colors = FilterChipDefaults.filterChipColors(
            selectedContainerColor = colors.accent,
            selectedLabelColor = colors.onAccent,
            labelColor = colors.onSurface,
        ),
        border = FilterChipDefaults.filterChipBorder(enabled = true, selected = selected, borderColor = colors.outline),
    )
}

@Composable
private fun Amount(label: Int, value: String, invalid: Boolean, error: Int, onValue: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onValue,
        label = { Text(stringResource(label)) },
        isError = invalid,
        supportingText = if (invalid) ({ Text(stringResource(error)) }) else null,
        textStyle = WhichrideTheme.numbers.medium,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
}

@Composable
fun SwitchRow(label: String, checked: Boolean, onChecked: (Boolean) -> Unit) {
    val colors = WhichrideTheme.colors
    Row(
        Modifier.fillMaxWidth().heightIn(min = WhichrideTokens.Size.touchTarget)
            .toggleable(checked, role = Role.Switch, onValueChange = onChecked),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(label, style = MaterialTheme.typography.bodyLarge, modifier = Modifier.weight(1f))
        Switch(
            checked = checked,
            onCheckedChange = null,
            colors = SwitchDefaults.colors(
                checkedTrackColor = colors.accent, checkedThumbColor = colors.onAccent,
                uncheckedTrackColor = colors.surfaceHighest, uncheckedThumbColor = colors.onSurfaceMuted, uncheckedBorderColor = colors.outline,
            ),
        )
    }
}

@Composable
fun ErrorText(text: String) = Text(text, style = MaterialTheme.typography.bodySmall, color = WhichrideTheme.colors.error)

private fun DealKind.label() = when (this) {
    DealKind.PERCENT -> R.string.deal_kind_percent
    DealKind.AMOUNT -> R.string.deal_kind_amount
    DealKind.PASS -> R.string.deal_kind_pass
    DealKind.REFUEL -> R.string.deal_kind_refuel
}

private fun DealKind.hint() = when (this) {
    DealKind.PERCENT -> R.string.deal_hint_percent
    DealKind.AMOUNT -> R.string.deal_hint_amount
    DealKind.PASS -> R.string.deal_hint_pass
    DealKind.REFUEL -> R.string.deal_hint_refuel
}
```

  `DatePicker` works in UTC milliseconds; dividing by a day gives the epoch day the user picked,
  independent of the phone's zone. Use the same switch colours as `DealsScreen` (and after Step 7's
  contrast check, the accepted ones). `MILLIS_PER_DAY` is a time constant, not a graphic value.

- [ ] **Step 9: Result and Home.** In `ResultScreen`, switch to `ResultActions` (Step 3). In the
  top bar, before **Change start**, add the pill:

```kotlin
            val active = state.personal.deals.count { it.isActive(state.today) }
            Surface(
                onClick = actions.onDeals,
                shape = RoundedCornerShape(WhichrideTokens.Radius.full),
                color = colors.surface,
                modifier = Modifier.heightIn(min = Size.touchTarget),
            ) {
                Text(
                    if (active == 0) stringResource(R.string.discounts_pill_none)
                    else pluralStringResource(R.plurals.discounts_pill, active, active),
                    style = MaterialTheme.typography.labelMedium,
                    modifier = Modifier.padding(horizontal = Space.sm, vertical = Space.xs),
                )
            }
```

  and as the first `item` of the `LazyColumn` (both branches, before `Intro`/`Hero`):

```kotlin
                if (state.personal.hasActiveRefuelCredit(state.today)) {
                    item { SwitchRow(stringResource(R.string.refuel_switch), state.refuel, actions.onRefuel) }
                }
```

  In `HomeScreen`, replace `onLicences: () -> Unit` with `onSettings: () -> Unit` and the
  `TextButton` with
  `IconButton(onClick = onSettings) { Icon(Icons.Filled.Settings, contentDescription = stringResource(R.string.settings)) }`;
  update `HomeScreenTest` if it clicks "Licences".

- [ ] **Step 10: Navigation and Undo.** In `WhichrideNav`: add a `SnackbarHostState`
  (`remember { SnackbarHostState() }`) and a `SnackbarHost` aligned bottom-centre in the `Box`;
  route `Screen.Settings`, `Screen.Deals`, `Screen.DealEditor`; Home passes
  `viewModel::openSettings`; Result passes
  `ResultActions(onBack = { viewModel.back() }, onEdit = viewModel::editTrip, onChangeStart = viewModel::changeStart, onAddCars = viewModel::addCars, onRemove = viewModel::remove, onSelect = viewModel::select, onOpen = viewModel::openChosen, onOpenMaps = viewModel::openMaps, onDeals = viewModel::openDeals, onRefuel = viewModel::setRefuel)`.
  The Undo snackbar:

```kotlin
    val usedName = state.usedOnce.firstOrNull()?.let { id -> state.personal.deals.firstOrNull { it.id == id }?.title(state.providers) }
    val usedText = usedName?.let { stringResource(R.string.once_used, it) }
    val undo = stringResource(R.string.undo)
    LaunchedEffect(state.usedOnce) {
        if (usedText == null) return@LaunchedEffect
        val result = snackbar.showSnackbar(usedText, actionLabel = undo, duration = SnackbarDuration.Long)
        if (result == SnackbarResult.ActionPerformed) viewModel.undoOnce() else viewModel.onceMessageShown()
    }
```

  Settings routes: `SettingsScreen(state, onBack = { viewModel.back() }, viewModel::openDeals, viewModel::chooseApp, viewModel::openLanguageSettings, viewModel::openLicences)`;
  `DealsScreen(state, onBack = { viewModel.back() }, viewModel::newDeal, viewModel::editDeal, viewModel::toggleDeal)`;
  `DealEditorScreen(state, onBack = { viewModel.back() }, viewModel::onDealForm, viewModel::saveDeal, viewModel::deleteDeal)`.

- [ ] **Step 11: Run** `./gradlew :app:testDebugUnitTest && (cd ../design && npm test)` — Expected: all pass, single-source guard green.

- [ ] **Step 12: Look at it.** `./gradlew :app:installDebug` (owner's phone, `adb devices`
  shows it; MIUI may need the USB-install prompt accepted once), then open Settings →
  Discounts, add a "Bolt −20 % until end of month", check the result screen's pill and prices;
  take screenshots with `adb exec-out screencap -p > /tmp/…` into the scratchpad and compare
  with the canvas's Discounts artboard (layout, not pixels).

- [ ] **Step 13: Commit** — `feat(android): settings, discounts and passes, refuel switch`.

## Part C — Crowd prices

### Task 9: Signed writes: install ID, `Installs.signed`, `Crowd`

**Files:**
- Modify: `server/TokenStore.kt`, `server/Installs.kt`, `server/ServerApi.kt`
- Create: `android/app/src/main/kotlin/io/github/locadani/whichride/crowd/Crowd.kt`
- Test: `server/InstallsTest.kt`, `server/ServerApiTest.kt`, `crowd/ServerCrowdTest.kt` (new)

**Interfaces:**
- Produces:
  - `data class Install(val id: String, val token: String)`; `TokenStore { load(): Install?; save(Install); clear() }`.
  - `ServerApi.register(...)` returns `ApiResult<Install>`; `writeHash(method, path, installId, body): String`;
    `ServerApi.write(path, body, bearer, integrityToken, read: (String) -> T): ApiResult<T>`;
    `data class ReportOutcome(val reportId: String?, val status: String)`, `readReportOutcome(text)`, `readStatus(text)`.
  - `Installs.signed(path: String, body: String, call: suspend (bearer: String, integrityToken: String) -> ApiResult<T>): ApiResult<T>`.
  - package `crowd`: `enum class Answer(val wire: String) { YES("yes"), NO("no") }`,
    `interface Crowd { suspend fun report(body: String): ApiResult<ReportOutcome>; suspend fun vote(reportId: String, answer: Answer): ApiResult<Unit>; suspend fun confirm(offerId: String, answer: Answer): ApiResult<Unit> }`,
    `class ServerCrowd(api: ServerApi, installs: Installs) : Crowd`.

- [ ] **Step 1: Write the failing tests.** In `InstallsTest.kt` change the fake store to hold an
  `Install?` (`var install: Install? = null`, `load() = install`, `save(install) { this.install = install }`,
  `clear() { install = null }`), update the existing tests to read `store.install?.token`, and add:

```kotlin
    @Test fun aKeystoreFailureKeepsTheInstallForThisSession() = runTest {
        val broken = object : TokenStore {
            override fun load(): Install? = null
            override fun save(install: Install) = throw java.security.KeyStoreException("no key")
            override fun clear() {}
        }
        reply(201, """{"install_id":"123e4567-e89b-42d3-a456-426614174000","token":"bearer-1"}""")
        val installs = Installs(api, integrity, broken, { "abcdefghijklmnop" }, { now })
        assertEquals(ApiResult.Ok("bearer-1"), installs.authorized { ApiResult.Ok(it) })
        assertEquals(ApiResult.Ok("bearer-1"), installs.authorized { ApiResult.Ok(it) })
        assertEquals(1, server.requestCount)
    }

    @Test fun signedWritesBindTheInstallPathAndBody() = runTest {
        store.install = Install("123e4567-e89b-42d3-a456-426614174000", "bearer-1")
        val result = installs().signed("/v1/offers/miles.s/confirmations", """{"answer":"yes"}""") { bearer, token ->
            ApiResult.Ok("$bearer $token")
        }
        assertEquals(ApiResult.Ok("bearer-1 a.b.c.d.e"), result)
        // The README's known value (crates/server/README.md, decision 44).
        assertEquals(listOf("78a4418304e69be8eebd247ac4e93fc97b3fca818a814017cfef1d82be803413"), hashes)
    }

    @Test fun aSignedWriteWithoutPlayIntegrityIsNotSent() = runTest {
        store.install = Install("123e4567-e89b-42d3-a456-426614174000", "bearer-1")
        var called = false
        val result = installs(tokens = { null }).signed("/v1/reports", "{}") { _, _ -> called = true; ApiResult.Ok(Unit) }
        assertEquals(ApiResult.Unavailable(Why.NOT_REGISTERED), result)
        assertFalse(called)
    }

    @Test fun aSignedWriteRegistersAgainOnAnUnknownToken() = runTest {
        store.install = Install("00000000-0000-4000-8000-000000000000", "stale")
        reply(201, """{"install_id":"123e4567-e89b-42d3-a456-426614174000","token":"bearer-2"}""")
        val seen = mutableListOf<String>()
        val result = installs().signed("/v1/offers/miles.s/confirmations", """{"answer":"yes"}""") { bearer, _ ->
            seen += bearer
            if (bearer == "stale") ApiResult.Refused(401, "unauthorized") else ApiResult.Ok(Unit)
        }
        assertEquals(ApiResult.Ok(Unit), result)
        assertEquals(listOf("stale", "bearer-2"), seen)
        assertEquals("78a4418304e69be8eebd247ac4e93fc97b3fca818a814017cfef1d82be803413", hashes.last())
    }
```

  In `ServerApiTest.kt` add:

```kotlin
    @Test fun aTrailingSlashInTheServerUrlIsIgnored() = runTest {
        val slashed = ServerApi(server.url("/").toString(), OkHttpClient())
        reply(200, """{"metres":7400,"seconds":1320}""")
        slashed.route("tok", alex, neun)
        assertEquals("/v1/route", server.takeRequest().url.encodedPath)
    }

    @Test fun writesCarryTheBearerAndTheIntegrityToken() = runTest {
        reply(201, """{"report":"11111111-2222-4333-8444-555555555555","status":"pending"}""")
        val body = """{"cards":[],"offer":"miles.s"}"""
        assertEquals(
            ApiResult.Ok(ReportOutcome("11111111-2222-4333-8444-555555555555", "pending")),
            api.write("/v1/reports", body, "tok", "a.b.c.d.e", ::readReportOutcome),
        )
        val request = server.takeRequest()
        assertEquals("/v1/reports", request.url.encodedPath)
        assertEquals("Bearer tok", request.headers["Authorization"])
        assertEquals("a.b.c.d.e", request.headers["X-Integrity-Token"])
        assertEquals(body, request.body!!.utf8())
    }

    @Test fun aRegistrationReturnsTheInstallId() = runTest {
        reply(201, """{"install_id":"123e4567-e89b-42d3-a456-426614174000","token":"bearer-1"}""")
        assertEquals(ApiResult.Ok(Install("123e4567-e89b-42d3-a456-426614174000", "bearer-1")), api.register("{}", "a.b.c.d.e"))
    }
```

  `crowd/ServerCrowdTest.kt`:

```kotlin
package io.github.locadani.whichride.crowd

import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.Install
import io.github.locadani.whichride.server.Installs
import io.github.locadani.whichride.server.IntegrityTokens
import io.github.locadani.whichride.server.ReportOutcome
import io.github.locadani.whichride.server.ServerApi
import io.github.locadani.whichride.server.TokenStore
import io.github.locadani.whichride.server.writeHash
import kotlinx.coroutines.test.runTest
import mockwebserver3.MockResponse
import mockwebserver3.MockWebServer
import okhttp3.OkHttpClient
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test

class ServerCrowdTest {
    private val server = MockWebServer()
    private val id = "123e4567-e89b-42d3-a456-426614174000"
    private val hashes = mutableListOf<String>()
    private lateinit var crowd: ServerCrowd

    @Before fun start() {
        server.start()
        val api = ServerApi(server.url("/").toString(), OkHttpClient())
        val store = object : TokenStore {
            override fun load() = Install(id, "tok")
            override fun save(install: Install) {}
            override fun clear() {}
        }
        crowd = ServerCrowd(api, Installs(api, IntegrityTokens { hashes += it; "a.b.c.d.e" }, store))
    }

    @After fun stop() = server.close()

    private fun reply(status: Int, body: String = "") =
        server.enqueue(MockResponse.Builder().code(status).body(body).build())

    @Test fun `a vote goes to its report`() = runTest {
        reply(200, """{"status":"accepted"}""")
        val report = "11111111-2222-4333-8444-555555555555"
        assertEquals(ApiResult.Ok(Unit), crowd.vote(report, Answer.NO))
        val request = server.takeRequest()
        assertEquals("/v1/reports/$report/votes", request.url.encodedPath)
        assertEquals("""{"answer":"no"}""", request.body!!.utf8())
        assertEquals(writeHash("POST", "/v1/reports/$report/votes", id, """{"answer":"no"}"""), hashes.single())
    }

    @Test fun `a confirmation answers no content`() = runTest {
        reply(204)
        assertEquals(ApiResult.Ok(Unit), crowd.confirm("miles.s", Answer.YES))
        assertEquals("/v1/offers/miles.s/confirmations", server.takeRequest().url.encodedPath)
    }

    @Test fun `a report sends the core's body byte for byte`() = runTest {
        reply(200, """{"report":null,"status":"current"}""")
        val body = """{"cards":[{"label":"Standard"}],"offer":"miles.s"}"""
        assertEquals(ApiResult.Ok(ReportOutcome(null, "current")), crowd.report(body))
        assertEquals(body, server.takeRequest().body!!.utf8())
    }

    @Test fun `refusals keep their code`() = runTest {
        reply(409, """{"error":"report_closed"}""")
        assertEquals(ApiResult.Refused(409, "report_closed"), crowd.vote("11111111-2222-4333-8444-555555555555", Answer.YES))
    }
}
```

- [ ] **Step 2: Run** `./gradlew :app:testDebugUnitTest --tests '*InstallsTest*' --tests '*ServerApiTest*' --tests '*ServerCrowdTest*'` — Expected: FAIL to compile.

- [ ] **Step 3: `TokenStore.kt`.** Add before the interface:

```kotlin
/** This install on the server: its ID (part of every signed request, decision 44) and its bearer token. */
data class Install(val id: String, val token: String)
```

  The interface becomes `fun load(): Install?`, `fun save(install: Install)`, `fun clear()`.
  `KeystoreTokenStore` encrypts `"${install.id}\n${install.token}"` and in `load()` decodes, then
  `val parts = text.split('\n'); if (parts.size != 2) { clear(); return null }; Install(parts[0], parts[1])`
  (a 5a value holds only a token: it reads as missing, so the install registers again, R15).
  Update its doc comment: "The install ID and bearer token, encrypted with …".

- [ ] **Step 4: `ServerApi.kt`.**

```kotlin
@Serializable private data class RegistrationDto(@SerialName("install_id") val installId: String, val token: String)
@Serializable private data class ReportOutcomeDto(val report: String? = null, val status: String)
@Serializable private data class StatusDto(val status: String)

/** The server's answer to a report: the report it created or joined, and its status (`pending`, `accepted`, `current`). */
data class ReportOutcome(val reportId: String?, val status: String)

private val lenient = Json { ignoreUnknownKeys = true }

fun readReportOutcome(text: String): ReportOutcome =
    lenient.decodeFromString<ReportOutcomeDto>(text).let { ReportOutcome(it.report, it.status) }

fun readStatus(text: String): String = lenient.decodeFromString<StatusDto>(text).status

/** Lowercase hex SHA-256 of a signed write's integrity binding (decision 44). */
fun writeHash(method: String, path: String, installId: String, body: String): String = sha256Hex("$method $path\n$installId\n$body")

fun registrationHash(body: String): String = sha256Hex("POST /v1/installs\n$body")

private fun sha256Hex(text: String) =
    MessageDigest.getInstance("SHA-256").digest(text.toByteArray()).joinToString("") { "%02x".format(it) }
```

  `class ServerApi(baseUrl: String, private val http: OkHttpClient)` with
  `private val baseUrl = baseUrl.trimEnd('/')` (5a minor). `register` returns
  `ApiResult<Install>`: `json.decodeFromString<RegistrationDto>(it).let { dto -> Install(dto.installId, dto.token) }`.
  New method:

```kotlin
    /** A report, vote or confirmation: bearer + a Play Integrity token for these exact bytes (decision 44). */
    suspend fun <T> write(path: String, body: String, bearer: String, integrityToken: String, read: (String) -> T): ApiResult<T> =
        call({ authed(path, body, bearer).header("X-Integrity-Token", integrityToken) }, read)
```

- [ ] **Step 5: `Installs.kt`.** Replace the class body's token handling:

```kotlin
    /** The install registered this session when the Keystore could not keep it (R15). */
    private var memory: Install? = null

    suspend fun <T> authorized(call: suspend (bearer: String) -> ApiResult<T>): ApiResult<T> = withInstall { call(it.token) }

    /** A write bound to this install, path and body; `call` gets the bearer and the integrity token. */
    suspend fun <T> signed(path: String, body: String, call: suspend (bearer: String, integrityToken: String) -> ApiResult<T>): ApiResult<T> =
        withInstall { install ->
            val token = integrity.token(writeHash("POST", path, install.id, body))
                ?: return@withInstall ApiResult.Unavailable(Why.NOT_REGISTERED)
            call(install.token, token)
        }

    private suspend fun <T> withInstall(call: suspend (Install) -> ApiResult<T>): ApiResult<T> {
        val first = when (val install = install()) {
            is ApiResult.Ok -> install.value
            is ApiResult.Unavailable -> return install
            is ApiResult.Refused -> return install
        }
        val result = call(first)
        if (result !is ApiResult.Refused || result.status != 401) return result
        mutex.withLock {
            if (stored() == first) {
                memory = null
                store.clear()
            }
        }
        return when (val again = install()) {
            is ApiResult.Ok -> call(again.value)
            is ApiResult.Unavailable -> again
            is ApiResult.Refused -> again
        }
    }

    private fun stored(): Install? = memory ?: store.load()

    private suspend fun install(): ApiResult<Install> = mutex.withLock {
        stored()?.let { return ApiResult.Ok(it) }
        // … unchanged nonce, integrity and retryAt logic, then on success:
        //     is ApiResult.Ok -> registered.also { keep(it.value) }
    }

    private fun keep(install: Install) {
        memory = install
        try {
            store.save(install)
        } catch (_: java.security.GeneralSecurityException) {
            // The Keystore refused (5a minor): the install lives in memory for this session.
        } catch (_: java.security.ProviderException) {
        }
    }
```

  `KeyStoreException` is a `GeneralSecurityException`; Android's Keystore also throws
  `ProviderException` subclasses. The test's `KeyStoreException` covers the first.

- [ ] **Step 6: `crowd/Crowd.kt`**

```kotlin
package io.github.locadani.whichride.crowd

import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.Installs
import io.github.locadani.whichride.server.ReportOutcome
import io.github.locadani.whichride.server.ServerApi
import io.github.locadani.whichride.server.map
import io.github.locadani.whichride.server.readReportOutcome
import io.github.locadani.whichride.server.readStatus

enum class Answer(val wire: String) { YES("yes"), NO("no") }

/** Crowd prices on the server (§7); every call is a signed write. The ViewModel's tests fake it. */
interface Crowd {
    /** `body` comes from the core (`reportBody`, `proposalBody`) and is sent byte for byte. */
    suspend fun report(body: String): ApiResult<ReportOutcome>
    suspend fun vote(reportId: String, answer: Answer): ApiResult<Unit>
    suspend fun confirm(offerId: String, answer: Answer): ApiResult<Unit>
}

class ServerCrowd(private val api: ServerApi, private val installs: Installs) : Crowd {
    override suspend fun report(body: String) = write(REPORTS, body, ::readReportOutcome)

    override suspend fun vote(reportId: String, answer: Answer) =
        write("/v1/reports/$reportId/votes", answerBody(answer), ::readStatus).map {}

    override suspend fun confirm(offerId: String, answer: Answer) =
        write("/v1/offers/$offerId/confirmations", answerBody(answer)) {}

    private suspend fun <T> write(path: String, body: String, read: (String) -> T): ApiResult<T> =
        installs.signed(path, body) { bearer, token -> api.write(path, body, bearer, token, read) }

    private companion object {
        const val REPORTS = "/v1/reports"
        fun answerBody(answer: Answer) = """{"answer":"${answer.wire}"}"""
    }
}
```

  Report and offer IDs in paths come from the server's catalogue through the core, which accepts
  only UUIDs (Task 3) and catalogue offer IDs.

- [ ] **Step 7: Run** the three test classes, then all unit tests — Expected: PASS.

- [ ] **Step 8: Commit** — `feat(android): signed writes with the install ID; crowd client`.

### Task 10: The price form (reports, corrections, proposals)

**Files:**
- Create: `android/app/src/main/kotlin/io/github/locadani/whichride/crowd/PriceForm.kt`
- Test: `android/app/src/test/kotlin/io/github/locadani/whichride/crowd/PriceFormTest.kt`

**Interfaces:**
- Consumes: ffi `RateCard`, `CardKind`, `reportBody`, `proposalBody`, `reportLimits`, `ReportException`
  (Task 3); `parseEuros`, `eurosInput` (Task 4).
- Produces:
  - `const val PROPOSAL_PREFIX = "proposal."` (equal to the ffi's; a test checks it).
  - `enum class PriceField { NAME, UNLOCK, PER_KM, PER_MINUTE, MINIMUM }`.
  - `sealed interface PricePurpose { data class Report(offerId, providerId, offerName); data class Propose(providerId) }`.
  - `data class PriceForm(purpose, name, unlock, perKm, perMinute, minimum, invalid: Set<PriceField>, sending: Boolean)`.
  - `fun priceForm(purpose: PricePurpose, cards: List<RateCard>, locale: Locale): PriceForm` (prefilled from the standard card).
  - `sealed interface Built { data class Report(body: String, cards: List<RateCard>); data class Proposal(body: String); data object Unchanged; data class Invalid(fields: Set<PriceField>) }`.
  - `fun PriceForm.build(current: List<RateCard>, template: RateCard?): Built` — `current` = the offer's
    cards the user sees now (override or shared); `template` = the provider's first standard card (proposals).

- [ ] **Step 1: Write the failing test**

```kotlin
package io.github.locadani.whichride.crowd

import io.github.locadani.whichride.core.CardKind
import io.github.locadani.whichride.core.RateCard
import io.github.locadani.whichride.core.Rounding
import java.util.Locale
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PriceFormTest {
    private val standard = RateCard("Standard", CardKind.STANDARD, 100, 29, 11, 0u, 0u, 250, Rounding.STARTED_UNIT, Rounding.STARTED_UNIT)
    private val pack = RateCard("3 h", CardKind.PACKAGE, 2_900, 19, 0, 40u, 180u, 0, Rounding.STARTED_UNIT, Rounding.STARTED_UNIT)
    private val report = PricePurpose.Report("bolt.polo", "bolt", "VW Polo")

    @Test fun `the form is prefilled from the standard card`() {
        val form = priceForm(report, listOf(pack, standard), Locale.GERMANY)
        assertEquals(listOf("1", "0,29", "0,11", "2,50"), listOf(form.unlock, form.perKm, form.perMinute, form.minimum))
    }

    @Test fun `a correction replaces only the standard card's values`() {
        val form = priceForm(report, listOf(standard, pack), Locale.GERMANY).copy(perKm = "0,31")
        val built = form.build(listOf(standard, pack), null) as Built.Report
        assertEquals(listOf(standard.copy(perKm = 31), pack), built.cards)
        val body = Json.parseToJsonElement(built.body).jsonObject
        assertEquals("bolt.polo", body["offer"]!!.jsonPrimitive.content)
        assertEquals(2, body["cards"]!!.jsonArray.size)
    }

    @Test fun `unchanged values send nothing`() {
        val form = priceForm(report, listOf(standard), Locale.UK)
        assertEquals(Built.Unchanged, form.build(listOf(standard), null))
    }

    @Test fun `values above the limits are refused before sending`() {
        val form = priceForm(report, listOf(standard), Locale.GERMANY).copy(perKm = "5,01", minimum = "x", unlock = "501")
        assertEquals(Built.Invalid(setOf(PriceField.PER_KM, PriceField.MINIMUM, PriceField.UNLOCK)), form.build(listOf(standard), null))
    }

    @Test fun `a proposal copies rounding and label from the provider's template`() {
        val template = standard.copy(minuteRounding = Rounding.EXACT)
        val form = PriceForm(PricePurpose.Propose("bolt"), name = " Fiat 500e ", unlock = "1", perKm = "0,25", perMinute = "0,10", minimum = "")
        val built = form.build(emptyList(), template) as Built.Proposal
        val body = Json.parseToJsonElement(built.body).jsonObject
        assertEquals("Fiat 500e", body["name"]!!.jsonPrimitive.content)
        val card = body["cards"]!!.jsonArray.single().jsonObject
        assertEquals("exact", card["minute_rounding"]!!.jsonPrimitive.content)
        assertEquals("0", card["minimum"]!!.jsonPrimitive.content)
        assertEquals(Built.Invalid(setOf(PriceField.NAME)), form.copy(name = " ").build(emptyList(), template))
    }

    @Test fun `the prefix matches the core's`() {
        assertEquals(io.github.locadani.whichride.core.proposalPrefix(), PROPOSAL_PREFIX)
        assertTrue(PROPOSAL_PREFIX.endsWith("."))
    }
}
```


- [ ] **Step 2: Run** — Expected: FAIL to compile.

- [ ] **Step 3: Implement**

```kotlin
package io.github.locadani.whichride.crowd

import io.github.locadani.whichride.core.CardKind
import io.github.locadani.whichride.core.RateCard
import io.github.locadani.whichride.core.ReportException
import io.github.locadani.whichride.core.proposalBody
import io.github.locadani.whichride.core.reportBody
import io.github.locadani.whichride.core.reportLimits
import io.github.locadani.whichride.format.eurosInput
import io.github.locadani.whichride.format.parseEuros
import java.util.Locale

/** Offer IDs of pending proposals (plan R14; the ffi's `PROPOSAL_PREFIX`). */
const val PROPOSAL_PREFIX = "proposal."

enum class PriceField { NAME, UNLOCK, PER_KM, PER_MINUTE, MINIMUM }

sealed interface PricePurpose {
    /** A report on an existing offer (§7.1), also the "No, fix it" step of the question (§7.2). */
    data class Report(val offerId: String, val providerId: String, val offerName: String) : PricePurpose
    /** "Car not listed? Add it" (§5.1). */
    data class Propose(val providerId: String) : PricePurpose
}

/** The canvas's "What does Bolt show?" form: the standard card's four prices, as typed. */
data class PriceForm(
    val purpose: PricePurpose,
    val name: String = "",
    val unlock: String = "",
    val perKm: String = "",
    val perMinute: String = "",
    val minimum: String = "",
    val invalid: Set<PriceField> = emptySet(),
    val sending: Boolean = false,
)

sealed interface Built {
    data class Report(val body: String, val cards: List<RateCard>) : Built
    data class Proposal(val body: String) : Built
    /** The values are what the user already sees: nothing to send or store (R9). */
    data object Unchanged : Built
    data class Invalid(val fields: Set<PriceField>) : Built
}

/** The card a report edits: the standard one, else the first (plan R9). */
fun List<RateCard>.editable(): RateCard? = firstOrNull { it.kind == CardKind.STANDARD } ?: firstOrNull()

fun priceForm(purpose: PricePurpose, cards: List<RateCard>, locale: Locale): PriceForm {
    val card = cards.editable() ?: return PriceForm(purpose)
    return PriceForm(
        purpose,
        unlock = eurosInput(card.baseFee, locale),
        perKm = eurosInput(card.perKm, locale),
        perMinute = eurosInput(card.perMinute, locale),
        minimum = eurosInput(card.minimum, locale),
    )
}

/**
 * The request body and cards this form stands for, checked with the core's limits (decision 65).
 * `current` are the offer's cards as the user sees them; `template` is the provider's first
 * standard card, whose label and rounding a proposal copies.
 */
fun PriceForm.build(current: List<RateCard>, template: RateCard?): Built {
    val limits = reportLimits()
    val invalid = mutableSetOf<PriceField>()
    fun euros(text: String, max: Long, field: PriceField, emptyIsZero: Boolean = false): Long =
        (if (emptyIsZero && text.isBlank()) 0L else parseEuros(text))?.takeIf { it <= max } ?: run { invalid += field; 0L }
    val unlock = euros(unlock, limits.maxBaseFee, PriceField.UNLOCK, emptyIsZero = true)
    val perKm = euros(perKm, limits.maxPerUnit, PriceField.PER_KM, emptyIsZero = true)
    val perMinute = euros(perMinute, limits.maxPerUnit, PriceField.PER_MINUTE, emptyIsZero = true)
    val minimum = euros(minimum, limits.maxMinimum, PriceField.MINIMUM, emptyIsZero = true)
    if (purpose is PricePurpose.Propose && (name.isBlank() || name.trim().length > limits.maxNameChars.toInt())) invalid += PriceField.NAME
    if (invalid.isNotEmpty()) return Built.Invalid(invalid)
    return when (purpose) {
        is PricePurpose.Report -> {
            val base = current.editable() ?: return Built.Invalid(setOf(PriceField.PER_KM))
            val edited = base.copy(baseFee = unlock, perKm = perKm, perMinute = perMinute, minimum = minimum)
            if (edited == base) return Built.Unchanged
            val cards = current.map { if (it === base) edited else it }
            try {
                Built.Report(reportBody(purpose.offerId, cards), cards)
            } catch (_: ReportException) {
                Built.Invalid(setOf(PriceField.PER_KM))
            }
        }
        is PricePurpose.Propose -> {
            val base = template ?: return Built.Invalid(setOf(PriceField.PER_KM))
            val card = base.copy(
                label = base.label, kind = CardKind.STANDARD, baseFee = unlock, perKm = perKm, perMinute = perMinute,
                includedKm = 0u, includedMinutes = 0u, minimum = minimum,
            )
            try {
                Built.Proposal(proposalBody(purpose.providerId, name, listOf(card)))
            } catch (_: ReportException.BadName) {
                Built.Invalid(setOf(PriceField.NAME))
            } catch (_: ReportException) {
                Built.Invalid(setOf(PriceField.PER_KM))
            }
        }
    }
}
```

  `it === base` keeps the other cards (packages) exactly as they are even if one equals the
  standard card's values.

- [ ] **Step 4: Run** — Expected: PASS (6 tests).

- [ ] **Step 5: Commit** — `feat(android): the price form for reports, corrections and proposals`.

### Task 11: Reports, the confirmation question and proposals in the compare flow

**Files:**
- Modify: `compare/CompareViewModel.kt`, `compare/UiState.kt`, `compare/Screen.kt`, `AppContainer.kt`
- Test: `compare/CrowdFlowTest.kt` (new), `compare/Fakes.kt`, `compare/CompareViewModelTest.kt` and
  `compare/PersonalFlowTest.kt` (factories: `crowd = FakeCrowd()`)

**Interfaces:**
- Consumes: Task 9 (`Crowd`, `Answer`, `ReportOutcome`), Task 10 (`PriceForm`, `priceForm`, `build`, `Built`,
  `PricePurpose`, `editable`, `PROPOSAL_PREFIX`), Task 3 (`Prices.question`, `cards`, `isLocked`, `dayOf`).
- Produces (used by Task 12):
  - `Screen.PriceForm`.
  - `enum class QuestionStep { ASK, FIX, CONFIRMED, CORRECTED, NOT_SENT }`;
    `data class QuestionState(question: Question, providerId: String, offerName: String, step: QuestionStep)` with `val isProposal: Boolean`;
    `enum class CrowdMessage { SENT, PROPOSED, NOT_SENT, LOCKED, ALREADY_CURRENT }`.
  - `UiState.priceForm: PriceForm?`, `UiState.question: QuestionState?`, `UiState.crowdMessage: CrowdMessage?`,
    `UiState.crowdEnabled: Boolean`.
  - `CompareViewModel(…, personal, crowd: Crowd, crowdEnabled: Boolean = true, today, newId, now)`.
  - Methods: `reportPrice()`, `proposeCar(providerId)`, `onPriceForm(form)`, `savePrice()`,
    `resetPrice(offerId)`, `answerYes()`, `answerNo()`, `dontKnow()`, `closeQuestion()`, `crowdMessageShown()`.
  - `fun UiState.canReport(): Boolean` (Price wrong? is shown) — in `UiState.kt`.

- [ ] **Step 1: Fakes.** Append to `Fakes.kt`:

```kotlin
class FakeCrowd : Crowd {
    var reportResult: ApiResult<ReportOutcome> = ApiResult.Ok(ReportOutcome("22222222-2222-4222-8222-222222222222", "pending"))
    var voteResult: ApiResult<Unit> = ApiResult.Ok(Unit)
    val reports = mutableListOf<String>()
    val votes = mutableListOf<Pair<String, Answer>>()
    val confirmations = mutableListOf<Pair<String, Answer>>()
    override suspend fun report(body: String) = reportResult.also { reports += body }
    override suspend fun vote(reportId: String, answer: Answer) = voteResult.also { votes += reportId to answer }
    override suspend fun confirm(offerId: String, answer: Answer) = voteResult.also { confirmations += offerId to answer }
}

const val PENDING_REPORT = "11111111-2222-4333-8444-555555555555"
const val PROPOSAL_REPORT = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee"
const val CARD_89 = """{"label":"Standard","kind":"standard","base_fee":100,"per_km":89,"per_minute":0,"km_rounding":"started_unit","minute_rounding":"started_unit"}"""

/** A `GET /v1/catalogue` answer over the seed: MILES S has a pending report, never confirmed. */
fun serverCatalogue(
    offers: String = """[{"offer":"miles.s","confirmed_on":null,"locked":false,"pending":{"report":"$PENDING_REPORT","cards":[$CARD_89]}}]""",
    proposals: String = "[]",
) = """{"catalogue":$SEED,"offers":$offers,"proposals":$proposals}"""

/** Prices as if the server answered: cached first, then from the server. */
class ServerPrices(private val json: String = serverCatalogue()) : PriceLoader {
    var loads = 0
    override suspend fun offline() = LoadedPrices(Prices.fromServerResponse(json), PriceSource.CACHED)
    override suspend fun load(): LoadedPrices {
        loads++
        return LoadedPrices(Prices.fromServerResponse(json), PriceSource.SERVER)
    }
}
```

- [ ] **Step 2: Write the failing test** `CrowdFlowTest.kt`:

```kotlin
package io.github.locadani.whichride.compare

import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.dayOf
import io.github.locadani.whichride.crowd.Answer
import io.github.locadani.whichride.crowd.PricePurpose
import io.github.locadani.whichride.personal.Personal
import io.github.locadani.whichride.personal.overrideCards
import io.github.locadani.whichride.personal.withOverride
import io.github.locadani.whichride.prices.PriceLoader
import io.github.locadani.whichride.server.ApiResult
import io.github.locadani.whichride.server.ReportOutcome
import io.github.locadani.whichride.server.Why
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.TestScope
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

@OptIn(ExperimentalCoroutinesApi::class)
class CrowdFlowTest {
    private val dispatcher = StandardTestDispatcher()
    private val launcher = FakeLauncher()
    private val crowd = FakeCrowd()
    private var personal = FakePersonalStore()
    private var loader: PriceLoader = ServerPrices()

    @Before fun main() = Dispatchers.setMain(dispatcher)
    @After fun reset() = Dispatchers.resetMain()

    private fun TestScope.trip(): CompareViewModel = CompareViewModel(
        routes = FakeRoutes(), location = { ALEX }, launcher = launcher, store = FakeStore(), priceLoader = loader,
        hasLocationPermission = { true }, personal = personal, crowd = crowd, today = { TODAY }, newId = { "new" }, now = { NOW },
    ).also {
        advanceUntilIdle()
        it.onShared(COORDINATE_LINK)
        advanceUntilIdle()
    }

    private fun CompareViewModel.addMilesS() {
        addCars(); pickProvider("miles"); toggle(Choice.Offer("miles.s")); commitCars()
    }

    private fun CompareViewModel.milesS() = state.value.quotes.first { it.offerId == "miles.s" }

    @Test fun `a correction applies to me at once and is sent`() = runTest(dispatcher) {
        val vm = trip()
        vm.addMilesS()
        vm.reportPrice()
        assertEquals(Screen.PriceForm, vm.state.value.screen)
        vm.onPriceForm(vm.state.value.priceForm!!.copy(perKm = "0.50"))
        vm.savePrice()
        assertEquals(Screen.Result, vm.state.value.screen)
        assertTrue(vm.milesS().overridden)
        assertEquals(50L, personal.saved.overrideCards("miles.s")!!.single().perKm)
        advanceUntilIdle()
        assertTrue(crowd.reports.single().contains("\"offer\":\"miles.s\""))
        assertEquals(CrowdMessage.SENT, vm.state.value.crowdMessage)
    }

    @Test fun `a correction without a server still applies to me`() = runTest(dispatcher) {
        crowd.reportResult = ApiResult.Unavailable(Why.OFFLINE)
        val vm = trip()
        vm.addMilesS(); vm.reportPrice()
        vm.onPriceForm(vm.state.value.priceForm!!.copy(perKm = "0,50"))
        vm.savePrice(); advanceUntilIdle()
        assertTrue(vm.milesS().overridden)
        assertEquals(CrowdMessage.NOT_SENT, vm.state.value.crowdMessage)
        assertEquals(1, crowd.reports.size)
    }

    @Test fun `a locked offer keeps my price`() = runTest(dispatcher) {
        crowd.reportResult = ApiResult.Refused(409, "offer_locked")
        val vm = trip()
        vm.addMilesS(); vm.reportPrice()
        vm.onPriceForm(vm.state.value.priceForm!!.copy(perKm = "0,50"))
        vm.savePrice(); advanceUntilIdle()
        assertEquals(CrowdMessage.LOCKED, vm.state.value.crowdMessage)
        assertTrue(vm.milesS().overridden)
    }

    @Test fun `when the server says it is already the price my override goes`() = runTest(dispatcher) {
        crowd.reportResult = ApiResult.Ok(ReportOutcome(null, "current"))
        val vm = trip()
        vm.addMilesS(); vm.reportPrice()
        vm.onPriceForm(vm.state.value.priceForm!!.copy(perKm = "0,50"))
        vm.savePrice(); advanceUntilIdle()
        assertEquals(CrowdMessage.ALREADY_CURRENT, vm.state.value.crowdMessage)
        assertNull(personal.saved.overrideCards("miles.s"))
    }

    @Test fun `unchanged values send nothing`() = runTest(dispatcher) {
        val vm = trip()
        vm.addMilesS(); vm.reportPrice(); vm.savePrice(); advanceUntilIdle()
        assertTrue(crowd.reports.isEmpty())
        assertEquals(Screen.Result, vm.state.value.screen)
    }

    @Test fun `the question follows open and return and a yes votes for the pending report`() = runTest(dispatcher) {
        val vm = trip()
        vm.addMilesS()
        vm.openChosen()
        vm.onResume()
        val question = vm.state.value.question!!
        assertEquals(PENDING_REPORT, question.question.reportId)
        assertEquals(89L, question.question.cards.single().perKm)
        assertEquals(dayOf(NOW / 1_000), personal.saved.lastAskedDay)
        vm.answerYes(); advanceUntilIdle()
        assertEquals(listOf(PENDING_REPORT to Answer.YES), crowd.votes)
        assertEquals(QuestionStep.CONFIRMED, vm.state.value.question!!.step)
        vm.closeQuestion()
        assertNull(vm.state.value.question)
    }

    @Test fun `the question is asked at most once a week and never after check in`() = runTest(dispatcher) {
        personal = FakePersonalStore(Personal(lastAskedDay = dayOf(NOW / 1_000) - 3))
        val vm = trip()
        vm.addMilesS(); vm.openChosen(); vm.onResume()
        assertNull(vm.state.value.question)
        personal.saved = personal.saved.copy(lastAskedDay = null)
        val fresh = trip()
        fresh.addCars(); fresh.pickProvider("miles"); fresh.checkIn("miles"); fresh.onResume()
        assertNull(fresh.state.value.question)
    }

    @Test fun `no question about built-in prices or without a server`() = runTest(dispatcher) {
        loader = SeedPrices()
        val vm = trip()
        vm.addMilesS(); vm.openChosen(); vm.onResume()
        assertNull(vm.state.value.question)
    }

    @Test fun `no then I don't know votes no on the pending report`() = runTest(dispatcher) {
        val vm = trip()
        vm.addMilesS(); vm.openChosen(); vm.onResume()
        vm.answerNo()
        assertEquals(QuestionStep.FIX, vm.state.value.question!!.step)
        assertEquals("0.89", vm.state.value.priceForm!!.perKm.replace(',', '.'))
        vm.dontKnow(); advanceUntilIdle()
        assertEquals(listOf(PENDING_REPORT to Answer.NO), crowd.votes)
        assertNull(vm.state.value.question)
    }

    @Test fun `a correction from the question is a report and my price`() = runTest(dispatcher) {
        val vm = trip()
        vm.addMilesS(); vm.openChosen(); vm.onResume(); vm.answerNo()
        vm.onPriceForm(vm.state.value.priceForm!!.copy(perKm = "0,79"))
        vm.savePrice(); advanceUntilIdle()
        assertEquals(QuestionStep.CORRECTED, vm.state.value.question!!.step)
        assertEquals(79L, personal.saved.overrideCards("miles.s")!!.single().perKm)
        assertEquals(1, crowd.reports.size)
        assertNull(vm.state.value.crowdMessage)
    }

    @Test fun `a current price that is stale is confirmed, not voted`() = runTest(dispatcher) {
        loader = ServerPrices(serverCatalogue(offers = "[]"))
        val vm = trip()
        vm.addMilesS(); vm.openChosen(); vm.onResume()
        assertNull(vm.state.value.question!!.question.reportId)
        vm.answerYes(); advanceUntilIdle()
        assertEquals(listOf("miles.s" to Answer.YES), crowd.confirmations)
    }

    @Test fun `a proposal is sent and the prices reloaded`() = runTest(dispatcher) {
        val server = ServerPrices().also { loader = it }
        val vm = trip()
        vm.addCars(); vm.pickProvider("bolt")
        vm.proposeCar("bolt")
        assertEquals(PricePurpose.Propose("bolt"), vm.state.value.priceForm!!.purpose)
        vm.onPriceForm(vm.state.value.priceForm!!.copy(name = "Fiat 500e", unlock = "1", perKm = "0,25", perMinute = "0,10"))
        vm.savePrice(); advanceUntilIdle()
        assertTrue(crowd.reports.single().contains("\"provider\":\"bolt\""))
        assertEquals(CrowdMessage.PROPOSED, vm.state.value.crowdMessage)
        assertEquals(Screen.Cars("bolt"), vm.state.value.screen)
        assertEquals(2, server.loads)
    }

    @Test fun `a proposal question offers no correction`() = runTest(dispatcher) {
        val proposal = """[{"report":"$PROPOSAL_REPORT","provider":"bolt","name":"Fiat 500e","cards":[$CARD_89]}]"""
        loader = ServerPrices(serverCatalogue(offers = "[]", proposals = proposal))
        val vm = trip()
        vm.addCars(); vm.pickProvider("bolt"); vm.toggle(Choice.Offer("proposal.$PROPOSAL_REPORT")); vm.commitCars()
        vm.openChosen(); vm.onResume()
        assertTrue(vm.state.value.question!!.isProposal)
        vm.answerNo(); advanceUntilIdle()
        assertEquals(listOf(PROPOSAL_REPORT to Answer.NO), crowd.votes)
        assertNull(vm.state.value.question)
    }

    @Test fun `my override goes once the shared price equals it`() = runTest(dispatcher) {
        val shared = io.github.locadani.whichride.core.Prices.fromServerResponse(serverCatalogue()).cards("miles.s")
        personal = FakePersonalStore(Personal().withOverride("miles.s", shared))
        trip()
        assertNull(personal.saved.overrideCards("miles.s"))
        assertNotNull(shared.firstOrNull())
    }
}
```

  (`ServerPrices.offline()` is `CACHED` and `load()` is `SERVER`, as a phone with a reachable
  server sees it. `0.89` is the pending report's per-km price; `miles.s`'s shared price is the
  seed's.)

- [ ] **Step 3: Run** `./gradlew :app:testDebugUnitTest --tests '*CrowdFlowTest*'` — Expected: FAIL to compile.

- [ ] **Step 4: State.** `Screen.kt`: `data object PriceForm : Screen`. `UiState.kt`:

```kotlin
enum class QuestionStep { ASK, FIX, CONFIRMED, CORRECTED, NOT_SENT }

/** The §7.2 sheet: what is asked, about which car, and how far the user got. */
data class QuestionState(val question: Question, val providerId: String, val offerName: String, val step: QuestionStep = QuestionStep.ASK) {
    val isProposal: Boolean get() = question.offerId.startsWith(PROPOSAL_PREFIX)
}

/** What happened to a report or proposal the user sent (R11). */
enum class CrowdMessage { SENT, PROPOSED, NOT_SENT, LOCKED, ALREADY_CURRENT }
```

  and in `UiState` after `usedOnce`:

```kotlin
    // Crowd (§7, §5.1)
    /** A server is configured, so reports and questions can be sent. */
    val crowdEnabled: Boolean = false,
    val priceForm: PriceForm? = null,
    val question: QuestionState? = null,
    val crowdMessage: CrowdMessage? = null,
```

  plus, at the end of the file:

```kotlin
/** The option "Price wrong? Tell us" is about: the selected one, else the cheapest. */
fun UiState.reportTarget(): Quote? {
    val ranked = options?.ranked.orEmpty()
    return (ranked.firstOrNull { it.choice == selected } ?: ranked.firstOrNull())?.quote
}

/** Reports are possible for an added, accepted, unlocked offer (R9, R14). */
fun UiState.canReport(): Boolean = reportTarget()?.let { !it.proposed && it.offerId !in lockedOffers } == true
```

  with `val lockedOffers: Set<String> = emptySet()` added to `UiState` (filled in `usePrices`
  from `prices.isLocked` for every quoted offer).

- [ ] **Step 5: ViewModel.** Constructor gains `private val crowd: Crowd, private val crowdEnabled: Boolean = true,`
  after `personal`; the initial state sets `crowdEnabled = crowdEnabled`. Fields:

```kotlin
    /** The offer the user opened with Open <provider>; the question comes when they return (R12). */
    private var opened: QuestionTarget? = null

    private data class QuestionTarget(val offerId: String, val providerId: String, val offerName: String)
```

  In `openChosen()`, after `launcher.openProvider(…)`:

```kotlin
        if (crowdEnabled && state.value.priceSource == PriceSource.SERVER) {
            opened = QuestionTarget(chosen.quote.offerId, provider.id, chosen.quote.offerName)
        }
```

  At the end of `onResume()`:

```kotlin
        opened?.let { target ->
            opened = null
            ask(target)
        }
```

  `usePrices(...)`: after setting `prices`, when the source is `SERVER` drop overrides the crowd
  caught up with, and record locks:

```kotlin
        if (loaded.second == PriceSource.SERVER) {
            val caughtUp = state.value.personal.droppingOverridesEqualTo { loaded.first.cards(it) }
            if (caughtUp != state.value.personal) {
                personal.save(caughtUp)
                mutableState.update { it.copy(personal = caughtUp) }
            }
        }
        val offers = loaded.first.providers().flatMap { p -> p.offers.map { it.id } }
        mutableState.update { it.copy(lockedOffers = offers.filterTo(mutableSetOf()) { id -> loaded.first.isLocked(id) }) }
```

  New section:

```kotlin
    // --- Crowd (§7, §5.1) --------------------------------------------------------------

    /** "Price wrong? Tell us": the price form for the option on screen (R9). */
    fun reportPrice() {
        if (!state.value.canReport()) return
        val quote = state.value.reportTarget() ?: return
        val purpose = PricePurpose.Report(quote.offerId, quote.providerId, quote.offerName)
        mutableState.update { it.copy(priceForm = priceForm(purpose, currentCards(quote.offerId), Locale.getDefault())) }
        push(Screen.PriceForm)
    }

    /** "Car not listed? Add it" (§5.1, R14). */
    fun proposeCar(providerId: String) {
        mutableState.update { it.copy(priceForm = PriceForm(PricePurpose.Propose(providerId))) }
        push(Screen.PriceForm)
    }

    fun onPriceForm(form: PriceForm) = mutableState.update { it.copy(priceForm = form.copy(invalid = emptySet())) }

    fun savePrice() {
        val form = state.value.priceForm ?: return
        val question = state.value.question?.takeIf { it.step == QuestionStep.FIX }
        val current = question?.question?.cards
            ?: (form.purpose as? PricePurpose.Report)?.let { currentCards(it.offerId) }
            ?: emptyList()
        val template = (form.purpose as? PricePurpose.Propose)?.let { template(it.providerId) }
        when (val built = form.build(current, template)) {
            is Built.Invalid -> mutableState.update { it.copy(priceForm = form.copy(invalid = built.fields)) }
            Built.Unchanged -> if (question != null) dontKnow() else closeForm()
            is Built.Report -> {
                val offerId = (form.purpose as PricePurpose.Report).offerId
                savePersonal(state.value.personal.withOverride(offerId, built.cards))
                if (question != null) {
                    mutableState.update { it.copy(priceForm = null, question = question.copy(step = QuestionStep.CORRECTED)) }
                } else {
                    closeForm()
                }
                send(built.body, offerId, quiet = question != null)
            }
            is Built.Proposal -> {
                closeForm()
                send(built.body, offerId = null, quiet = false)
            }
        }
    }

    /** "Use the shared price" (R10). */
    fun resetPrice(offerId: String) = savePersonal(state.value.personal.withoutOverride(offerId))

    fun answerYes() {
        val asked = state.value.question ?: return
        mutableState.update { it.copy(question = asked.copy(step = QuestionStep.CONFIRMED)) }
        viewModelScope.launch {
            val reportId = asked.question.reportId
            val result = if (reportId != null) crowd.vote(reportId, Answer.YES) else crowd.confirm(asked.question.offerId, Answer.YES)
            if (result is ApiResult.Ok) refreshPrices()
            else mutableState.update { it.copy(question = it.question?.copy(step = QuestionStep.NOT_SENT)) }
        }
    }

    /** "No, fix it"; for a proposal, a plain no (R13). */
    fun answerNo() {
        val asked = state.value.question ?: return
        if (asked.isProposal) return dontKnow()
        val purpose = PricePurpose.Report(asked.question.offerId, asked.providerId, asked.offerName)
        mutableState.update {
            it.copy(question = asked.copy(step = QuestionStep.FIX), priceForm = priceForm(purpose, asked.question.cards, Locale.getDefault()))
        }
    }

    /** "I don't know the numbers": a no on a pending report, nothing on a current price (decision 36). */
    fun dontKnow() {
        val asked = state.value.question ?: return
        asked.question.reportId?.let { report -> viewModelScope.launch { crowd.vote(report, Answer.NO) } }
        closeQuestion()
    }

    /** Skip and Close. */
    fun closeQuestion() = mutableState.update { it.copy(question = null, priceForm = null) }

    fun crowdMessageShown() = mutableState.update { it.copy(crowdMessage = null) }

    private fun ask(target: QuestionTarget) {
        val prices = prices ?: return
        val nowSeconds = now() / 1_000
        val question = prices.question(target.offerId, nowSeconds, state.value.personal.lastAskedDay) ?: return
        savePersonal(state.value.personal.copy(lastAskedDay = dayOf(nowSeconds)))
        mutableState.update { it.copy(question = QuestionState(question, target.providerId, target.offerName)) }
    }

    private fun send(body: String, offerId: String?, quiet: Boolean) {
        viewModelScope.launch {
            val result = crowd.report(body)
            val message = when {
                result is ApiResult.Ok && result.value.status == "current" -> {
                    offerId?.let { savePersonal(state.value.personal.withoutOverride(it)) }
                    CrowdMessage.ALREADY_CURRENT
                }
                result is ApiResult.Ok -> if (offerId == null) CrowdMessage.PROPOSED else CrowdMessage.SENT
                result is ApiResult.Refused && result.code == "offer_locked" -> CrowdMessage.LOCKED
                else -> CrowdMessage.NOT_SENT
            }
            if (!(quiet && message == CrowdMessage.SENT)) mutableState.update { it.copy(crowdMessage = message) }
            if (result is ApiResult.Ok) refreshPrices()
        }
    }

    private fun refreshPrices() {
        viewModelScope.launch {
            val fresh = priceLoader.load()
            if (fresh.source == PriceSource.SERVER) usePrices(fresh.prices to fresh.source)
        }
    }

    private fun closeForm() {
        mutableState.update { it.copy(priceForm = null) }
        if (state.value.screen == Screen.PriceForm) pop()
    }

    /** The cards the user sees for an offer: their own, else the shared ones. */
    private fun currentCards(offerId: String) =
        state.value.personal.overrideCards(offerId) ?: prices?.cards(offerId).orEmpty()

    /** The provider's first standard card: a proposal copies its label and rounding (R14). */
    private fun template(providerId: String): RateCard? {
        val prices = prices ?: return null
        return state.value.providers.firstOrNull { it.id == providerId }?.offers
            ?.firstNotNullOfOrNull { offer -> prices.cards(offer.id).firstOrNull { it.kind == CardKind.STANDARD } }
    }
```

  `back()` from `Screen.PriceForm` clears `priceForm`. In `offerIds(providerId)` include the
  provider's quoted offers so proposals are part of its choices:
  `(provider offers ids) + state.value.quotes.filter { it.providerId == providerId }.map { it.offerId }`.

- [ ] **Step 6: Wiring.** `AppContainer`: `crowd = ServerCrowd(api, installs), crowdEnabled = BuildConfig.SERVER_URL.isNotEmpty(),`.
  Existing tests' factories pass `crowd = FakeCrowd()`.

- [ ] **Step 7: Run** `./gradlew :app:testDebugUnitTest` — Expected: all pass (14 new).

- [ ] **Step 8: Commit** — `feat(android): price reports, the confirmation question and proposals`.

### Task 12: Screens for reports, the question and proposals

**Files:**
- Create: `ui/PriceFormScreen.kt`, `ui/QuestionSheet.kt`
- Modify: `ui/ResultScreen.kt` ("Price wrong? Tell us", `ResultActions.onReport`), `ui/CarsScreen.kt`
  (proposals section, "Car not listed? Add it"), `ui/ProvidersScreen.kt` (lowest price without
  proposals), `ui/Text.kt` (`optionMeta`), `ui/WhichrideNav.kt`, `res/values/strings.xml`
- Test: `ui/PriceFormScreenTest.kt`, `ui/QuestionSheetTest.kt` (new), `ui/CarsScreenTest.kt`, `ui/ResultScreenTest.kt`

**Interfaces:**
- Consumes: Task 11's state and methods.
- Produces: `PriceFormScreen(state, onBack, onChange: (PriceForm) -> Unit, onSave, onReset: (offerId) -> Unit)`,
  `PriceFields(form, onChange)`, `QuestionContent(state, actions: QuestionActions)`, `QuestionSheet(state, actions)`,
  `QuestionActions(onYes, onNo, onDontKnow, onClose, onChange, onSave)`.

- [ ] **Step 1: Strings** (add to `values/strings.xml`):

```xml
    <string name="report_price">Price wrong? Tell us</string>
    <string name="your_price">your price</string>
    <string name="proposed_meta">added by a user, not confirmed</string>
    <string name="price_title">What does %1$s show?</string>
    <string name="price_body">Change only what\'s different. %1$s, %2$s.</string>
    <string name="propose_title">Add a %1$s car</string>
    <string name="propose_body">The name as %1$s shows it, and its prices for pay-as-you-go trips.</string>
    <string name="field_name">Car name</string>
    <string name="field_unlock">Unlock (€)</string>
    <string name="field_per_km">Per km (€)</string>
    <string name="field_per_minute">Per minute (€)</string>
    <string name="field_minimum">Minimum (€)</string>
    <string name="field_invalid_euros">Up to %1$s, e.g. 0,29</string>
    <string name="field_invalid_name">1 to %1$d characters</string>
    <string name="price_note">Your numbers apply to you right away. Others see them once another user confirms.</string>
    <string name="propose_note">Others see this car once another user confirms it.</string>
    <string name="price_save">Save correction</string>
    <string name="propose_save">Send car</string>
    <string name="price_reset">Use the shared price</string>

    <string name="question_back_from">Back from %1$s</string>
    <string name="question_title">Is this the price %1$s showed for the %2$s?</string>
    <string name="question_title_proposal">Did %1$s show the %2$s at these prices?</string>
    <string name="question_per_km">per km</string>
    <string name="question_per_minute">per min</string>
    <string name="question_unlock">unlock</string>
    <string name="question_yes">Yes, that\'s right</string>
    <string name="question_no_fix">No, fix it</string>
    <string name="question_no">No</string>
    <string name="question_skip">Skip</string>
    <string name="question_note">We ask at most once a week. Your answer keeps prices right for everyone.</string>
    <string name="question_dont_know">I don\'t know the numbers</string>
    <string name="question_thanks">Thanks!</string>
    <string name="question_confirmed">Price confirmed. Thanks for keeping it right.</string>
    <string name="question_corrected">Your prices apply to you now. Others see them once someone confirms.</string>
    <string name="question_not_sent">We couldn\'t send your answer now. Nothing changed.</string>

    <string name="proposals_title">Added by users, not confirmed yet</string>
    <string name="car_not_listed">Car not listed? Add it</string>

    <string name="crowd_sent">Thanks! Others see it once someone confirms.</string>
    <string name="crowd_proposed">Thanks! The car appears for others once someone confirms it.</string>
    <string name="crowd_not_sent">Saved for you; we couldn\'t send it now.</string>
    <string name="crowd_locked">This price is locked for now; your numbers still apply to you.</string>
    <string name="crowd_already_current">That\'s already the price everyone sees.</string>
```

- [ ] **Step 2: Write the failing UI tests.** `QuestionSheetTest.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.QuestionState
import io.github.locadani.whichride.compare.QuestionStep
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.CardKind
import io.github.locadani.whichride.core.Question
import io.github.locadani.whichride.core.RateCard
import io.github.locadani.whichride.core.Rounding
import io.github.locadani.whichride.crowd.PriceForm
import io.github.locadani.whichride.crowd.PricePurpose
import io.github.locadani.whichride.designsystem.WhichrideTheme
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class QuestionSheetTest {
    @get:Rule val rule = createComposeRule()
    private val card = RateCard("Standard", CardKind.STANDARD, 100, 29, 11, 0u, 0u, 0, Rounding.STARTED_UNIT, Rounding.STARTED_UNIT)
    private fun state(offerId: String = "bolt.polo", step: QuestionStep = QuestionStep.ASK) = UiState(
        providers = PROVIDERS,
        question = QuestionState(Question(offerId, null, listOf(card)), "bolt", "VW Polo", step),
        priceForm = if (step == QuestionStep.FIX) PriceForm(PricePurpose.Report(offerId, "bolt", "VW Polo"), unlock = "1", perKm = "0.29", perMinute = "0.11", minimum = "0") else null,
    )

    @Test fun `the question shows the values and the three answers`() {
        val pressed = mutableListOf<String>()
        rule.setContent {
            WhichrideTheme {
                QuestionContent(state(), QuestionActions(onYes = { pressed += "yes" }, onNo = { pressed += "no" }, onClose = { pressed += "skip" }))
            }
        }
        rule.onNodeWithText("Is this the price Bolt Drive showed for the VW Polo?").assertExists()
        rule.onNodeWithText("€0.29").assertExists()
        rule.onNodeWithText("€0.11").assertExists()
        rule.onNodeWithText("€1.00").assertExists()
        rule.onNodeWithText("Yes, that's right").performClick()
        rule.onNodeWithText("No, fix it").performClick()
        rule.onNodeWithText("Skip").performClick()
        assertEquals(listOf("yes", "no", "skip"), pressed)
    }

    @Test fun `a proposal is a plain yes or no`() {
        rule.setContent { WhichrideTheme { QuestionContent(state(offerId = "proposal.x"), QuestionActions()) } }
        rule.onNodeWithText("Did Bolt Drive show the VW Polo at these prices?").assertExists()
        rule.onNodeWithText("No").assertExists()
        rule.onNodeWithText("No, fix it").assertDoesNotExist()
    }

    @Test fun `fixing offers the form and I don't know`() {
        var dontKnow = false
        rule.setContent { WhichrideTheme { QuestionContent(state(step = QuestionStep.FIX), QuestionActions(onDontKnow = { dontKnow = true })) } }
        rule.onNodeWithText("What does Bolt Drive show?").assertExists()
        rule.onNodeWithText("Per km (€)").assertExists()
        rule.onNodeWithText("I don't know the numbers").performClick()
        assertEquals(true, dontKnow)
    }

    @Test fun `the end says what happened`() {
        rule.setContent { WhichrideTheme { QuestionContent(state(step = QuestionStep.NOT_SENT), QuestionActions()) } }
        rule.onNodeWithText("We couldn't send your answer now. Nothing changed.").assertExists()
        rule.onNodeWithText("Close").assertExists()
    }
}
```

  `PriceFormScreenTest.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.Screen
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.crowd.PriceField
import io.github.locadani.whichride.crowd.PriceForm
import io.github.locadani.whichride.crowd.PricePurpose
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.core.Options
import io.github.locadani.whichride.core.RankedOption
import io.github.locadani.whichride.core.Choice
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

@RunWith(AndroidJUnit4::class)
class PriceFormScreenTest {
    @get:Rule val rule = createComposeRule()

    @Test fun `a report marks out-of-range values and offers the shared price back`() {
        var reset: String? = null
        val mine = quote("bolt", "bolt.polo", "VW Polo", 912).copy(overridden = true)
        val state = UiState(
            stack = listOf(Screen.Home, Screen.Result, Screen.PriceForm),
            providers = PROVIDERS,
            quotes = listOf(mine),
            priceForm = PriceForm(PricePurpose.Report("bolt.polo", "bolt", "VW Polo"), perKm = "9", invalid = setOf(PriceField.PER_KM)),
        )
        rule.setContent { WhichrideTheme { PriceFormScreen(state, {}, {}, {}, { reset = it }) } }
        rule.onNodeWithText("What does Bolt Drive show?").assertExists()
        rule.onNodeWithText("Up to €5.00, e.g. 0,29").assertExists()
        rule.onNodeWithText("Use the shared price").performClick()
        assertEquals("bolt.polo", reset)
    }

    @Test fun `a proposal asks for the car's name`() {
        val state = UiState(providers = PROVIDERS, priceForm = PriceForm(PricePurpose.Propose("bolt")))
        rule.setContent { WhichrideTheme { PriceFormScreen(state, {}, {}, {}, {}) } }
        rule.onNodeWithText("Add a Bolt Drive car").assertExists()
        rule.onNodeWithText("Car name").assertExists()
        rule.onNodeWithText("Send car").assertExists()
        rule.onNodeWithText("Use the shared price").assertDoesNotExist()
    }
}
```

  `CarsScreenTest.kt` — add (keep the file's existing setup; the new parameter `onPropose`
  goes last in `CarsScreen`'s signature):

```kotlin
    @Test fun `proposals are listed apart and a missing car can be proposed`() {
        var proposed: String? = null
        val proposal = quote("bolt", "proposal.x", "Fiat 500e", 700).copy(proposed = true)
        val state = resultState().copy(stack = listOf(Screen.Home, Screen.Result, Screen.Providers, Screen.Cars("bolt")), quotes = QUOTES + proposal, crowdEnabled = true)
        rule.setContent { WhichrideTheme { CarsScreen(state, "bolt", {}, {}, {}, {}, {}, onPropose = { proposed = it }) } }
        rule.onNodeWithText("Added by users, not confirmed yet").assertExists()
        rule.onNodeWithText("Fiat 500e").assertExists()
        rule.onNodeWithText("from €7.90").assertExists() // any car ignores the cheaper proposal
        rule.onNodeWithText("Car not listed? Add it").performScrollTo().performClick()
        assertEquals("bolt", proposed)
    }
```

  `ResultScreenTest.kt` — add:

```kotlin
    @Test fun `price wrong is offered for accepted offers only`() {
        var reported = false
        val polo = RankedOption(Choice.Offer("bolt.polo"), QUOTES[2])
        rule.setContent { WhichrideTheme { ResultScreen(resultState(listOf(polo)), ResultActions(onReport = { reported = true })) } }
        rule.onNodeWithText("Price wrong? Tell us").performScrollTo().performClick()
        assertEquals(true, reported)
    }

    @Test fun `no price wrong for a proposal`() {
        val proposal = RankedOption(Choice.Offer("proposal.x"), quote("bolt", "proposal.x", "Fiat 500e", 700).copy(proposed = true))
        rule.setContent { WhichrideTheme { ResultScreen(resultState(listOf(proposal)), ResultActions()) } }
        rule.onNodeWithText("Price wrong? Tell us").assertDoesNotExist()
        rule.onNodeWithText("added by a user, not confirmed", substring = true).assertExists()
    }
```

- [ ] **Step 3: Run** the four test classes — Expected: FAIL to compile.

- [ ] **Step 4: `PriceFormScreen.kt`**

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.KeyboardCapitalization
import androidx.compose.ui.text.input.KeyboardType
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.core.reportLimits
import io.github.locadani.whichride.crowd.PriceField
import io.github.locadani.whichride.crowd.PriceForm
import io.github.locadani.whichride.crowd.PricePurpose
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens

private val Space = WhichrideTokens.Space

/** "Price wrong? Tell us" (§7.1) and "Car not listed? Add it" (§5.1): one form, two purposes. */
@Composable
fun PriceFormScreen(state: UiState, onBack: () -> Unit, onChange: (PriceForm) -> Unit, onSave: () -> Unit, onReset: (String) -> Unit) {
    val form = state.priceForm ?: return
    val colors = WhichrideTheme.colors
    val purpose = form.purpose
    val provider = state.providers.nameOf(
        when (purpose) {
            is PricePurpose.Report -> purpose.providerId
            is PricePurpose.Propose -> purpose.providerId
        },
    )
    Column(Modifier.fillMaxSize().imePadding()) {
        TopBar(onBack, close = true)
        Column(
            Modifier.weight(1f).verticalScroll(rememberScrollState()).padding(horizontal = Space.md),
            verticalArrangement = Arrangement.spacedBy(Space.md),
        ) {
            when (purpose) {
                is PricePurpose.Report -> {
                    Title(stringResource(R.string.price_title, provider))
                    Text(stringResource(R.string.price_body, purpose.offerName, provider), style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceMuted)
                }
                is PricePurpose.Propose -> {
                    Title(stringResource(R.string.propose_title, provider))
                    Text(stringResource(R.string.propose_body, provider), style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceMuted)
                }
            }
            PriceFields(form, onChange)
            Text(
                stringResource(if (purpose is PricePurpose.Propose) R.string.propose_note else R.string.price_note),
                style = MaterialTheme.typography.bodySmall,
                color = colors.onSurfaceMuted,
            )
            if (purpose is PricePurpose.Report && state.quotes.any { it.offerId == purpose.offerId && it.overridden }) {
                TextButton(onClick = { onReset(purpose.offerId) }) { Text(stringResource(R.string.price_reset), color = colors.accentText) }
            }
        }
        PrimaryButton(
            stringResource(if (purpose is PricePurpose.Propose) R.string.propose_save else R.string.price_save),
            onSave,
            Modifier.padding(Space.md),
            enabled = !form.sending,
        )
    }
}

@Composable
private fun Title(text: String) =
    Text(text, style = MaterialTheme.typography.headlineSmall, modifier = Modifier.semantics { heading() })

/** The name (proposals only) and the four prices, each marked when out of range. */
@Composable
fun PriceFields(form: PriceForm, onChange: (PriceForm) -> Unit) {
    val limits = reportLimits()
    Column(verticalArrangement = Arrangement.spacedBy(Space.xs)) {
        if (form.purpose is PricePurpose.Propose) {
            OutlinedTextField(
                value = form.name,
                onValueChange = { onChange(form.copy(name = it)) },
                label = { Text(stringResource(R.string.field_name)) },
                isError = PriceField.NAME in form.invalid,
                supportingText = if (PriceField.NAME in form.invalid) ({ Text(stringResource(R.string.field_invalid_name, limits.maxNameChars.toInt())) }) else null,
                keyboardOptions = KeyboardOptions(capitalization = KeyboardCapitalization.Words),
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
        }
        Euros(R.string.field_unlock, form.unlock, PriceField.UNLOCK in form.invalid, limits.maxBaseFee) { onChange(form.copy(unlock = it)) }
        Euros(R.string.field_per_km, form.perKm, PriceField.PER_KM in form.invalid, limits.maxPerUnit) { onChange(form.copy(perKm = it)) }
        Euros(R.string.field_per_minute, form.perMinute, PriceField.PER_MINUTE in form.invalid, limits.maxPerUnit) { onChange(form.copy(perMinute = it)) }
        Euros(R.string.field_minimum, form.minimum, PriceField.MINIMUM in form.invalid, limits.maxMinimum) { onChange(form.copy(minimum = it)) }
    }
}

@Composable
private fun Euros(label: Int, value: String, invalid: Boolean, max: Long, onValue: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onValue,
        label = { Text(stringResource(label)) },
        isError = invalid,
        supportingText = if (invalid) ({ Text(stringResource(R.string.field_invalid_euros, money(max))) }) else null,
        textStyle = WhichrideTheme.numbers.medium,
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
    )
}
```

  (`PrimaryButton` needs an `enabled` parameter: it already has one.)

- [ ] **Step 5: `QuestionSheet.kt`**

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Check
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import io.github.locadani.whichride.R
import io.github.locadani.whichride.compare.QuestionStep
import io.github.locadani.whichride.compare.UiState
import io.github.locadani.whichride.crowd.PriceForm
import io.github.locadani.whichride.crowd.editable
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.designsystem.WhichrideTokens

private val Space = WhichrideTokens.Space

data class QuestionActions(
    val onYes: () -> Unit = {},
    val onNo: () -> Unit = {},
    val onDontKnow: () -> Unit = {},
    /** Skip, Close, or a swipe down. */
    val onClose: () -> Unit = {},
    val onChange: (PriceForm) -> Unit = {},
    val onSave: () -> Unit = {},
)

/** Canvas "Price check" (§7.2) as a bottom sheet over the result. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun QuestionSheet(state: UiState, actions: QuestionActions) {
    if (state.question == null) return
    ModalBottomSheet(
        onDismissRequest = actions.onClose,
        sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
        containerColor = WhichrideTheme.colors.surfaceLow,
        contentColor = WhichrideTheme.colors.onSurface,
        scrimColor = WhichrideTheme.colors.scrim,
    ) { QuestionContent(state, actions) }
}

@Composable
fun QuestionContent(state: UiState, actions: QuestionActions) {
    val asked = state.question ?: return
    val colors = WhichrideTheme.colors
    val provider = state.providers.nameOf(asked.providerId)
    Column(
        Modifier.fillMaxWidth().verticalScroll(rememberScrollState()).padding(start = Space.lg, end = Space.lg, bottom = Space.xl),
        verticalArrangement = Arrangement.spacedBy(Space.md),
    ) {
        when (asked.step) {
            QuestionStep.ASK -> {
                Text(stringResource(R.string.question_back_from, provider), style = WhichrideTheme.numbers.small, color = colors.onSurfaceMuted)
                Text(
                    stringResource(if (asked.isProposal) R.string.question_title_proposal else R.string.question_title, provider, asked.offerName),
                    style = MaterialTheme.typography.headlineSmall,
                    modifier = Modifier.semantics { heading() },
                )
                asked.question.cards.editable()?.let { card ->
                    Row(horizontalArrangement = Arrangement.spacedBy(Space.xs)) {
                        Value(stringResource(R.string.question_per_km), money(card.perKm), Modifier.weight(1f))
                        Value(stringResource(R.string.question_per_minute), money(card.perMinute), Modifier.weight(1f))
                        Value(stringResource(R.string.question_unlock), money(card.baseFee), Modifier.weight(1f))
                    }
                }
                PrimaryButton(stringResource(R.string.question_yes), actions.onYes)
                Row(horizontalArrangement = Arrangement.spacedBy(Space.xs)) {
                    SecondaryButton(stringResource(if (asked.isProposal) R.string.question_no else R.string.question_no_fix), actions.onNo, Modifier.weight(1f))
                    TextButton(onClick = actions.onClose, modifier = Modifier.weight(1f)) {
                        Text(stringResource(R.string.question_skip), color = colors.onSurfaceMuted, style = MaterialTheme.typography.labelLarge)
                    }
                }
                Text(stringResource(R.string.question_note), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
            }
            QuestionStep.FIX -> {
                Text(stringResource(R.string.price_title, provider), style = MaterialTheme.typography.headlineSmall, modifier = Modifier.semantics { heading() })
                Text(stringResource(R.string.price_body, asked.offerName, provider), style = MaterialTheme.typography.bodyMedium, color = colors.onSurfaceMuted)
                state.priceForm?.let { PriceFields(it, actions.onChange) }
                Text(stringResource(R.string.price_note), style = MaterialTheme.typography.bodySmall, color = colors.onSurfaceMuted)
                PrimaryButton(stringResource(R.string.price_save), actions.onSave)
                TextButton(onClick = actions.onDontKnow, modifier = Modifier.fillMaxWidth()) {
                    Text(stringResource(R.string.question_dont_know), color = colors.onSurfaceMuted)
                }
            }
            QuestionStep.CONFIRMED, QuestionStep.CORRECTED, QuestionStep.NOT_SENT -> {
                Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(Space.sm)) {
                    if (asked.step != QuestionStep.NOT_SENT) {
                        Box(Modifier.size(WhichrideTokens.Size.controlLg).clip(CircleShape).background(colors.accent), contentAlignment = Alignment.Center) {
                            Icon(Icons.Filled.Check, contentDescription = null, tint = colors.onAccent)
                        }
                        Text(stringResource(R.string.question_thanks), style = MaterialTheme.typography.titleLarge, modifier = Modifier.semantics { heading() })
                    }
                    Text(
                        stringResource(
                            when (asked.step) {
                                QuestionStep.CONFIRMED -> R.string.question_confirmed
                                QuestionStep.CORRECTED -> R.string.question_corrected
                                else -> R.string.question_not_sent
                            },
                        ),
                        style = MaterialTheme.typography.bodyMedium,
                        color = colors.onSurfaceMuted,
                        textAlign = TextAlign.Center,
                    )
                }
                SecondaryButton(stringResource(R.string.close), actions.onClose)
            }
        }
    }
}

@Composable
private fun Value(label: String, value: String, modifier: Modifier) {
    Surface(modifier, shape = MaterialTheme.shapes.large, color = WhichrideTheme.colors.surface) {
        Column(Modifier.padding(Space.sm), verticalArrangement = Arrangement.spacedBy(Space.xxs)) {
            Text(label, style = MaterialTheme.typography.labelSmall, color = WhichrideTheme.colors.onSurfaceMuted)
            Text(value, style = WhichrideTheme.numbers.medium)
        }
    }
}
```

  Add the pairs the sheet introduces (`on-surface`/`surface-low`, `on-surface-muted`/`surface-low`)
  to `design/contrast.json` if they are not listed, and run `cd design && npm test`.

- [ ] **Step 6: Result, Cars, Providers, labels.**
  - `ResultActions` gains `val onReport: () -> Unit = {}`. In `ResultScreen`'s `LazyColumn`, after
    the `CheaperLine` item:

```kotlin
                if (state.canReport()) {
                    item {
                        TextButton(onClick = actions.onReport) {
                            Text(stringResource(R.string.report_price), color = WhichrideTheme.colors.accentText)
                        }
                    }
                }
```

  - `optionMeta` in `Text.kt`: first item `if (option.quote.proposed) add(stringResource(R.string.proposed_meta))`;
    after the package item `if (option.quote.overridden) add(stringResource(R.string.your_price))`.
  - `CarsScreen`: add the parameter `onPropose: (String) -> Unit` last. Split the provider's quotes:
    `val quotes = state.quotes.filter { it.providerId == providerId && !it.proposed }` (used for
    "Any car", its "from" price and the main list, as before) and
    `val proposals = state.quotes.filter { it.providerId == providerId && it.proposed }`. After the
    main `items(...)` (and only when `terms.isEmpty()`):

```kotlin
            if (proposals.isNotEmpty() && terms.isEmpty()) {
                item {
                    Text(
                        stringResource(R.string.proposals_title),
                        style = MaterialTheme.typography.labelMedium,
                        color = colors.onSurfaceMuted,
                        modifier = Modifier.padding(top = Space.sm).semantics { heading() },
                    )
                }
                items(proposals, key = { it.offerId }) { quote ->
                    val choice = Choice.Offer(quote.offerId)
                    CarRow(quote.offerName, stringResource(R.string.proposed_meta), money(quote.totalCents), choice in state.draft, strong = false) { onToggle(choice) }
                }
            }
            if (state.crowdEnabled) {
                item {
                    OutlinedButton(
                        onClick = { onPropose(providerId) },
                        modifier = Modifier.fillMaxWidth().heightIn(min = WhichrideTokens.Size.touchTarget).padding(top = Space.xs),
                        shape = MaterialTheme.shapes.medium,
                        border = BorderStroke(WhichrideTokens.Size.stroke, colors.outline),
                    ) { Text(stringResource(R.string.car_not_listed), color = colors.onSurfaceMuted) }
                }
            }
```

    (A proposal is never covered by "Any <provider> car": the any-car tick does not tick it.)
  - `ProvidersScreen`: `val quotes = state.quotes.filter { it.providerId == provider.id && !it.proposed }`.

- [ ] **Step 7: Navigation.** In `WhichrideNav`:
  - `Screen.PriceForm -> PriceFormScreen(state, onBack = { viewModel.back() }, viewModel::onPriceForm, viewModel::savePrice) { viewModel.resetPrice(it); viewModel.back() }`;
  - `CarsScreen(…, onPropose = viewModel::proposeCar)`; `ResultActions(…, onReport = viewModel::reportPrice)`;
  - after the `when`, inside the `Box`:
    `QuestionSheet(state, QuestionActions(viewModel::answerYes, viewModel::answerNo, viewModel::dontKnow, viewModel::closeQuestion, viewModel::onPriceForm, viewModel::savePrice))`;
  - crowd messages through the same `SnackbarHostState` as Undo:

```kotlin
    val crowdText = state.crowdMessage?.let { stringResource(it.text()) }
    LaunchedEffect(state.crowdMessage) {
        if (crowdText == null) return@LaunchedEffect
        snackbar.showSnackbar(crowdText)
        viewModel.crowdMessageShown()
    }
```

    with, in `Text.kt`:

```kotlin
fun CrowdMessage.text() = when (this) {
    CrowdMessage.SENT -> R.string.crowd_sent
    CrowdMessage.PROPOSED -> R.string.crowd_proposed
    CrowdMessage.NOT_SENT -> R.string.crowd_not_sent
    CrowdMessage.LOCKED -> R.string.crowd_locked
    CrowdMessage.ALREADY_CURRENT -> R.string.crowd_already_current
}
```

- [ ] **Step 8: Run** `./gradlew :app:testDebugUnitTest && (cd ../design && npm test)` — Expected: all pass.

- [ ] **Step 9: Look at it** on the phone as in Task 8 Step 12: Price wrong? → form → Save
  (expect "Saved for you; we couldn't send it now" until milestone 6 links Play Integrity), the
  row says "your price", Use the shared price restores it. Compare with the canvas's PriceCheck
  artboard by forcing the sheet in a debug session is not needed; its content is covered by
  `QuestionSheetTest`.

- [ ] **Step 10: Commit** — `feat(android): report prices, answer the price check, propose missing cars`.

## Part D — Leftovers, languages, accessibility

### Task 13: The remaining 5a minors

**Files:**
- Modify: `compare/CompareViewModel.kt`, `ui/WhichrideNav.kt`
- Test: `compare/CompareViewModelTest.kt`

**Interfaces:**
- Produces: `CompareViewModel.locationRequestLaunched()`.

- [ ] **Step 1: Write the failing tests** (in `CompareViewModelTest`, using its existing helpers):

```kotlin
    @Test fun `the location request is launched once even if the screen is recreated`() = runTest(dispatcher) {
        permission = false
        val vm = viewModel()
        vm.onShared(coordinateLink)
        advanceUntilIdle()
        assertTrue(vm.state.value.askLocation)
        vm.locationRequestLaunched()
        assertFalse("a recreated screen must not ask again", vm.state.value.askLocation)
        assertEquals(Screen.Resolving, vm.state.value.screen)
        vm.onLocationAnswered()
        advanceUntilIdle()
        assertEquals(Screen.Result, vm.state.value.screen)
    }

    @Test fun `an expanded link that is not a short link says so`() = runTest(dispatcher) {
        routes.expand = ApiResult.Refused(422, "not_a_short_link")
        val vm = viewModel()
        vm.onShared("https://maps.app.goo.gl/x7Qa2Lm")
        advanceUntilIdle()
        assertEquals(Screen.Home, vm.state.value.screen)
        assertEquals(InputError.NOT_A_MAPS_LINK, vm.state.value.inputError)
        routes.expand = ApiResult.Refused(200, "invalid_destination")
        vm.onShared("https://maps.app.goo.gl/x7Qa2Lm")
        advanceUntilIdle()
        assertEquals(InputError.NO_DESTINATION, vm.state.value.inputError)
    }

    @Test fun `the cars screen closes when its provider leaves the catalogue`() = runTest(dispatcher) {
        val vm = viewModel()
        vm.onShared(coordinateLink); advanceUntilIdle()
        vm.addCars(); vm.pickProvider("sixt")
        vm.usePricesForTest(Prices.fromSeed(SEED.replaceSixt()))
        assertEquals(Screen.Providers, vm.state.value.screen)
    }
```

  For the last test, `usePrices` must be reachable: make it `internal` and annotate
  `@VisibleForTesting internal fun usePricesForTest(prices: Prices) = usePrices(prices to PriceSource.SERVER)`,
  and add to `Fakes.kt`:

```kotlin
/** The seed without SIXT share. */
fun String.replaceSixt(): String {
    val catalogue = kotlinx.serialization.json.Json.parseToJsonElement(this).jsonObject
    val providers = catalogue["providers"]!!.jsonArray.filter { it.jsonObject["id"]!!.jsonPrimitive.content != "sixt" }
    return kotlinx.serialization.json.JsonObject(mapOf("providers" to kotlinx.serialization.json.JsonArray(providers))).toString()
}
```

  (imports `kotlinx.serialization.json.jsonArray`, `jsonObject`, `jsonPrimitive`.)

- [ ] **Step 2: Run** — Expected: FAIL (unresolved `locationRequestLaunched`, wrong errors, screen still `Cars`).

- [ ] **Step 3: Implement.**
  - ViewModel: `fun locationRequestLaunched() = mutableState.update { it.copy(askLocation = false) }`
    with doc "The system dialog is up; a recreated screen must not launch it again (5a minor).
    The trip keeps waiting for `onLocationAnswered`." `WhichrideNav`: call
    `viewModel.locationRequestLaunched()` right after `permission.launch(…)`.
    (`ActivityResultRegistry` delivers the answer to the re-registered launcher after rotation.)
  - In `resolve`, `Destination.ShortLink`'s `Refused` branch:

```kotlin
            is ApiResult.Refused -> when (expanded.code) {
                "no_destination", "invalid_destination" -> Resolution.Failed(InputError.NO_DESTINATION, "")
                "not_a_short_link" -> Resolution.Failed(InputError.NOT_A_MAPS_LINK, "")
                else -> Resolution.Unrouted(Target(label, null, MapsTarget.Link(destination.url)), ManualReason.OFFLINE)
            }
```

  - In `usePrices`, after updating `providers`:

```kotlin
        // The provider of an open step 2 left the catalogue (5a minor): back to step 1.
        val screen = state.value.screen
        if (screen is Screen.Cars && state.value.providers.none { it.id == screen.providerId }) {
            mutableState.update { it.copy(stack = it.stack.dropLast(1), draft = emptySet()) }
        }
```

- [ ] **Step 4: Run** `./gradlew :app:testDebugUnitTest` — Expected: all pass.

- [ ] **Step 5: Commit** — `fix(android): ask for location once, map expand errors, leave a vanished provider`.

### Task 14: Italian, German and Turkish

**Files:**
- Create: `android/app/src/main/res/values-it/strings.xml`, `values-de/strings.xml`,
  `values-tr/strings.xml`, `android/app/src/main/res/resources.properties`
- Modify: `android/app/build.gradle.kts`, `values/strings.xml` (`app_name` not translatable)
- Test: `android/app/src/test/kotlin/io/github/locadani/whichride/i18n/TranslationsTest.kt`

- [ ] **Step 1: Write the failing test**

```kotlin
package io.github.locadani.whichride.i18n

import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.w3c.dom.Element

/** Every string in English exists in every language, with the same placeholders (R17). */
class TranslationsTest {
    private val res = File("src/main/res")
    private val placeholder = Regex("""%(\d+\$)?[sd]|%%""")

    /** name → texts (one for a string, one per quantity for plurals); untranslatable ones left out. */
    private fun read(folder: String): Map<String, Map<String, String>> {
        val document = DocumentBuilderFactory.newInstance().newDocumentBuilder().parse(File(res, "$folder/strings.xml"))
        val result = mutableMapOf<String, Map<String, String>>()
        val strings = document.getElementsByTagName("string")
        for (i in 0 until strings.length) {
            val element = strings.item(i) as Element
            if (element.getAttribute("translatable") == "false") continue
            result[element.getAttribute("name")] = mapOf("" to element.textContent)
        }
        val plurals = document.getElementsByTagName("plurals")
        for (i in 0 until plurals.length) {
            val element = plurals.item(i) as Element
            val items = element.getElementsByTagName("item")
            result["plurals:" + element.getAttribute("name")] =
                (0 until items.length).associate { (items.item(it) as Element).let { item -> item.getAttribute("quantity") to item.textContent } }
        }
        return result
    }

    private fun placeholders(text: String) = placeholder.findAll(text).map { it.value }.toSortedSet()

    @Test fun everyLanguageHasEveryStringWithTheSamePlaceholders() {
        val english = read("values")
        for (language in listOf("it", "de", "tr")) {
            val translated = read("values-$language")
            assertEquals("$language: names", english.keys, translated.keys)
            for ((name, texts) in english) {
                val englishPlaceholders = placeholders(texts["other"] ?: texts.getValue(""))
                for ((quantity, text) in translated.getValue(name)) {
                    assertTrue("$language $name[$quantity] is empty", text.isNotBlank())
                    assertEquals("$language $name[$quantity]", englishPlaceholders, placeholders(text).let { found ->
                        // "one" may spell the number out ("un'auto"), so it may drop the count placeholder.
                        if (quantity == "one") found + englishPlaceholders.filter { it.endsWith("d") } else found
                    }.toSortedSet())
                }
                if (name.startsWith("plurals:")) assertTrue("$language $name has other", "other" in translated.getValue(name))
            }
        }
    }
}
```

- [ ] **Step 2: Run** `./gradlew :app:testDebugUnitTest --tests '*TranslationsTest*'` — Expected: FAIL (no `values-it`).

- [ ] **Step 3: Locale configuration.** `values/strings.xml`: `<string name="app_name" translatable="false">whichride</string>`.
  `res/resources.properties`: `unqualifiedResLocale=en`. In `build.gradle.kts`'s `android { }`:
  `androidResources { generateLocaleConfig = true }` (Android 13+ lists the four languages in
  the per-app language settings that Settings → App language opens).

- [ ] **Step 4: Translate.** Write the three files with every `string` and `plurals` of
  `values/strings.xml`, in the same order, same names, same placeholders. Rules:
  - Informal address: Italian *tu*, German *du*, Turkish *sen*. Short, plain sentences, like the English.
  - Never translate: whichride, MILES, Bolt Drive, SIXT share, SIXT, BVG Jelbi, Google Maps, km, min, €.
  - Escape apostrophes as `\'` (Italian has many: `l\'auto`), keep `%%` for a literal percent,
    keep typographic quotes “ ” where English has them.
  - Plurals: `one` and `other` for all three (CLDR); Turkish nouns stay singular after numbers
    ("3 araç").
  - Glossary:

| English | Italiano | Deutsch | Türkçe |
|---|---|---|---|
| provider | operatore | Anbieter | şirket |
| car / cars near you | auto / auto vicino a te | Auto / Autos in deiner Nähe | araç / yakınındaki araçlar |
| trip | viaggio | Fahrt | yolculuk |
| Open <provider> | Apri %1$s | %1$s öffnen | %1$s uygulamasını aç |
| discount | sconto | Rabatt | indirim |
| pass | abbonamento | Abo | abonelik |
| refuel credit | credito rifornimento | Tankgutschrift | yakıt kredisi |
| I'll refuel or charge | Faccio rifornimento o ricarico | Ich tanke oder lade | Yakıt alacağım veya şarj edeceğim |
| unlock (fee) | sblocco | Freischaltung | kilit açma |
| per km / per minute | al km / al minuto | pro km / pro Minute | km başı / dakika başı |
| minimum | minimo | Mindestpreis | en az ücret |
| package | pacchetto | Paket | paket |
| confirmed 2 days ago | confermato 2 giorni fa | vor 2 Tagen bestätigt | 2 gün önce onaylandı |
| price wrong? tell us | Prezzo sbagliato? Dillo a noi | Preis falsch? Sag es uns | Fiyat yanlış mı? Bize bildir |
| your price | il tuo prezzo | dein Preis | senin fiyatın |
| route service | servizio percorsi | Routendienst | rota hizmeti |
| settings | Impostazioni | Einstellungen | Ayarlar |

  The owner (Italian) reviews `values-it`; German and Turkish get a native speaker's review in
  the closed test (milestone 6, recorded in decision 71).

- [ ] **Step 5: Run** the test, then `./gradlew :app:testDebugUnitTest :app:lintDebug` — Expected:
  PASS; lint reports no `MissingTranslation`, `StringFormatInvalid` or `ExtraTranslation`.
  Robolectric screen tests keep running in English (their default locale).

- [ ] **Step 6: Look at it** on the phone: Android Settings → Apps → whichride → Language →
  Italiano / Deutsch / Türkçe; open Home, a result, Discounts and the price form in each; long
  German words must wrap, not clip (screenshots into the scratchpad).

- [ ] **Step 7: Commit** — `feat(android): Italian, German and Turkish`.

### Task 15: Accessibility pass

**Files:**
- Create: `android/app/src/test/kotlin/io/github/locadani/whichride/ui/Accessibility.kt`,
  `android/app/src/test/kotlin/io/github/locadani/whichride/ui/AccessibilityTest.kt`
- Modify: whatever the tests show (expected: `ui/ResultScreen.kt` top bar, `ui/Components.kt` `Monogram`)

- [ ] **Step 1: The helper** `Accessibility.kt`:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.semantics.SemanticsNode
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.SemanticsNodeInteractionsProvider
import androidx.compose.ui.test.isRoot
import androidx.compose.ui.unit.dp
import org.junit.Assert.assertTrue

/**
 * Every control on screen has a name TalkBack can read and a touch area of at least 48 × 48 dp
 * (spec §11, plan R18).
 */
fun SemanticsNodeInteractionsProvider.assertAccessible() {
    val root = onNode(isRoot()).fetchSemanticsNode()
    val controls = root.descendants().filter { node ->
        val config = node.config
        SemanticsActions.OnClick in config || SemanticsProperties.ToggleableState in config || SemanticsActions.SetText in config
    }
    assertTrue("no controls found", controls.isNotEmpty())
    for (node in controls) {
        val config = node.config
        val name = config.getOrNull(SemanticsProperties.ContentDescription)?.joinToString()
            ?: config.getOrNull(SemanticsProperties.Text)?.joinToString()
            ?: config.getOrNull(SemanticsProperties.EditableText)?.text
        assertTrue("control without a name: $config", !name.isNullOrBlank())
        val minimum = with(node.layoutInfo.density) { 48.dp.toPx() } - 0.5f
        val bounds = node.touchBoundsInRoot
        assertTrue("\"$name\" is ${bounds.width} × ${bounds.height} px, below 48 dp", bounds.width >= minimum && bounds.height >= minimum)
    }
}

private fun SemanticsNode.descendants(): List<SemanticsNode> = children.flatMap { listOf(it) + it.descendants() }
```

  (Text fields count their label as their name: in the merged tree the label is part of the
  field's `Text`. `48.dp` here is a test threshold, not UI code; the single-source guard skips tests.)

- [ ] **Step 2: Write the failing test** `AccessibilityTest.kt`: one test per screen, each in
  two variants, normal and **font scale 2.0** on a small phone:

```kotlin
package io.github.locadani.whichride.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.unit.Density
import androidx.test.ext.junit.runners.AndroidJUnit4
import io.github.locadani.whichride.compare.QuestionState
import io.github.locadani.whichride.compare.Screen
import io.github.locadani.whichride.core.Choice
import io.github.locadani.whichride.core.Question
import io.github.locadani.whichride.core.RankedOption
import io.github.locadani.whichride.crowd.PriceForm
import io.github.locadani.whichride.crowd.PricePurpose
import io.github.locadani.whichride.designsystem.WhichrideTheme
import io.github.locadani.whichride.personal.DealForm
import io.github.locadani.whichride.personal.DealKind
import io.github.locadani.whichride.personal.PercentOff
import io.github.locadani.whichride.personal.Personal
import io.github.locadani.whichride.personal.RefuelCreditDeal
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.junit.runners.Parameterized
import org.robolectric.ParameterizedRobolectricTestRunner
import org.robolectric.annotation.Config

@RunWith(ParameterizedRobolectricTestRunner::class)
@Config(qualifiers = "w360dp-h640dp")
class AccessibilityTest(private val fontScale: Float) {
    companion object {
        @JvmStatic @ParameterizedRobolectricTestRunner.Parameters(name = "font scale {0}")
        fun scales() = listOf(arrayOf<Any>(1f), arrayOf<Any>(2f))
    }

    @get:Rule val rule = createComposeRule()

    private fun show(content: @Composable () -> Unit) = rule.setContent {
        val density = LocalDensity.current
        CompositionLocalProvider(LocalDensity provides Density(density.density, fontScale)) { WhichrideTheme { content() } }
    }

    private val options = listOf(
        RankedOption(Choice.AnyOf("bolt"), QUOTES[0]),
        RankedOption(Choice.Offer("miles.s"), QUOTES[1]),
    )
    private val personal = Personal(deals = listOf(PercentOff("a", "bolt", 2_000, until = null), RefuelCreditDeal("f", "miles", 300)))

    @Test fun result() {
        show { ResultScreen(resultState(options).copy(personal = personal), ResultActions()) }
        rule.assertAccessible()
        rule.onNodeWithText("Open Bolt Drive").assertIsDisplayed()
    }

    @Test fun emptyResult() {
        show { ResultScreen(resultState(), ResultActions()) }
        rule.assertAccessible()
        rule.onNodeWithText("Add the cars near you").assertIsDisplayed()
    }

    @Test fun cars() {
        show { CarsScreen(resultState().copy(stack = listOf(Screen.Home, Screen.Cars("bolt")), crowdEnabled = true), "bolt", {}, {}, {}, {}, {}, {}) }
        rule.assertAccessible()
        rule.onNodeWithText("Done").assertIsDisplayed()
    }

    @Test fun providers() {
        show { ProvidersScreen(resultState(), {}, {}) }
        rule.assertAccessible()
    }

    @Test fun deals() {
        show { DealsScreen(resultState().copy(personal = personal), {}, {}, {}, {}) }
        rule.assertAccessible()
    }

    @Test fun dealEditor() {
        show { DealEditorScreen(resultState().copy(dealForm = DealForm(kind = DealKind.PASS)), {}, {}, {}, {}) }
        rule.assertAccessible()
        rule.onNodeWithText("Save discount").assertIsDisplayed()
    }

    @Test fun settings() {
        show { SettingsScreen(resultState(), {}, {}, { _, _ -> }, {}, {}) }
        rule.assertAccessible()
    }

    @Test fun priceForm() {
        show { PriceFormScreen(resultState().copy(priceForm = PriceForm(PricePurpose.Propose("bolt"))), {}, {}, {}, {}) }
        rule.assertAccessible()
        rule.onNodeWithText("Send car").assertIsDisplayed()
    }

    @Test fun question() {
        val asked = QuestionState(Question("bolt.polo", null, emptyList()), "bolt", "VW Polo")
        show { QuestionContent(resultState().copy(question = asked), QuestionActions()) }
        rule.assertAccessible()
    }

    @Test fun home() {
        show { HomeScreen(resultState().copy(stack = listOf(Screen.Home)), {}, {}, {}, {}, {}) }
        rule.assertAccessible()
    }

    @Test fun manualTrip() {
        show { ManualTripScreen(resultState().copy(stack = listOf(Screen.Home, Screen.ManualTrip)), {}, {}, {}, {}, {}, {}) }
        rule.assertAccessible()
        rule.onNodeWithText("Compare").assertIsDisplayed()
    }
}
```

  (Match each screen's parameter list to its current signature; `ManualTripScreen` and
  `HomeScreen` are from 5a. `QuestionContent` with no cards shows no value boxes, which is fine.)

- [ ] **Step 3: Run** `./gradlew :app:testDebugUnitTest --tests '*AccessibilityTest*'` — Expected: some
  FAIL. Known ones and their fixes:
  - **Result top bar at 200 %:** title, Discounts pill, Change start and Edit don't fit one row
    (the Open button is pushed off or controls shrink below 48 dp). Fix: the top bar keeps back +
    title; the pill, **Change start** and **Edit** move to a `FlowRow` under the title
    (`horizontalArrangement = Arrangement.spacedBy(Space.xs)`), each at least `Size.touchTarget` high.
  - **`Monogram` is read as a letter** ("B, Bolt Drive"): give it `Modifier.clearAndSetSemantics {}`;
    the provider's name is always next to it.
  - Any control the helper names: add its label or `heightIn(min = Size.touchTarget)`.
  Re-run until green; keep every fix inside the single-source rules.

- [ ] **Step 4: TalkBack details (5a minor).** Check by reading the code (the owner walks
  through on the phone in Task 16):
  - result rows announce "selected" (they are `selectable` with `Role.RadioButton`) and their
    price; the remove button says "Remove <provider> <car>";
  - step 2's checkboxes announce their state; the search box has a label;
  - the question sheet's title is a heading and is read first (`heading()`);
  - snackbars (Undo, crowd messages) are announced (Material's `SnackbarHost` uses a live region).

- [ ] **Step 5: Run** all unit tests and `cd design && npm test` — Expected: PASS.

- [ ] **Step 6: Commit** — `fix(android): accessibility pass: names, touch targets, 200 % font`.

### Task 16: Decisions, roadmap, README, final verification

**Files:**
- Modify: `docs/DECISIONS.md` (rows 65–73), `docs/plans/ROADMAP.md` (5b ✅), `android/README.md`

- [ ] **Step 1: Decisions.** Append to `docs/DECISIONS.md`:

| # | Date | Area | Decision | Alternatives considered | Why |
|---|---|---|---|---|---|
| 65 | 2026-10-20 | Prices | The **limits on reported values** (≤ 5 cards, label ≤ 40 and name ≤ 60 characters, unlock ≤ €500, per km and per minute ≤ €5, minimum ≤ €50, included units ≤ 20 000, no control or bidi characters) live in the core's `report` module; the server and the app use the same checks. | Limits in the server only | §10.1: one implementation; the app refuses a value before it is sent |
| 66 | 2026-10-20 | Providers | A provider may list **other apps** that open it (`app_name`, `other_apps` in `catalogue.json`); SIXT share opens BVG Jelbi by default or the SIXT app (`com.sixt.reservation`); each user's choice is kept on the phone. | A hard-coded list in the app | Provider facts live in the catalogue (§5.1); decision 48 promised the choice |
| 67 | 2026-10-20 | Personal | Discounts, passes, refuel credits, overrides, chosen apps and the last question day are one JSON file on the phone (no backup). A deal applies while enabled and up to its last day (phone's time zone); a **once** amount is used by **Open <provider>**, with Undo. | Preferences; using a once discount after a confirmation | §6 device-only; Open is the only moment a trip starts; Undo covers a trip not taken |
| 68 | 2026-10-20 | Crowd prices | A report edits the offer's **standard card** (unlock, per km, per minute, minimum); it becomes the user's override at once and is sent **once, best effort** (no outbox). The override goes when the shared price equals it or the user resets it. | Editing every card field; a retry queue | The canvas's form; integrity tokens expire within minutes and the override already gives the reporter the right price |
| 69 | 2026-10-20 | Crowd prices | The §7.2 question is armed by **Open <provider>** only, asked on return when this session's prices came from the server, decided in the core (`Prices.question`); the day it is shown counts as asked. | Asking after Check in; asking about cached prices | §4.1 excludes Check in; stale prices would be confirmed by mistake |
| 70 | 2026-10-20 | Crowd prices | Pending **proposals** are shown to every user in step 2 under "Added by users, not confirmed yet" and priced only when added; their offer ID is `proposal.<report>`; the question about them is yes/no. | Showing them only to the proposer | Another install must see a proposal to confirm it (§5.1, decision 11) |
| 71 | 2026-10-20 | i18n | Translations are written with a glossary and checked by a test (names, plurals, placeholders); Italian reviewed by the owner, German and Turkish by native speakers in the closed test; Android 13+ per-app language through `generateLocaleConfig`. | Machine translation at runtime; an in-app language picker | §11; Android already offers per-app languages |
| 72 | 2026-10-20 | Accessibility | A JVM test checks every control's name and 48 dp touch area on every screen at font scale 1 and 2; TalkBack is checked by hand on the phone. | Compose's ATF checks | ATF under Robolectric is unproven; the helper is deterministic |
| 73 | 2026-10-20 | Android | A **Settings** screen (gear on Home) holds discounts, the app per provider, language and licences; the install ID is stored with its token, and a Keystore failure keeps the install in memory. | Settings scattered over screens | One place for personal choices; decision 44 needs the ID |

- [ ] **Step 2: Roadmap and README.** `ROADMAP.md`: row 5b's Plan column becomes
  `` `05b-personal-and-crowd.md` ✅ ``. `android/README.md`: add sections "Personal" (where
  discounts live, that nothing leaves the phone), "Crowd prices" (writes need Play Integrity,
  which works only after milestone 6 links it; until then reports stay personal overrides),
  "Languages" (how to add a string: all four files, `TranslationsTest`).

- [ ] **Step 3: Full verification** (all must pass; paste the summaries in the final report):

```bash
cargo fmt --all --check
RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets --locked
TEST_DATABASE_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace --locked
(cd design && npm test)
(cd android && JAVA_HOME=/opt/homebrew/opt/openjdk@21 ./gradlew :app:testDebugUnitTest :designsystem:testDebugUnitTest :app:lintDebug :app:assembleDebug :app:assembleRelease)
```

- [ ] **Step 4: Release APK on the phone (5a minor: R8 and JNA).** Sign the unsigned release
  with the debug key and install it:

```bash
cd android/app/build/outputs/apk/release
BT=$(ls -d ~/Library/Android/sdk/build-tools/* | sort -V | tail -1)
$BT/zipalign -f 4 app-release-unsigned.apk aligned.apk
$BT/apksigner sign --ks ~/.android/debug.keystore --ks-pass pass:android --out whichride-release-debugsigned.apk aligned.apk
adb install -r whichride-release-debugsigned.apk
```

  (If the debug build is installed, `adb uninstall io.github.locadani.whichride` first: the
  signatures match, but say so to the owner since it removes their discounts.) Open the app,
  compare a pasted coordinate link with manual km/min, open a provider. Any crash = an R8 rule
  missing for JNA or the bindings: `adb logcat -b crash`, add the keep rule, rebuild.

- [ ] **Step 5: TalkBack walk-through (owner, on the phone).** Hand the owner this checklist:
  1. Home: the gear says "Settings"; the destination field says its label.
  2. Result: each option says provider, car, price and "selected"; the × says "Remove …";
     Discounts pill, refuel switch ("I'll refuel or charge, off"), Price wrong? are reachable.
  3. Step 2: "Any Bolt Drive car, checkbox, not checked"; proposals section heading.
  4. Discounts: each switch says "<deal> on, on/off"; the editor's chips say "selected".
  5. Price check sheet: the title is read first; Yes / No, fix it / Skip.
  6. Font size at maximum (Android Settings → Display → Font size): every screen still shows
     its main button.
  Record what the owner reports; fix issues before the PR or list them for milestone 6.

- [ ] **Step 6: Commit and push** — `docs: decisions 65–73, roadmap 5b done, Android README`; then
  `git push -u origin feat/m5b-personal-and-crowd`. Ask a fresh reviewer (most capable model) to
  review the whole branch against this plan and the spec; fix Critical/Important findings in
  `fix(android): review findings` (or the relevant scope), list Minor ones as deferred in the
  final summary. Report to the owner: branch head, test counts, what needs milestone 6 (Play
  Integrity link for every crowd write), and that the owner opens the PR and squash-merges.
