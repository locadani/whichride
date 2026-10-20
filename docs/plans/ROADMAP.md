# Roadmap

The design spec (`docs/specs/2026-10-19-carsharing-compare-design.md`) is built in milestones.
Each milestone has its own plan in this folder and ends with working, tested software. A plan is
written only when the previous milestone is done, so it builds on real code instead of guesses.

| # | Milestone | Delivers | Spec sections | Plan |
|---|---|---|---|---|
| 0 | **Feasibility spike** | Compose app calling Rust via UniFFI on a real phone | §13 | `spikes/uniffi-android/README.md` ✅ |
| 1 | **Foundation & pricing core** | Git repo, Cargo workspace, secret protection, CI + security workflows; pure `carsharing-core` crate: money, usage charges, rate cards and packages, personal adjustments, provider catalogue, `compare()` | §2, §3, §5, §6, §10, §13 | `01-foundation-and-pricing-core.md` ✅ |
| 2 | **Destination input** | Core: Google Maps link parsing (long links, `geo:` URIs, coordinates vs place names), short-link detection; voting rules (§7.3–7.5) as pure functions | §4, §7 | `02-destination-input.md` ✅ |
| 3 | **Server** | Axum server: provider catalogue + seed file with owner-entered prices, routing/geocoding/autocomplete proxy (OpenRouteService), short-link expansion, install registration, reports and votes, Play Integrity verification, emergency-switch CLI, Postgres (Neon), Dockerfile, Cloud Run deploy | §5.1, §7, §8, §9 | `03-server.md` ✅ |
| 4 | **Design system** | `design/tokens.json` + `logo.svg` → Style Dictionary → Android theme and `tokens.css`; contrast check in CI | §10.5 | `04-design-system.md` ✅ |
| 5a | **Android app v1: core flow** | UniFFI bindings built from Gradle; `:app`; share-sheet and `geo:` entry, pasted links, typed addresses with server autocomplete; location; result screen; adding the cars near you (§4.1); Open <provider> (SIXT through Jelbi) and Open in Google Maps; manual km/min fallback; install registration (Play Integrity) for routing | §4, §4.1, §7.5, §8, §10.3 | `05a-android-core-flow.md` ✅ |
| 5b | **Android app v1: personal and crowd** | Personal discounts and passes (§6), price reports and confirmations with Play Integrity (§7), "Car not listed? Add it" (§5.1), per-user choice of which app opens a provider (e.g. SIXT through Jelbi or the official app), EN/IT/DE/TR, accessibility pass (TalkBack, 200 % font) | §5.1, §6, §7, §11 | — |
| 6 | **Release** | Privacy policy, Play Console, closed test (12 testers × 14 days), production; link Play Integrity to the Cloud project in Play Console | §12 | — |
| 7 | **Website** | Astro site, core via WebAssembly, Turnstile, per-provider SEO pages | §8.1, §10.4 | — |
| 8 | **Free2move** | Added through the §5.1 process, as its real test (SIXT share moved into v1, decision 48) | §5.1 | — |
| 9 | **Zone fees** | Airport/zone fees (e.g. MILES at BER) from start/destination coordinates | §5 | — |
| 10 | **Availability feeds** *(only if the IVSG reform requires them)* | Nearby cars from official feeds via Mobilithek, replacing manual marking where available | §4.1 | — |

**Before milestone 1 starts:** the GitHub repository is created by the owner (outward-facing).
