# Carsharing Compare — Design Spec

- **Date:** 2026-10-19
- **Status:** Approved (2026-10-19)
- **App name:** `whichride` (repo: github.com/locadani/whichride, public)

---

## 1. Purpose

In Berlin the user rents cars from several free-floating carsharing providers (Miles, Bolt and
others). Each charges differently (per km, per minute, unlock fees, minimum charges, discounts),
so which one is cheaper depends on the trip. No existing app compares the cost of **a given trip**
across providers.

The app takes a destination (a Google Maps link someone sent, or an address or place name typed
directly), estimates distance and driving time, and shows what the trip would cost with each provider, then opens the chosen
provider's app.

**Success criteria**

1. From a Google Maps link to a cost comparison in a few taps (Share → app → result).
2. Correct prices for every listed car/class, kept up to date by users without owner moderation.
3. Ready for other users from the second user onwards (no "personal only" shortcuts).
4. Zero running cost; usage spikes degrade the service instead of producing a bill.
5. Portfolio quality: every decision defensible on its merits ("it was faster" is never the reason).
6. Website findable by search engines and accessible; app accessible.

---

## 2. Constraints

| Constraint | Consequence |
|---|---|
| No money | Free tiers with hard limits only; no Google Maps Platform; Cloud Run max instances = 1 |
| Privacy | No routes or locations stored anywhere (§9) |
| No moderation work for the owner | Fully automatic price acceptance (§7); only an emergency switch |
| No partnerships, no unofficial APIs | No live car locations (the user marks nearby cars, §4.1); apps opened with plain intents/links |
| Portfolio + standards of smoke-tracker | Tests on all core logic, CI/CD on GitHub, free security tooling (cargo-deny, gitleaks, CodeQL, Dependabot), ADRs |
| One source for brand assets | Design tokens + one logo file generate the Android and web assets (§10.5) |
| Tokens never pushed | gitleaks pre-commit + CI; keys only in server environment |

---

## 3. Glossary

| Term | Meaning |
|---|---|
| **Provider** | A carsharing company (Miles, Bolt Drive, …). |
| **Offer** | One priced option of a provider: a car model (Bolt) or a size class (Miles S/M/L…). |
| **Rate card** | The set of prices of an offer: per km, per minute, unlock fee, minimum charge, etc. |
| **Trip estimate** | Distance (km) and driving time (minutes) from start to destination. |
| **Personal adjustment** | A user's own discount, subscription or override; never shared. |
| **Price report** | A user's claim that an offer's rate card changed. |
| **Confirmation** | A yes/no answer from a user who has just seen the price in a provider's app. |
| **Install ID** | Anonymous random identity of one app installation; no account. |

---

## 4. Core flow

1. The destination comes from one of:
   - a Google Maps link **shared to the app** (Android share sheet) or a `geo:` link opened with it;
   - a pasted link;
   - an **address or place name typed directly**, with suggestions while typing (OpenRouteService
     autocomplete via our server; Nominatim's usage policy forbids autocomplete).
2. The app resolves the destination:
   - short links (`maps.app.goo.gl`) are expanded by following the redirect;
   - coordinates are taken from the link when present;
   - otherwise the place name is geocoded.
3. Start = the phone's current location (location permission), changeable by the user.
4. The routing proxy (§8) returns **km and minutes**. Manual entry is always available as a fallback.
5. The core computes the cost of every offer of every provider, applying personal adjustments.
6. Result screen: offers sorted by price, price freshness labels (§7.5). The user marks which
   cars are near them (§4.1); the cheapest **marked** offer is highlighted.
7. Buttons: **Open <provider>** (opens the app, or the Play Store page if not installed) and
   **Open in Google Maps** (shows the route). No map inside the app in v1.

Android cannot make the app the default handler for `google.com/maps` links (Google owns the
domain), so Share and `geo:` are the entry points.

### 4.1 Cars near you (marked by the user)

The cheapest offer overall is useless if no such car is parked nearby. No provider publishes an
official availability feed for Berlin (research:
`docs/research/2026-10-19-vehicle-availability-feeds.md`), and unofficial APIs are ruled out (§2),
so the **user tells the app what they see** in the providers' apps.

- **What is marked:** an **offer**, i.e. whatever the provider prices by: a car model (Bolt Drive,
  Free2move) or a size class (MILES S/M, Premium, L, XL). Packages are not marked; they are chosen
  before the trip and the core already picks the cheapest one per offer.
- **How:** the result screen has a "What's near you?" panel with one toggle chip per offer,
  grouped by provider. One tap marks or unmarks an offer and the list re-ranks at once:
  - marked offers on top, cheapest first, the cheapest highlighted;
  - unmarked offers below a "Not marked nearby" divider, still visible with their price, so the
    user can see what walking further would save.
- **Whole provider:** tapping a provider's name marks all its offers, for "there's a Bolt car
  around, I don't know which model".
- **Check in the app:** each provider group has a **Check in <provider>** button that opens the
  provider's app; coming back keeps the panel and its marks. This return does **not** trigger the
  §7.2 confirmation question, which follows only **Open <provider>** for the chosen car.
- **Missing car:** "Not in the list? Add it" starts the new-offer proposal (§5.1).
- **Nothing marked:** the screen works as before: all offers cheapest first, the cheapest
  highlighted, with the panel inviting the user to mark what's nearby.
- **Lifetime:** marks are kept on the device only and cleared after **15 minutes** (cars move;
  value to be tuned in the closed test). Never sent to the server.
- **Core:** splitting quotes into marked and unmarked and picking the cheapest marked one is a
  pure, tested core function, so the app and the website behave the same.
- **Accessibility:** chips are toggle buttons whose on/off state is announced; touch targets meet
  §11.
- The interaction is tried in the clickable prototype before milestone 5 builds it.

---

## 5. Providers and pricing model

- v1 covers **all carsharing providers active in Berlin**. Research of official pages on
  2026-10-19:

| Provider | Type | Android package | Pricing structure | Granularity |
|---|---|---|---|---|
| MILES | free-floating | `com.driveby.app` | per km (rounded up) + unlock fee; parked minutes; BER fee; minimum for L/XL | size class (S/M, Premium, L, XL) |
| Free2move (ex Share Now) | free-floating | `com.car2go` | per minute incl. 200 km, then per km; airport fee | car model |
| SIXT share *(open, unverified)* | free-floating | `com.sixt.reservation` | per minute + unlock fee; per-km tariff since 2026; dynamic | category/model |
| Bolt Drive | free-floating | `ee.mtakso.client` | per minute + per km (rounded up) + minimum fee; dynamic allowed | car |
| Flinkster (DB) | station-based | `de.flinkster.sharing.app` | per hour (daily cap) + per km + booking fee | class |
| cambio | station-based | `de.cambio.app` | per hour + per km; plan-dependent rates | class + plan |

  Not included: WeShare (gone, bought by MILES), Zity (not in Germany), Greenwheels and Ubeeqo
  (not confirmed in Berlin), Getaround (peer-to-peer, host-set prices).
- **Station-based providers** (Flinkster, cambio) are **excluded** (decided 2026-10-19): they need
  a round trip back to the station, which doesn't fit the one-way A→B flow. v1 = MILES, Bolt
  Drive, Free2move. **SIXT share is left open** and will be added after launch as the real test
  of §5.1 (adding a provider).
- Granularity differs (car model, size class, category). The data model is
  `provider → offers → rate card`, so all fit.
- Rate card components (superset; each provider uses a subset): per km, per minute, included km,
  unlock/booking fee, minimum charge, parked minutes, airport/zone fees, rounding rule (e.g. km
  rounded up).
- **Packages** (hour/day bundles with km allowance, chosen before the trip) are modelled as
  alternative rate cards of the same offer; the core computes all and shows the cheapest,
  naming the package.
- **Dynamic pricing** (Bolt, SIXT; MILES discounted cars): stored prices are typical prices, shown
  as estimates; crowd confirmations keep them current.
- No automatic "parking buffer" minutes are added to minute-based prices.
- Money is computed in integer cents; rounding is explicit and tested.
- Price data lives as data (seeded from a versioned file, updated through §7), never in code.
- The owner enters the initial prices from the providers' apps.

### 5.1 Adding providers and offers

| Change | Who | How |
|---|---|---|
| Price of an existing offer | Any user | Crowd report (§7) |
| New offer of an existing provider (e.g. a new Bolt car model) | Any user | Proposed in the app with its rate card; same confirmation rule as a price report |
| New provider | Owner | Entry in the versioned provider file in the repo (name, app package, offers, which rate card components it uses) → pull request → CI tests → deploy. If its pricing uses a component the core doesn't know yet, the core is extended first, with tests. |

Apps download the provider list and prices from the server, so a new provider appears without an
app update (unless the core needed a new component).

---

## 6. Personal adjustments (stored on the device only)

| Type | Example | Effect |
|---|---|---|
| Discount, percentage | "Bolt −20% until 31 Oct" | Applied to that provider's total |
| Discount, fixed amount | "Miles −€5, once" | Subtracted from that provider's total (not below €0) |
| Subscription | MILES Pass: % off km rate, waived unlock fee; Free2move Pass: % off minutes only; cambio: different rate table | Permanent; acts on specific components or replaces rates; monthly fee ignored per trip |
| Refuel/charge credit | "MILES credits me €5"; toggle "I'll refuel" on a trip | Subtracts the credit the user entered for that provider (providers don't publish it, so it is never shared data) |
| Price override | User's own rate card for an offer | Used instead of the shared price; applied instantly |

Order of application (fixed in the core and tested): override → subscription → percentage
discount → fixed discount and refuel credit → minimum charge.

---

## 7. Crowd-updated prices (automatic, no owner moderation)

### 7.1 Reporting
A user who sees a different price edits the rate card. The edit becomes their **personal override
immediately** and is sent as a **price report** (offer, new rate card, time). Personal discounts
are never reported.

### 7.2 Confirmation
When the user taps **Open <provider>** and comes back to the app, it may ask one prefilled
question, e.g. "Bolt Corsa: €0.29/km, €0.11/min — correct?":

- **Yes** → +1 for that rate card (a pending report or the current price), refreshes its
  "confirmed" date.
- **No** → the same screen lets them correct the numbers: with a value it counts as a vote for
  that value (new report or support for a matching one); without a value it counts as −1.
- **Skip** → no effect.

Asked at most **once a week per install**, and only when there is a pending report or a price not
confirmed recently.

### 7.3 Acceptance rule
A report is accepted when **at least one other install confirms it** and the weighted yes votes
exceed the weighted no votes. This works from the second user. Until acceptance, only the reporter
sees the new value.

### 7.4 Weighting and abuse resistance
- Install ID: random, created on first launch, key held in the Android Keystore.
- Every report or vote carries a **Play Integrity** token bound to the request (request hash);
  the server verifies app integrity, device integrity and the request hash with Google.
- Weight grows with the install's age and with how often its past votes matched final outcomes;
  new installs weigh less.
- Owner has an **emergency switch** (revert/lock an offer) for abuse only, not routine work. It is
  a **server CLI command** run by the owner against the production database (as in smoke-tracker),
  not an endpoint: the app and the server API have no admin role, so there is nothing to attack.
  The owner is identified by having access to the cloud project, not by anything in the app.
- Rejected: the "must run a simulation afterwards" rule (easy to fake, loses honest reports) and
  "what did you actually pay?" prompts (too invasive).

### 7.5 Freshness
Each offer shows when its price was last confirmed ("confirmed 2 days ago").

---

## 8. Routing and costs

- Routing: **OpenRouteService** (free plan with a hard daily limit; no card, no overage bill).
  Geocoding: OpenRouteService geocoder or Nominatim.
- The app never holds the key. It calls **our server**, which adds the key and forwards the
  request; the server also checks Play Integrity so only the real app can use the quota.
  (Integrity tokens are checked when an install registers and on every write; routing calls need
  the install's token, decision 33.)
- Cloud Run with **max instances = 1**; per-install rate limits.
- When the daily quota is used up or the server is unreachable, the app says "route service busy —
  enter km and minutes" and shows two fields; the comparison still works because it is calculated
  on the device.
- Opening apps and Google Maps uses intents: free, no key.
- Estimates have no live traffic; acceptable.

### 8.1 Requests from the website
Play Integrity exists only for Android apps, so web requests are protected differently:
- **Read-only:** the website reads prices and computes comparisons, but cannot send price reports
  or votes (they need Play Integrity).
- **Routing and autocomplete** from the web: Cloudflare Turnstile (free, normally invisible check
  that the request comes from a real browser) + per-IP rate limits.
- The web gets its own **share of the daily routing quota**, so web abuse can't use up the app's.

---

## 9. Privacy

- **No routes or locations are stored**: not in the database, not in caches, not in logs.
- Coordinates travel in the **request body**, never in URLs (Cloud Run logs URLs).
- Any route caching happens on the device only.
- Server stores only: install IDs, price reports, votes, accepted prices (offer, values, times).
- Privacy policy required (Play Store, GDPR).

---

## 10. Architecture

```
crates/core/    Rust: data model, price calculation, adjustments, voting rules, link parsing + tests
crates/server/  Rust (Axum): prices, reports, votes, routing proxy, Play Integrity verification
android/        Kotlin + Jetpack Compose; uses core via UniFFI (built with cargo-ndk / Gobley)
web/            Astro static site; uses core via WebAssembly
design/         tokens.json, logo.svg → generated assets for android/ and web/
```

Rust crates live under `crates/` in one Cargo workspace at the repo root, as in smoke-tracker.

### 10.1 Why a Rust core
One tested implementation of prices and voting rules runs in the app, the website and the server,
so they can never disagree (the server validates with the same code the app calculates with).
Each UI is native to its platform. Same pattern as Mozilla (Firefox), Proton, Signal.

### 10.2 Rejected alternatives
- **Kotlin Multiplatform core:** simpler Android build, but weaker path to a normal web page
  (Kotlin/JS) and no server reuse of the smoke-tracker stack.
- **Compose Multiplatform for the web:** canvas rendering, poor SEO and accessibility.
- **KMP + Rust core together:** KMP only pays off for sharing Kotlin with iOS; no iOS app, so an
  extra layer with no benefit.
- **PWA only:** weak share-sheet integration on Android, none on iPhone.
- **Separate Kotlin and TypeScript logic:** duplicated rules that drift.
- **App talking to the database directly (Firebase/Supabase):** voting rules would be rewritten
  in database rules or JS/TS functions instead of reusing the Rust core; the routing key would
  still need a server; vendor lock-in.

### 10.3 Android
Native Compose UI, Material accessibility, share-sheet and `geo:` intent filters, opens provider
apps by package name (Play Store fallback).

### 10.4 Website (later, code prepared now)
Astro static pages: paste a link or an address → comparison. Per-provider pages ("Miles prices
Berlin", "Miles vs Bolt") generated from the price data for search engines. Core via WebAssembly.

### 10.5 Shared brand assets
`design/tokens.json` (W3C design token format) → **Style Dictionary** → `Colors.kt`/theme and
`tokens.css`. One `logo.svg` → Android vector drawable and adaptive icon, and the web logo/favicon.
CI checks that generated files are up to date and that colour pairs meet WCAG contrast.

---

## 11. Languages and accessibility

- English, Italian, German, Turkish (app and website).
- Android: content descriptions, touch target sizes, dynamic font size, TalkBack checks.
- Web: semantic HTML, WCAG 2.2 AA.

---

## 12. Release

- Google Play: one-time developer fee; new personal accounts must run a **closed test with at
  least 12 testers for 14 days** before production. Plan: §14 (being discussed).
- Website: hosted for free (GitHub Pages or similar).

---

## 13. Delivery order

1. **Feasibility spike:** an empty Compose app calling one Rust function via UniFFI + cargo-ndk
   with current versions. **Done 2026-10-19: works on a real phone**; findings in
   `spikes/uniffi-android/README.md`.
2. Core: data model, prices, adjustments, link parsing (all tested).
3. Server: price data, routing proxy, Play Integrity.
4. Android v1: full flow, personal adjustments, reports and confirmations.
5. Closed test on Google Play, then production.
6. Website.

---

## 14. Open questions

- **Play Store testers** for the closed test (being discussed).
- SIXT share details (official site unreachable during research) and exact rounding rules.
- Bolt minimum charge and Miles unlock fee: exact current values (owner enters them).
