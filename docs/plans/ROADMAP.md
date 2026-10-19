# Roadmap

The design spec (`docs/specs/2026-10-19-carsharing-compare-design.md`) is built in milestones.
Each milestone has its own plan in this folder and ends with working, tested software. A plan is
written only when the previous milestone is done, so it builds on real code instead of guesses.

| # | Milestone | Delivers | Spec sections | Plan |
|---|---|---|---|---|
| 0 | **Feasibility spike** | Compose app calling Rust via UniFFI on a real phone | §13 | `spikes/uniffi-android/README.md` ✅ |
| 1 | **Foundation & pricing core** | Git repo, Cargo workspace, secret protection, CI + security workflows; pure `carsharing-core` crate: money, usage charges, rate cards and packages, personal adjustments, provider catalogue, `compare()` | §2, §3, §5, §6, §10, §13 | `01-foundation-and-pricing-core.md` |
| 2 | **Destination input** | Core: Google Maps link parsing (long links, `geo:` URIs, coordinates vs place names), short-link detection; airport/zone fees (e.g. MILES at BER) from start/destination coordinates; voting rules (§7.3–7.5) as pure functions | §4, §7 | — |
| 3 | **Server** | Axum server: provider catalogue + seed file with owner-entered prices, routing/geocoding/autocomplete proxy (OpenRouteService), short-link expansion, install registration, reports and votes, Play Integrity verification, emergency-switch CLI, Postgres (Neon), Dockerfile, Cloud Run deploy | §5.1, §7, §8, §9 | — |
| 4 | **Design system** | `design/tokens.json` + `logo.svg` → Style Dictionary → Android theme and `tokens.css`; contrast check in CI | §10.5 | — |
| 5 | **Android app v1** | UniFFI bindings built from Gradle, share-sheet and `geo:` entry, location, result screen, open provider / Google Maps, personal adjustments, price reports and confirmations, EN/IT/DE/TR, accessibility | §4, §6, §7, §10.3, §11 | — |
| 6 | **Release** | Privacy policy, Play Console, closed test (12 testers × 14 days), production | §12 | — |
| 7 | **Website** | Astro site, core via WebAssembly, Turnstile, per-provider SEO pages | §8.1, §10.4 | — |
| 8 | **SIXT share** | Added through the §5.1 process, as its real test | §5.1 | — |

**Before milestone 1 starts:** the GitHub repository is created by the owner (outward-facing).
