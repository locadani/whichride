# Decision Log

Every product and technical decision, newest at the bottom. Each one records what else was
considered and why this option won. Changing a decision means adding a new row that supersedes
the old one; rows are never rewritten. The full design is in
`docs/specs/2026-10-19-carsharing-compare-design.md`.

| # | Date | Area | Decision | Alternatives considered | Why |
|---|---|---|---|---|---|
| 1 | 2026-10-19 | Product | Compare the cost of **one trip** across Berlin carsharing providers | Price list comparison sites | Existing sites list prices; none answers "which is cheaper for this trip" |
| 2 | 2026-10-19 | Input | Destination from a **shared Google Maps link**, a pasted link, or a typed address/place | Link only | Links are how destinations arrive; typing covers everything else |
| 3 | 2026-10-19 | Platform | **Native Android app first**, website later | PWA only; native + PWA | Share-sheet and `geo:` integration are reliable only natively; PWA share target is Chrome-only and absent on iPhone |
| 4 | 2026-10-19 | Data | **No partnerships, no unofficial APIs**, no live car map | Reverse-engineered provider APIs | Unofficial APIs break terms and change without notice |
| 5 | 2026-10-19 | Cost | No Google Maps Platform; **OpenRouteService** behind our server | Google Maps APIs | Hard free limits; Google budgets only alert, they don't cap |
| 6 | 2026-10-19 | Cost | Quota exhausted → **manual km/minutes entry**; comparison still works | Paid fallback | Spikes degrade the service instead of producing a bill |
| 7 | 2026-10-19 | Privacy | **No routes or locations stored**; coordinates in request bodies, never URLs; no server-side route cache | Server cache of routes | Owner requirement; Cloud Run logs URLs |
| 8 | 2026-10-19 | Prices | **Crowd-updated prices, fully automatic**, from the second user | Owner moderation | The app must simplify the owner's life |
| 9 | 2026-10-19 | Prices | Reporter's edit is a **personal override immediately** | Wait for consensus | Reporter gets correct numbers now; trolls only affect themselves |
| 10 | 2026-10-19 | Prices | Confirmation asked of users **returning from "Open <provider>"**, max once a week | Ask random users | Only someone who just saw the price can confirm it |
| 11 | 2026-10-19 | Prices | Accept with **one other install confirming** and weighted yes > no | Fixed N confirmations | Works with two users |
| 12 | 2026-10-19 | Abuse | **Play Integrity** + install-ID reputation | "Must run a simulation afterwards" | That rule is easy to fake and loses honest reports |
| 13 | 2026-10-19 | Prices | No "what did you actually pay?" prompts | Payment feedback | Too invasive |
| 14 | 2026-10-19 | Prices | Personal discounts (% and €), subscriptions, refuel credits are **device-only** | Shared | They're personal |
| 15 | 2026-10-19 | Owner | Emergency switch is a **server CLI command**, no admin endpoint | Admin role in the app | Nothing to attack |
| 16 | 2026-10-19 | Architecture | **Rust core** shared by Android (UniFFI), web (WASM), server (Axum) | Kotlin Multiplatform; KMP + Rust; separate Kotlin/TS logic | One tested implementation everywhere; web path mature; reuses smoke-tracker server stack; validated by spike |
| 17 | 2026-10-19 | Architecture | Own server, not app → database directly | Firebase / Supabase | Voting rules stay in the Rust core; key still needs a server; no lock-in |
| 18 | 2026-10-19 | Web | **Astro static site**; read-only; Turnstile + per-IP limits + own quota share | Compose for web | SEO and accessibility; Play Integrity doesn't exist on the web |
| 19 | 2026-10-19 | Brand | One `tokens.json` + one `logo.svg` → Style Dictionary → Android and web | Separate assets | Single source of truth |
| 20 | 2026-10-19 | Providers | v1 = **MILES, Bolt Drive, Free2move** | Include station-based (Flinkster, cambio) | Station-based needs a round trip; doesn't fit one-way A→B |
| 21 | 2026-10-19 | Providers | **SIXT share left open**, added later as the test of the add-provider process | Include now | Its details are unverified; it exercises §5.1 for real |
| 22 | 2026-10-19 | Pricing | No automatic parking buffer minutes | +N minutes for Bolt | Owner decision |
| 23 | 2026-10-19 | Map | No map in v1; **Open in Google Maps** | In-app map | Free, no key, YAGNI |
| 24 | 2026-10-19 | i18n | English, Italian, German, Turkish | English only | Owner's audience |
| 25 | 2026-10-19 | Release | Closed test with **Berlin carsharing users** as testers | Paid testers; tester exchanges only | Real feedback; their confirmations start the crowd prices |
| 26 | 2026-10-19 | Name | App and repo named **whichride**; public repo github.com/locadani/whichride | `carsharing-compare`, `rideprice-comparator`, `fareview` | General enough for rides, scooters and more providers; portfolio |
