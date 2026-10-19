# Milestone 3: Server — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A deployable Axum server, `crates/server`, that serves the provider catalogue with
crowd prices, accepts price reports, votes and confirmations from genuine app installs, proxies
routing, geocoding and autocomplete to OpenRouteService without storing a single location,
expands Google Maps short links, and gives the owner an emergency-switch CLI — plus the two
core fixes left over from milestone 2's review.

**Architecture:** One binary crate (`whichride-server`, library `whichride_server`) with a
`serve` command and owner CLI commands, the same shape as smoke-tracker's `app` crate. Every
price and voting rule is a call into `carsharing-core`; the server only stores, authenticates
and forwards. Postgres (Neon in production) holds installs, offers, reports, votes and price
history — never coordinates. External services (OpenRouteService, Google Play Integrity, Google's
short-link redirects) sit behind small clients that tests point at `wiremock` servers.

**Tech Stack:** Rust (edition 2024, workspace lints as before), axum 0.8.9, tokio 1.53, sqlx 0.9
(Postgres, rustls/ring), reqwest 0.13 (rustls/ring, JSON, no redirects for short links),
tower-http 0.7.1 (trace, request-id, catch-panic, body limit), clap 4.6.7, tracing 0.1.44,
sha2 0.11, base64 0.22, uuid 1 (v4), getrandom 0.4, wiremock 0.6 (tests). Docker
(distroless static, as smoke-tracker), Terraform (copy of smoke-tracker's `cloud-run-app`
module), GitHub Actions.

**Spec:** `docs/specs/2026-10-19-carsharing-compare-design.md` (§4 core flow, §5/§5.1 providers
and adding offers, §7 crowd prices, §8 routing and costs, §9 privacy, §10 architecture).
Decisions: `docs/DECISIONS.md` (rows 1–31). Previous milestones:
`docs/plans/01-foundation-and-pricing-core.md`, `docs/plans/02-destination-input.md`.
Infrastructure to reuse: `../smoke-tracker` (`Dockerfile`, `infra/`, `.github/workflows/deploy.yml`,
`crates/storage-postgres/tests/common/mod.rs`).

## Before starting

This plan was written on branch `feat/m3-server`, cut from `main` while milestone 2
(`feat/m2-destination-input`) was still waiting for its PR. **Milestone 2 must be squash-merged
first.** Then:

```bash
git fetch origin
git switch feat/m3-server
git rebase --onto origin/main $(git merge-base feat/m3-server origin/main)
cargo test --workspace   # milestone 2's code must be present: crates/core/src/link.rs exists
```

If `crates/core/src/link.rs`, `voting.rs` or `day.rs` is missing, stop: milestone 2 is not on
`main` yet.

## Global Constraints

- Work on branch `feat/m3-server`. Never commit to `main`. The owner pushes or approves pushes;
  the remote is `git@github-locadani:locadani/whichride.git` (SSH alias for the `locadani` key).
  Never use `gh` for this repo: it is logged in as a different account.
- Before every commit, `git config user.name` must print `locadani` and `git config user.email`
  must print `61902944+locadani@users.noreply.github.com`. Never put any email address in a
  tracked file.
- Never commit secrets (the OpenRouteService key, database URLs, tokens). The gitleaks
  pre-commit hook (`git config core.hooksPath .githooks`) must be active. Keys exist only in
  the server's environment (Secret Manager in production, your shell locally).
- Every commit message ends with a blank line followed by
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Commit subjects use Conventional Commits: `feat(core): …`, `feat(server): …`, `ci: …`,
  `build: …`, `docs: …`.
- Doc comments explain *what* and *why*; no tutorial-style comments.
- Workspace-wide: `unsafe_code = "forbid"`, clippy warnings are errors (`-D warnings`),
  `cargo fmt` enforced. New workspace dependencies go in the root `[workspace.dependencies]`
  with the versions below and are used with `.workspace = true`.
- `carsharing-core` stays pure: no I/O, no clock, no floating point. Floats are allowed in the
  server **only** at the OpenRouteService boundary, converted to integers immediately.
- **Privacy (spec §9), non-negotiable:**
  - No coordinates, place names, search text or short links in the database, in logs, in error
    messages or in URLs the server receives. They travel in POST bodies only.
  - No route, geocoding or short-link cache on the server, not even in memory.
  - Request logs contain method, path, status, latency and request ID — never bodies or headers.
- Spec §2: Cloud Run **max instances = 1**; no paid services; usage spikes degrade service
  (HTTP 503 `route_service_busy`) instead of producing a bill.
- Spec §7.4: no admin endpoint and no admin role. Owner actions are CLI commands only.
- Tests: unit tests in `#[cfg(test)] mod tests` at the bottom of each source file; HTTP and
  database tests in `crates/server/tests/`. Postgres tests use a fresh database per test on
  the server named by `TEST_POSTGRES_URL` and are skipped (with a message) when it is unset,
  except in CI where it must be set.
- Local Postgres for tests: if none is running, start one with
  `docker run -d --name whichride-pg --rm -p 5432:5432 -e POSTGRES_HOST_AUTH_METHOD=trust postgres:17-alpine`
  and `export TEST_POSTGRES_URL=postgres://postgres@localhost:5432/postgres`.
- Disk is nearly full on the owner's Mac: at the end, report `df -h /System/Volumes/Data` and
  give the `cargo clean` and `docker rmi` commands; never `rm -rf` build folders.
- Anything outward-facing (creating cloud resources, Play Console, Neon, pushing) is done by the
  owner. Tasks that need it say **OWNER** and stop.

## Review Focus

1. **A location leaking into logs.** A route request whose coordinates appear in a log line, an
   error message or a panic message. Logs must contain none of them, even at `trace` level for
   the server's own targets (test in Task 10).
2. **Short links that bounce through Google's EU consent page.** From a European server,
   `maps.app.goo.gl/…` can redirect to `consent.google.com/…?continue=<the real maps URL>`. The
   expansion must follow `continue`, not fail (test in Task 11).
3. **Concurrent votes on the same report.** Two installs confirming at the same moment must not
   accept the report twice, double-count accuracy or create two price-history rows (test in
   Task 8).
4. **Absurd or hostile report values.** €1000 per km, 50 rate cards, a 10 kB label, a vote on
   an already accepted report, a report on a locked offer. Each gets a clear 4xx and changes
   nothing (tests in Task 8).
5. **Upstream quota exhausted or down.** OpenRouteService answering 429/403/500 or timing out.
   The app must get `503 route_service_busy` (so it shows the manual km/min fields), never a 500
   with the upstream's body (tests in Task 10).

---

## Decisions this plan makes

These become rows 32–41 of `docs/DECISIONS.md` in Task 16. They are listed here so a reviewer
can reject one before any code exists.

| # | Decision | Alternatives | Why |
|---|---|---|---|
| 32 | Install identity: the server issues a random install ID and a 256-bit bearer token at registration; it stores only the token's SHA-256. The app keeps the token encrypted with an Android Keystore key. | A Keystore key pair signing every request | Play Integrity already binds every write to a genuine app and to the exact request (§7.4); the token only has to say *which* install. Signatures would add per-request crypto on both sides and no protection the integrity token does not already give. |
| 33 | Play Integrity on **registration, reports, votes and confirmations** (standard requests, `requestHash` = SHA-256 of method, path and body). Routing, geocoding, autocomplete and short-link expansion need a registered install's token and per-install rate limits, not a fresh integrity token. | An integrity token on every routing and autocomplete call | Only genuine apps can register, so only they can use the quota (§8). Autocomplete fires per keystroke; a Google round trip each time would add latency and spend the Play Integrity daily allowance. |
| 34 | Provider data (name, Android package, refuel credit) comes from `crates/server/catalogue.json`, compiled into the binary; offer prices live in Postgres, seeded from that file only for offers the database does not have yet. | Everything in the database; everything in the file | Matches §5.1: providers change by pull request, prices change by crowd reports. A redeploy never overwrites crowd prices. |
| 35 | SQL with runtime-checked `sqlx::query` and `FromRow`, every query covered by tests against a real Postgres. | `query!` macros with committed `.sqlx/` metadata (smoke-tracker) | Tests against Postgres check constraints, locking and results, not just column types; no generated metadata to keep in sync. |
| 36 | A "no" without a value on the **current** price is recorded nowhere and changes nothing; only a report with a value can change a price. | Count it against the current price | §7.2 defines −1 for reports; a current price has no rule that a −1 could trigger, and an unused counter is a stored fact with no purpose. |
| 37 | The confirmation question about a pending change is about the **oldest pending report** of that offer; the catalogue shows its values to every install, but comparisons use them only for the reporter. | Show nothing pending | §7.2's question has to show the values being confirmed. |
| 38 | Identical rate cards reported again for the same offer become a **yes vote** on the existing pending report, not a second report. | Separate reports | §7.2: "a vote for that value (new report or support for a matching one)". |
| 39 | When a report is accepted, every other pending report of the same offer is **superseded**; installs' accuracy is settled for all of them (yes matched on the accepted one, no matched on the superseded ones). | Leave them pending | Only one value can be the price; stale reports would ask questions about values nobody sees. |
| 40 | Production runs in its **own Google Cloud project and Neon project**, with a copy of smoke-tracker's Terraform module extended for extra secrets. | Share smoke-tracker's project | Separate blast radius, budget and IAM; the public repo stays self-contained. |
| 41 | Android application ID **`io.github.locadani.whichride`** (the server's default `ANDROID_PACKAGE` in production). | `app.whichride`, `dev.locadani.whichride` | The reverse of a domain the owner controls (locadani.github.io); a Play application ID can never change. |

---

## API

All bodies are JSON. Errors are `{"error": "<code>"}` with the status below. Every `POST` body is
at most 16 KiB.

| Method and path | Auth | Body → response |
|---|---|---|
| `GET /health` | — | → `200 ok` (plain text) |
| `GET /v1/catalogue` | — | → `200 CatalogueResponse` |
| `POST /v1/installs` | integrity | `{}` → `201 {"install_id","token"}` |
| `POST /v1/reports` | bearer + integrity | `ReportRequest` → `201 {"report","status"}` (new) or `200` (joined an identical report, or equals the current price) |
| `POST /v1/reports/{id}/votes` | bearer + integrity | `{"answer":"yes"\|"no"}` → `200 {"status"}` |
| `POST /v1/offers/{id}/confirmations` | bearer + integrity | `{"answer"}` → `204` |
| `POST /v1/route` | bearer | `{"from":Coordinate,"to":Coordinate}` → `200 Trip` |
| `POST /v1/geocode` | bearer | `{"text","near"?}` → `200 {"places":[{"label","at"}]}` |
| `POST /v1/autocomplete` | bearer | `{"text","near"?}` → same as geocode |
| `POST /v1/expand` | bearer | `{"url"}` → `200 Destination` (never `short_link`) |

Headers: `Authorization: Bearer <token>`; `X-Integrity-Token: <Play Integrity standard token>`.

| Error code | Status | When |
|---|---|---|
| `bad_request` | 400 | Body is not valid JSON for the endpoint, or a value is out of range |
| `unauthorized` | 401 | Missing or unknown bearer token |
| `integrity_failed` | 403 | Missing, invalid, stale or non-matching integrity token |
| `not_found` | 404 | Unknown offer or report |
| `offer_locked` / `report_closed` | 409 | Owner locked the offer / report no longer pending |
| `not_a_short_link` / `no_destination` | 422 | Expand input is not a short link / its target has no destination |
| `no_route` | 422 | OpenRouteService found no route |
| `too_many_requests` | 429 | Per-install limit reached |
| `integrity_unavailable` | 503 | Google's verdict service is down |
| `route_service_busy` | 503 | OpenRouteService quota used up, rate limited, down or slow |
| `internal` | 500 | Anything else (logged without request data) |

---

## File Structure

```
Cargo.toml                         + server dependencies in [workspace.dependencies]
crates/core/src/
├─ geo.rs                          Coordinate JSON goes through Coordinate::new        (Task 1)
├─ voting.rs                       Weight JSON capped at MAX_WEIGHT; settle()           (Tasks 1, 2)
├─ link.rs                         ShortLink url = https://host/path                    (Task 1)
├─ day.rs                          Day::from_unix_seconds                               (Task 2)
└─ lib.rs                          re-exports
crates/server/
├─ Cargo.toml
├─ catalogue.json                  seed: providers, offers, owner-entered prices        (Task 4)
├─ migrations/0001_initial.sql                                                          (Task 4)
├─ src/
│  ├─ main.rs                      parses the CLI, runs a command                       (Task 3)
│  ├─ lib.rs                       module list                                          (Task 3)
│  ├─ cli.rs                       clap commands                                        (Tasks 3, 12)
│  ├─ config.rs                    environment → ServeConfig                            (Task 3)
│  ├─ state.rs                     AppState shared by handlers                          (Task 3)
│  ├─ app.rs                       router, middleware                                   (Task 3)
│  ├─ error.rs                     ApiError → JSON responses                            (Task 3)
│  ├─ db.rs                        connect, migrate, seed                               (Task 4)
│  ├─ catalogue.rs                 GET /v1/catalogue                                    (Task 5)
│  ├─ integrity.rs                 Play Integrity verdicts and Google client            (Task 6)
│  ├─ auth.rs                      tokens, extractors (Attested, Signed, Authed)        (Task 7)
│  ├─ installs.rs                  POST /v1/installs                                    (Task 7)
│  ├─ crowd.rs                     reports, votes, confirmations (SQL + core rules)     (Tasks 8, 9)
│  ├─ limits.rs                    per-install rate limits                              (Task 10)
│  ├─ ors.rs                       OpenRouteService client                              (Task 10)
│  ├─ routing.rs                   /v1/route, /v1/geocode, /v1/autocomplete             (Task 10)
│  ├─ expand.rs                    short-link expansion and /v1/expand                  (Task 11)
│  └─ owner.rs                     emergency-switch commands                            (Task 12)
└─ tests/
   ├─ common/mod.rs                fresh database, test app, request helpers            (Task 4)
   ├─ catalogue.rs, installs.rs, crowd.rs, proposals.rs, routing.rs, expand.rs, owner.rs
Dockerfile, .dockerignore                                                               (Task 13)
.github/workflows/ci.yml           + Postgres service                                   (Task 4)
.github/workflows/deploy.yml                                                            (Task 15)
infra/                             copy of smoke-tracker's, + secret_env                (Task 14)
docs/DECISIONS.md, docs/plans/ROADMAP.md, README.md, docs/specs/…                       (Task 16)
```

---
### Task 1: Core — range-checked JSON and tidy short links (deferred from milestone 2)

Milestone 2's review left two points for this milestone, because the server is the first code
that reads these types from untrusted JSON:

- `Coordinate` and `Weight` derive `Deserialize` directly, so `{"lat_e7": 2000000000, …}` or a
  weight of 4 billion pass straight in. JSON must go through the same checks as the
  constructors.
- `Destination::ShortLink` keeps the link exactly as shared, including tracking queries
  (`?g_st=ic`), `http://` and fragments. The server should receive only `https://host/path`.

**Files:**
- Modify: `crates/core/src/geo.rs`
- Modify: `crates/core/src/voting.rs`
- Modify: `crates/core/src/link.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `Coordinate::new`, `CoordinateError`, `Weight`, `MAX_WEIGHT`, `Destination` from
  milestone 2.
- Produces: `#[serde(try_from = …)]` on `Coordinate` and `Weight`; new
  `WeightError(pub u32)` (re-exported from the crate root); `impl From<Weight> for u32`;
  `Destination::ShortLink { url }` where `url` is always `https://<host><path>`; a short link
  with no ID (`https://maps.app.goo.gl/`) is `Err(LinkError::NoDestination)`.

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `crates/core/src/geo.rs`:

```rust
    #[test]
    fn json_round_trips() {
        let point = at(525_219_184, 134_132_147);
        let json = serde_json::to_string(&point).unwrap();
        assert_eq!(json, r#"{"lat_e7":525219184,"lon_e7":134132147}"#);
        assert_eq!(serde_json::from_str::<Coordinate>(&json).unwrap(), point);
    }

    #[test]
    fn json_accepts_the_limits() {
        let json = r#"{"lat_e7":-900000000,"lon_e7":1800000000}"#;
        assert_eq!(
            serde_json::from_str::<Coordinate>(json).unwrap(),
            at(-900_000_000, 1_800_000_000)
        );
    }

    #[test]
    fn json_rejects_points_off_the_earth() {
        for json in [
            r#"{"lat_e7":900000001,"lon_e7":0}"#,
            r#"{"lat_e7":0,"lon_e7":-1800000001}"#,
            r#"{"lat_e7":2147483647,"lon_e7":0}"#,
        ] {
            assert!(serde_json::from_str::<Coordinate>(json).is_err(), "{json}");
        }
    }

    #[test]
    fn json_rejects_missing_or_non_integer_fields() {
        for json in [
            r#"{"lat_e7":0}"#,
            r#"{"lat_e7":52.5,"lon_e7":13}"#,
            r#"{"lat_e7":"0","lon_e7":0}"#,
        ] {
            assert!(serde_json::from_str::<Coordinate>(json).is_err(), "{json}");
        }
    }
```

Append to the `tests` module in `crates/core/src/voting.rs`:

```rust
    #[test]
    fn weight_json_is_a_plain_number() {
        assert_eq!(serde_json::to_string(&Weight(1_000)).unwrap(), "1000");
        assert_eq!(serde_json::from_str::<Weight>("1000").unwrap(), Weight(1_000));
        assert_eq!(serde_json::from_str::<Weight>("10000").unwrap(), MAX_WEIGHT);
    }

    #[test]
    fn weight_json_above_the_maximum_is_rejected() {
        assert!(serde_json::from_str::<Weight>("10001").is_err());
        assert!(serde_json::from_str::<Weight>("4294967295").is_err());
        assert!(serde_json::from_str::<Weight>("-1").is_err());
        assert_eq!(Weight::try_from(10_001), Err(WeightError(10_001)));
    }

    #[test]
    fn a_vote_with_an_oversized_weight_is_rejected() {
        let json = r#"{"install":"b","answer":"yes","weight":99999}"#;
        assert!(serde_json::from_str::<Vote>(json).is_err());
    }
```

In `crates/core/src/link.rs`, **replace** the test `short_links_are_left_for_the_server` with:

```rust
    #[test]
    fn short_links_are_left_for_the_server() {
        assert_eq!(
            parse("https://maps.app.goo.gl/AbC123"),
            Ok(Destination::ShortLink {
                url: "https://maps.app.goo.gl/AbC123".into()
            })
        );
        assert_eq!(
            parse("https://goo.gl/maps/xyz"),
            Ok(Destination::ShortLink {
                url: "https://goo.gl/maps/xyz".into()
            })
        );
    }

    #[test]
    fn short_links_are_tidied_to_https_host_and_path() {
        for link in [
            "https://maps.app.goo.gl/AbC123?g_st=ic",
            "http://maps.app.goo.gl/AbC123",
            "https://maps.app.goo.gl/AbC123#here",
            "HTTPS://MAPS.APP.GOO.GL/AbC123",
            "https://user@maps.app.goo.gl:443/AbC123?g_st=iw&x=1",
        ] {
            assert_eq!(
                parse(link),
                Ok(Destination::ShortLink {
                    url: "https://maps.app.goo.gl/AbC123".into()
                }),
                "{link}"
            );
        }
    }

    #[test]
    fn a_short_link_without_an_id_has_no_destination() {
        for link in [
            "https://maps.app.goo.gl/",
            "https://maps.app.goo.gl",
            "https://goo.gl/maps/",
        ] {
            assert_eq!(parse(link), Err(LinkError::NoDestination), "{link}");
        }
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p carsharing-core`
Expected: compile error `cannot find type WeightError`. Temporarily comment out the two
`WeightError` lines to see the rest fail: `json_rejects_points_off_the_earth`,
`weight_json_above_the_maximum_is_rejected`, `a_vote_with_an_oversized_weight_is_rejected`,
`short_links_are_tidied_to_https_host_and_path` and `a_short_link_without_an_id_has_no_destination`
FAIL. Restore the lines.

- [ ] **Step 3: Implement**

In `crates/core/src/geo.rs`, replace the `Coordinate` definition with:

```rust
/// A point in 1e-7 degrees, the precision OpenStreetMap uses. JSON goes through
/// [`Coordinate::new`], so a point off the Earth cannot be read from a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "RawCoordinate")]
pub struct Coordinate {
    pub lat_e7: i32,
    pub lon_e7: i32,
}

/// The JSON shape of a [`Coordinate`] before its range is checked.
#[derive(Deserialize)]
struct RawCoordinate {
    lat_e7: i32,
    lon_e7: i32,
}

impl TryFrom<RawCoordinate> for Coordinate {
    type Error = CoordinateError;

    fn try_from(raw: RawCoordinate) -> Result<Coordinate, CoordinateError> {
        Coordinate::new(raw.lat_e7, raw.lon_e7)
    }
}
```

In `crates/core/src/voting.rs`, add `use thiserror::Error;` and replace the `Weight`
definition with:

```rust
/// How much one install's vote counts, from 0 to [`MAX_WEIGHT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Weight(pub u32);

pub const MAX_WEIGHT: Weight = Weight(10_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("weight {0} is above the maximum of 10000")]
pub struct WeightError(pub u32);

impl TryFrom<u32> for Weight {
    type Error = WeightError;

    fn try_from(value: u32) -> Result<Weight, WeightError> {
        if value > MAX_WEIGHT.0 {
            return Err(WeightError(value));
        }
        Ok(Weight(value))
    }
}

impl From<Weight> for u32 {
    fn from(weight: Weight) -> u32 {
        weight.0
    }
}
```

`weight()` already returns at most `MAX_WEIGHT` (100 age points × at most 100); keep its
`unwrap_or(MAX_WEIGHT.0)`.

In `crates/core/src/link.rs`, change `parse_link`'s short-link branch to:

```rust
    if is_short_link(&url) {
        return short_link(&url);
    }
```

and add below `is_short_link`:

```rust
/// The short link as `https://host/path`: the scheme forced to https, and the user, port,
/// query (share tracking such as `?g_st=ic`) and fragment dropped, so the server fetches only
/// what identifies the link. A short link without an ID has no destination.
fn short_link(url: &Url) -> Result<Destination, LinkError> {
    let host = url.host_str().unwrap_or_default();
    let id = match host {
        "goo.gl" => url.path().strip_prefix("/maps/"),
        _ => url.path().strip_prefix('/'),
    };
    if id.is_none_or(str::is_empty) {
        return Err(LinkError::NoDestination);
    }
    Ok(Destination::ShortLink {
        url: format!("https://{host}{}", url.path()),
    })
}
```

`is_short_link` stays as it is (`goo.gl` needs a path starting with `/maps/`;
`https://goo.gl/maps` without the slash is `NotGoogleMaps`, as before).

In `crates/core/src/lib.rs`, change the voting re-export to:

```rust
pub use voting::{
    Answer, InstallId, InstallRecord, Vote, Weight, WeightError, is_accepted, weight,
};
```

(`cargo fmt` decides the final line breaks.)

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p carsharing-core && cargo clippy -p carsharing-core --all-targets -- -D warnings`
Expected: PASS, no warnings. The existing tests `finds_the_link_inside_shared_text`,
`punctuation_around_the_link_is_ignored` and `a_later_maps_link_wins_over_an_earlier_other_link`
still pass unchanged.

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "fix(core): range-check coordinates and weights read from JSON; tidy short links

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Core — settling a decided report, and today's date from the clock

The server needs two more pure rules: who was right once a report is decided (to update each
install's accuracy, which feeds `weight`), and turning the system clock into a `Day`.

**Files:**
- Modify: `crates/core/src/voting.rs`
- Modify: `crates/core/src/day.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `InstallId`, `Vote`, `Answer` (Task 1 state).
- Produces:
  - `pub fn settle(reporter: &InstallId, votes: &[Vote], accepted: bool) -> Vec<(InstallId, bool)>`
    — one entry per install (reporter first, then voters sorted by ID), `true` when it was
    right. Uses each install's last vote and ignores the reporter's own votes, exactly like
    `is_accepted`.
  - `Day::from_unix_seconds(i64) -> Day` (floors; saturates at `i32::MIN`/`i32::MAX`).

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `crates/core/src/voting.rs`:

```rust
    fn right(install: &str, was_right: bool) -> (InstallId, bool) {
        (InstallId(install.into()), was_right)
    }

    #[test]
    fn settling_an_accepted_report() {
        let votes = [
            vote("c", Answer::No, 1_000),
            vote("b", Answer::Yes, 1_000),
        ];
        assert_eq!(
            settle(&reporter(), &votes, true),
            vec![right("reporter", true), right("b", true), right("c", false)]
        );
    }

    #[test]
    fn settling_a_report_that_lost() {
        let votes = [vote("b", Answer::Yes, 1_000), vote("c", Answer::No, 1_000)];
        assert_eq!(
            settle(&reporter(), &votes, false),
            vec![right("reporter", false), right("b", false), right("c", true)]
        );
    }

    #[test]
    fn settling_uses_the_last_vote_and_ignores_the_reporters_votes() {
        let votes = [
            vote("b", Answer::No, 1_000),
            vote("reporter", Answer::No, 1_000),
            vote("b", Answer::Yes, 1_000),
        ];
        assert_eq!(
            settle(&reporter(), &votes, true),
            vec![right("reporter", true), right("b", true)]
        );
    }
```

Append to the `tests` module in `crates/core/src/day.rs`:

```rust
    #[test]
    fn days_from_unix_seconds() {
        assert_eq!(Day::from_unix_seconds(0), Day(0));
        assert_eq!(Day::from_unix_seconds(86_399), Day(0));
        assert_eq!(Day::from_unix_seconds(86_400), Day(1));
        // 2026-10-19T12:00:00Z
        assert_eq!(Day::from_unix_seconds(1_792_411_200), Day(20_745));
    }

    #[test]
    fn days_before_1970_round_down() {
        assert_eq!(Day::from_unix_seconds(-1), Day(-1));
        assert_eq!(Day::from_unix_seconds(-86_400), Day(-1));
        assert_eq!(Day::from_unix_seconds(-86_401), Day(-2));
    }

    #[test]
    fn extreme_seconds_saturate() {
        assert_eq!(Day::from_unix_seconds(i64::MAX), Day(i32::MAX));
        assert_eq!(Day::from_unix_seconds(i64::MIN), Day(i32::MIN));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p carsharing-core`
Expected: compile errors `cannot find function settle` and
`no function or associated item named from_unix_seconds`.

- [ ] **Step 3: Implement**

In `crates/core/src/voting.rs`, extract the "last vote per install" loop of `is_accepted`
into a helper and add `settle`:

```rust
/// Each install's last vote, in no particular order; the reporter's own votes are left out.
fn last_votes<'a>(reporter: &InstallId, votes: &'a [Vote]) -> HashMap<&'a InstallId, &'a Vote> {
    let mut last = HashMap::new();
    for vote in votes.iter().filter(|vote| &vote.install != reporter) {
        last.insert(&vote.install, vote);
    }
    last
}

/// Who was right once a report is decided, to update each install's [`InstallRecord`]: the
/// reporter was right if the report was accepted; a voter was right if its last vote was yes on
/// an accepted report or no on one that was not. The reporter comes first, then voters by ID.
pub fn settle(reporter: &InstallId, votes: &[Vote], accepted: bool) -> Vec<(InstallId, bool)> {
    let mut voters: Vec<(InstallId, bool)> = last_votes(reporter, votes)
        .into_values()
        .map(|vote| (vote.install.clone(), (vote.answer == Answer::Yes) == accepted))
        .collect();
    voters.sort();
    let mut settled = vec![(reporter.clone(), accepted)];
    settled.extend(voters);
    settled
}
```

and make `is_accepted` start with `let last_votes = last_votes(reporter, votes);` instead of
building the map itself (the rest of its body is unchanged).

In `crates/core/src/day.rs`, add to `impl Day`:

```rust
    pub const SECONDS_PER_DAY: i64 = 86_400;

    /// The UTC day containing this Unix time; dates beyond `i32` days saturate.
    pub fn from_unix_seconds(seconds: i64) -> Day {
        let days = seconds.div_euclid(Self::SECONDS_PER_DAY);
        Day(i32::try_from(days).unwrap_or(if days < 0 { i32::MIN } else { i32::MAX }))
    }
```

In `crates/core/src/lib.rs`, add `settle` to the `voting` re-export.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p carsharing-core && cargo clippy -p carsharing-core --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "feat(core): settle decided reports; days from Unix time

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 3: Server skeleton — crate, configuration, health check, private request logs

**Files:**
- Modify: `Cargo.toml` (workspace dependencies)
- Create: `crates/server/Cargo.toml`
- Create: `crates/server/src/main.rs`, `lib.rs`, `cli.rs`, `config.rs`, `state.rs`, `app.rs`, `error.rs`
- Create: `crates/server/tests/health.rs`
- Create: `.env.example`

**Interfaces:**
- Consumes: `carsharing_core::Day`.
- Produces (later tasks extend these; each extension is spelled out in its task):
  - `whichride_server::state::AppState` — `#[derive(Clone)]`, fields added over the plan:
    `today: Arc<dyn Fn() -> Day + Send + Sync>` (this task), `db: PgPool` and
    `providers: Arc<Catalogue>` (Task 4), `integrity: Arc<Integrity>` (Task 6),
    `ors: Arc<Ors>` and `limits: Arc<RateLimits>` (Task 10), `expander: Arc<Expander>` (Task 11).
  - `whichride_server::app::router(state: AppState) -> axum::Router`.
  - `whichride_server::error::ApiError` (all variants below) implementing `IntoResponse`.
  - `whichride_server::config::{ServeConfig, ConfigError, IntegrityMode}` with
    `ServeConfig::from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<ServeConfig, ConfigError>`.
  - `whichride_server::cli::{Cli, Command}`; `Command::Serve` and `Command::Migrate` now, owner
    commands in Task 12.
  - `whichride_server::system_today() -> Day`.

- [ ] **Step 1: Add the workspace dependencies**

Add to `[workspace.dependencies]` in the root `Cargo.toml` (keep alphabetical order):

```toml
anyhow = "1.0.104"
axum = { version = "0.8.9", default-features = false, features = ["http1", "json", "matched-path", "tokio", "tracing"] }
base64 = "0.22"
clap = { version = "4.6.7", features = ["derive"] }
getrandom = "0.4.3"
hex = "0.4"
http-body-util = "0.1.5"
reqwest = { version = "0.13", default-features = false, features = ["json", "query", "rustls-no-provider"] }
rustls = { version = "0.23", default-features = false, features = ["ring", "std"] }
sha2 = "0.11"
sqlx = { version = "0.9", default-features = false, features = ["json", "macros", "migrate", "postgres", "runtime-tokio", "tls-rustls-ring-webpki", "uuid"] }
tokio = { version = "1.53", features = ["macros", "rt-multi-thread"] }
tower = { version = "0.5.3", default-features = false, features = ["util"] }
tower-http = { version = "0.7.1", features = ["catch-panic", "limit", "request-id", "trace"] }
tracing = "0.1.44"
tracing-subscriber = { version = "0.3.23", default-features = false, features = ["ansi", "env-filter", "fmt"] }
uuid = { version = "1", features = ["serde", "v4"] }
whichride-server = { path = "crates/server" }
wiremock = "0.6"
```

Before going on, check two version assumptions and adjust the lines if they are wrong:
`cargo info reqwest@0.13` must list a `rustls-no-provider` feature (it uses rustls with a
crypto provider we install ourselves: ring, as sqlx, because aws-lc-rs needs cmake in the
Alpine Docker build), and `cargo tree -i rustls` after the first build must show a single
`rustls` 0.23.x. If `sqlx` 0.9 renamed a feature, `cargo info sqlx@0.9` shows the new name.

- [ ] **Step 2: Create the crate**

`crates/server/Cargo.toml`:

```toml
[package]
name = "whichride-server"
version = "0.1.0"
description = "whichride's server: catalogue, crowd prices, routing proxy and owner commands"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
repository.workspace = true

[lib]
name = "whichride_server"

[[bin]]
name = "whichride-server"
path = "src/main.rs"

[dependencies]
anyhow.workspace = true
axum.workspace = true
base64.workspace = true
carsharing-core.workspace = true
clap.workspace = true
getrandom.workspace = true
hex.workspace = true
reqwest.workspace = true
rustls.workspace = true
serde.workspace = true
serde_json.workspace = true
sha2.workspace = true
sqlx.workspace = true
thiserror.workspace = true
tokio = { workspace = true, features = ["net", "signal", "sync", "time"] }
tower-http.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
url.workspace = true
uuid.workspace = true

[dev-dependencies]
http-body-util.workspace = true
tower.workspace = true
wiremock.workspace = true

[lints]
workspace = true
```

Some dependencies are first used in later tasks; `cargo` does not warn about unused
dependencies, and adding them once keeps every later task's diff to code.

- [ ] **Step 3: Write the failing tests**

`crates/server/src/config.rs` — tests at the bottom (implementation goes above them in Step 5):

```rust
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |name| map.get(name).cloned()
    }

    const MINIMAL: &[(&str, &str)] = &[
        ("ORS_API_KEY", "key"),
        ("ANDROID_PACKAGE", "app.whichride"),
    ];

    #[test]
    fn defaults() {
        let config = ServeConfig::from_lookup(lookup(MINIMAL)).unwrap();
        assert_eq!(config.bind_addr, "127.0.0.1:8080".parse().unwrap());
        assert_eq!(config.ors_base_url.as_str(), "https://api.openrouteservice.org/");
        assert_eq!(config.ors_api_key, "key");
        assert_eq!(
            config.integrity,
            IntegrityMode::Google {
                package: "app.whichride".into()
            }
        );
    }

    #[test]
    fn the_routing_key_is_required() {
        let error = ServeConfig::from_lookup(lookup(&[("ANDROID_PACKAGE", "p")])).unwrap_err();
        assert_eq!(error, ConfigError::Missing("ORS_API_KEY"));
    }

    #[test]
    fn google_integrity_needs_the_package_name() {
        let error = ServeConfig::from_lookup(lookup(&[("ORS_API_KEY", "k")])).unwrap_err();
        assert_eq!(error, ConfigError::Missing("ANDROID_PACKAGE"));
    }

    #[test]
    fn integrity_can_be_switched_off_locally() {
        let config = ServeConfig::from_lookup(lookup(&[
            ("ORS_API_KEY", "k"),
            ("PLAY_INTEGRITY", "off"),
        ]))
        .unwrap();
        assert_eq!(config.integrity, IntegrityMode::Off);
    }

    #[test]
    fn integrity_cannot_be_switched_off_on_cloud_run() {
        let error = ServeConfig::from_lookup(lookup(&[
            ("ORS_API_KEY", "k"),
            ("PLAY_INTEGRITY", "off"),
            ("K_SERVICE", "whichride"),
        ]))
        .unwrap_err();
        assert!(matches!(error, ConfigError::Invalid { name: "PLAY_INTEGRITY", .. }));
    }

    #[test]
    fn unknown_values_are_rejected() {
        let mut pairs = MINIMAL.to_vec();
        pairs.push(("PLAY_INTEGRITY", "maybe"));
        assert!(ServeConfig::from_lookup(lookup(&pairs)).is_err());
        let mut pairs = MINIMAL.to_vec();
        pairs.push(("BIND_ADDR", "nowhere"));
        assert!(ServeConfig::from_lookup(lookup(&pairs)).is_err());
        let mut pairs = MINIMAL.to_vec();
        pairs.push(("ORS_BASE_URL", "not a url"));
        assert!(ServeConfig::from_lookup(lookup(&pairs)).is_err());
    }

    #[test]
    fn blank_values_count_as_missing() {
        let error = ServeConfig::from_lookup(lookup(&[
            ("ORS_API_KEY", "  "),
            ("ANDROID_PACKAGE", "p"),
        ]))
        .unwrap_err();
        assert_eq!(error, ConfigError::Missing("ORS_API_KEY"));
    }

    #[test]
    fn debug_output_hides_the_key() {
        let config = ServeConfig::from_lookup(lookup(&[
            ("ORS_API_KEY", "secret-key-123"),
            ("ANDROID_PACKAGE", "p"),
        ]))
        .unwrap();
        assert!(!format!("{config:?}").contains("secret-key-123"));
    }
}
```

`crates/server/src/cli.rs` — tests at the bottom:

```rust
#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn parse(args: &[&str]) -> Result<Command, clap::Error> {
        Cli::try_parse_from(std::iter::once("whichride-server").chain(args.iter().copied()))
            .map(|cli| cli.command)
    }

    #[test]
    fn the_command_line_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn every_command_parses() {
        assert_eq!(parse(&["serve"]).unwrap(), Command::Serve);
        assert_eq!(parse(&["migrate"]).unwrap(), Command::Migrate);
        assert!(parse(&["unknown"]).is_err());
    }
}
```

`crates/server/src/error.rs` — tests at the bottom:

```rust
#[cfg(test)]
mod tests {
    use axum::body::to_bytes;

    use super::*;

    async fn render(error: ApiError) -> (StatusCode, String) {
        let response = error.into_response();
        let status = response.status();
        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn errors_are_json_codes() {
        assert_eq!(
            render(ApiError::RouteServiceBusy).await,
            (
                StatusCode::SERVICE_UNAVAILABLE,
                r#"{"error":"route_service_busy"}"#.into()
            )
        );
        assert_eq!(
            render(ApiError::OfferLocked).await,
            (StatusCode::CONFLICT, r#"{"error":"offer_locked"}"#.into())
        );
    }

    #[tokio::test]
    async fn internal_errors_reveal_nothing() {
        let (status, body) = render(ApiError::internal("password=hunter2 at 52.52,13.40")).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body, r#"{"error":"internal"}"#);
    }
}
```

`crates/server/tests/health.rs`:

```rust
use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use carsharing_core::Day;
use tower::ServiceExt;
use whichride_server::app::router;
use whichride_server::state::AppState;

#[tokio::test]
async fn health_answers_ok() {
    let state = AppState {
        today: Arc::new(|| Day(20_745)),
    };
    let response = router(state)
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("x-request-id"));
    let body = to_bytes(response.into_body(), 64).await.unwrap();
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn unknown_paths_are_json_404s() {
    let state = AppState {
        today: Arc::new(|| Day(20_745)),
    };
    let response = router(state)
        .oneshot(Request::get("/nope").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = to_bytes(response.into_body(), 64).await.unwrap();
    assert_eq!(&body[..], br#"{"error":"not_found"}"#);
}
```

(Task 4 replaces the literal `AppState { … }` here with the shared `common::test_state()`
helper once the state has a database.)

- [ ] **Step 4: Run the tests to verify they fail**

Run: `cargo test -p whichride-server`
Expected: compile errors (`ServeConfig`, `Cli`, `ApiError`, `router`, `AppState` not found).

- [ ] **Step 5: Implement**

`crates/server/src/lib.rs`:

```rust
//! whichride's server: the provider catalogue with crowd prices, price reports and votes from
//! genuine app installs, a routing proxy that stores no locations, short-link expansion, and the
//! owner's emergency-switch commands. Every price and voting rule comes from `carsharing-core`.

pub mod app;
pub mod cli;
pub mod config;
pub mod error;
pub mod state;

use std::time::{SystemTime, UNIX_EPOCH};

use carsharing_core::Day;

/// Today's UTC date from the system clock.
pub fn system_today() -> Day {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX));
    Day::from_unix_seconds(seconds)
}
```

`crates/server/src/state.rs`:

```rust
//! What every request handler can reach.

use std::sync::Arc;

use carsharing_core::Day;

#[derive(Clone)]
pub struct AppState {
    /// Today's date; tests replace the clock.
    pub today: Arc<dyn Fn() -> Day + Send + Sync>,
}
```

`crates/server/src/error.rs` (implementation above the tests):

```rust
//! Every error a client can see, as `{"error": "<code>"}` with a fitting status.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    BadRequest,
    Unauthorized,
    IntegrityFailed,
    NotFound,
    OfferLocked,
    ReportClosed,
    NotAShortLink,
    NoDestination,
    NoRoute,
    TooManyRequests,
    IntegrityUnavailable,
    RouteServiceBusy,
    /// Logged with its message; the client sees only `internal`.
    Internal(String),
}

impl ApiError {
    /// An unexpected failure. The message goes to the log, so it must never contain request
    /// data (coordinates, search text, links, tokens).
    pub fn internal(message: impl Into<String>) -> ApiError {
        ApiError::Internal(message.into())
    }

    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            ApiError::BadRequest => (StatusCode::BAD_REQUEST, "bad_request"),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            ApiError::IntegrityFailed => (StatusCode::FORBIDDEN, "integrity_failed"),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            ApiError::OfferLocked => (StatusCode::CONFLICT, "offer_locked"),
            ApiError::ReportClosed => (StatusCode::CONFLICT, "report_closed"),
            ApiError::NotAShortLink => (StatusCode::UNPROCESSABLE_ENTITY, "not_a_short_link"),
            ApiError::NoDestination => (StatusCode::UNPROCESSABLE_ENTITY, "no_destination"),
            ApiError::NoRoute => (StatusCode::UNPROCESSABLE_ENTITY, "no_route"),
            ApiError::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, "too_many_requests"),
            ApiError::IntegrityUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, "integrity_unavailable")
            }
            ApiError::RouteServiceBusy => (StatusCode::SERVICE_UNAVAILABLE, "route_service_busy"),
            ApiError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let ApiError::Internal(message) = &self {
            tracing::error!(%message, "request failed");
        }
        let (status, code) = self.status_and_code();
        (status, Json(json!({ "error": code }))).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> ApiError {
        ApiError::internal(format!("database: {error}"))
    }
}
```

sqlx errors carry SQL text and database messages, never bound parameter values, so they are safe
to log; queries in this crate bind every value as a parameter.

`crates/server/src/config.rs` (implementation above the tests):

```rust
//! Configuration from the environment only; `.env.example` lists every variable.

use std::fmt;
use std::net::SocketAddr;

use thiserror::Error;
use url::Url;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("{0} is not set (see .env.example)")]
    Missing(&'static str),
    #[error("{name} is invalid: {reason}")]
    Invalid { name: &'static str, reason: String },
}

/// How Play Integrity tokens are checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrityMode {
    /// Verdicts from Google for this Android package.
    Google { package: String },
    /// No checks: for running the server next to an emulator. Refused on Cloud Run.
    Off,
}

/// What `serve` needs besides `DATABASE_URL`.
#[derive(Clone, PartialEq, Eq)]
pub struct ServeConfig {
    pub bind_addr: SocketAddr,
    pub ors_api_key: String,
    pub ors_base_url: Url,
    pub integrity: IntegrityMode,
}

impl fmt::Debug for ServeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServeConfig")
            .field("bind_addr", &self.bind_addr)
            .field("ors_api_key", &"<hidden>")
            .field("ors_base_url", &self.ors_base_url.as_str())
            .field("integrity", &self.integrity)
            .finish()
    }
}

impl ServeConfig {
    pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";
    pub const DEFAULT_ORS_BASE_URL: &str = "https://api.openrouteservice.org";

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<ServeConfig, ConfigError> {
        let get = |name: &str| get(name).filter(|value| !value.trim().is_empty());
        let bind_addr = get("BIND_ADDR")
            .unwrap_or_else(|| Self::DEFAULT_BIND_ADDR.to_owned())
            .trim()
            .parse()
            .map_err(|e| invalid("BIND_ADDR", e))?;
        let ors_api_key = get("ORS_API_KEY").ok_or(ConfigError::Missing("ORS_API_KEY"))?;
        let ors_base_url = get("ORS_BASE_URL")
            .unwrap_or_else(|| Self::DEFAULT_ORS_BASE_URL.to_owned());
        let ors_base_url = Url::parse(ors_base_url.trim()).map_err(|e| invalid("ORS_BASE_URL", e))?;
        let integrity = match get("PLAY_INTEGRITY").as_deref().map(str::trim) {
            None | Some("google") => IntegrityMode::Google {
                package: get("ANDROID_PACKAGE")
                    .ok_or(ConfigError::Missing("ANDROID_PACKAGE"))?
                    .trim()
                    .to_owned(),
            },
            Some("off") if get("K_SERVICE").is_some() => {
                return Err(invalid("PLAY_INTEGRITY", "cannot be off on Cloud Run"));
            }
            Some("off") => IntegrityMode::Off,
            Some(other) => {
                return Err(invalid("PLAY_INTEGRITY", format!("expected google or off, got {other:?}")));
            }
        };
        Ok(ServeConfig {
            bind_addr,
            ors_api_key: ors_api_key.trim().to_owned(),
            ors_base_url,
            integrity,
        })
    }
}

/// `DATABASE_URL`, needed by every command.
pub fn database_url(get: impl Fn(&str) -> Option<String>) -> Result<String, ConfigError> {
    get("DATABASE_URL")
        .filter(|url| !url.trim().is_empty())
        .ok_or(ConfigError::Missing("DATABASE_URL"))
}

fn invalid(name: &'static str, reason: impl fmt::Display) -> ConfigError {
    ConfigError::Invalid {
        name,
        reason: reason.to_string(),
    }
}
```

(`K_SERVICE` is set by Cloud Run in every container; it is how the server knows it is in
production.)

`crates/server/src/cli.rs` (implementation above the tests):

```rust
//! The command line. Owner commands (Task 12) are the emergency switch of spec §7.4: they run
//! against the production database from the owner's machine or Cloud Shell, never over HTTP.

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "whichride-server",
    version,
    about = "whichride's server and the owner's emergency switch"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Apply migrations, add new offers from the seed catalogue, then serve on BIND_ADDR.
    Serve,
    /// Apply migrations and add new offers from the seed catalogue, then exit.
    Migrate,
}
```

`crates/server/src/app.rs`:

```rust
//! The HTTP router and its middleware.

use axum::Router;
use axum::extract::{DefaultBodyLimit, MatchedPath, Request};
use axum::routing::get;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::error::ApiError;
use crate::state::AppState;

/// Largest request body accepted; real requests are well under 2 KiB.
pub const BODY_LIMIT_BYTES: usize = 16 * 1024;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .fallback(|| async { ApiError::NotFound })
        .with_state(state)
        .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
        .layer(CatchPanicLayer::new())
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(request_span)
                .on_request(())
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
}

/// The log span of a request: method, the *route template* (`/v1/reports/{id}/votes`, never the
/// actual path, so nothing a client puts in a URL reaches the logs) and the request ID. Headers
/// and bodies are never logged (spec §9).
fn request_span(request: &Request) -> tracing::Span {
    let route = request
        .extensions()
        .get::<MatchedPath>()
        .map_or("unmatched", MatchedPath::as_str);
    let id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("-");
    tracing::info_span!("request", method = %request.method(), route, id)
}
```

Later tasks add their routes in `router` above `.fallback(…)`. If `MatchedPath` is not
available in `request_span` because the trace layer wraps the whole router (check with the
Task 10 log test), move `TraceLayer` to `Router::route_layer` — the test, not this note, is the
judge.

`crates/server/src/main.rs`:

```rust
use std::process::ExitCode;

use clap::Parser;
use whichride_server::cli::{Cli, Command};

#[tokio::main]
async fn main() -> ExitCode {
    init_logging();
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Serve => whichride_server::commands::serve().await,
        Command::Migrate => whichride_server::commands::migrate().await,
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .with_writer(std::io::stderr)
        .init();
}
```

Create `crates/server/src/commands.rs` and add `pub mod commands;` to `lib.rs`:

```rust
//! What each CLI command does. `serve` grows as later tasks add state.

use std::sync::Arc;

use anyhow::Context;
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::app::router;
use crate::config::ServeConfig;
use crate::state::AppState;

pub async fn serve() -> anyhow::Result<()> {
    let config = ServeConfig::from_lookup(|name| std::env::var(name).ok())?;
    let state = AppState {
        today: Arc::new(crate::system_today),
    };
    let listener = TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("cannot listen on {}", config.bind_addr))?;
    info!(addr = %config.bind_addr, "listening");
    axum::serve(listener, router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

pub async fn migrate() -> anyhow::Result<()> {
    anyhow::bail!("the database arrives in Task 4")
}

/// Resolves on Ctrl-C, or on SIGTERM (what Cloud Run and `docker stop` send).
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                warn!(%error, "cannot listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    info!("shutting down");
}
```

`.env.example` at the repo root (no values that look like secrets):

```sh
# Copy to .env (git-ignored) and fill in. The server reads only the environment.
DATABASE_URL=postgres://postgres@localhost:5432/whichride
BIND_ADDR=127.0.0.1:8080
# Free key from https://openrouteservice.org/dev/#/signup
ORS_API_KEY=
# ORS_BASE_URL=https://api.openrouteservice.org
# google (default) or off; off is refused on Cloud Run
PLAY_INTEGRITY=off
# Android application ID, needed when PLAY_INTEGRITY=google
ANDROID_PACKAGE=
RUST_LOG=info
```

Check that `.gitignore` already ignores `.env`; if not, add a line `.env`.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p whichride-server && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: PASS, no warnings.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock crates/server .env.example .gitignore
git commit -m "feat(server): crate skeleton with configuration, health check and private request logs

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 4: Database — schema, seed catalogue, test harness, CI Postgres

**Files:**
- Create: `crates/server/migrations/0001_initial.sql`
- Create: `crates/server/catalogue.json` (**OWNER** enters the prices)
- Create: `crates/server/src/db.rs`
- Modify: `crates/server/src/lib.rs`, `state.rs`, `commands.rs`
- Create: `crates/server/tests/common/mod.rs`, `crates/server/tests/common/catalogue.json`
- Create: `crates/server/tests/database.rs`
- Modify: `crates/server/tests/health.rs`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `Catalogue`, `RateCard`, `Day` (core); `AppState` (Task 3).
- Produces:
  - `db::MIGRATOR`, `db::connect(url: &str) -> sqlx::Result<PgPool>`,
    `db::prepare(pool: &PgPool, seed: &Catalogue, today: Day) -> anyhow::Result<()>`
    (migrate, then `seed_offers`), `db::seed_offers(pool, seed, today) -> sqlx::Result<u64>`
    (returns how many offers were new), `db::seed_catalogue() -> Result<Catalogue, CatalogueError>`.
  - `db::OfferRow { id: String, provider_id: String, name: String, cards: Json<Vec<RateCard>>, confirmed_on: Option<i32>, locked: bool }`
    and `db::list_offers(pool) -> sqlx::Result<Vec<OfferRow>>` (ordered by `id`).
  - `AppState` gains `db: PgPool` and `providers: Arc<Catalogue>` (the seed catalogue; its
    offers are ignored at runtime, the database's are used).
  - Test helpers in `tests/common/mod.rs`: `TestApp::start() -> Option<TestApp>`,
    `TestApp::{get, post, post_with, set_today}`, `TestApp.pool`, `common::TODAY` (= 20 745).

- [ ] **Step 1: Write the schema**

`crates/server/migrations/0001_initial.sql`:

```sql
-- Days are carsharing_core::Day values: whole days since 1970-01-01 (UTC).
-- Nothing in this schema can hold a location, a route, a search or a link (spec §9).

CREATE TABLE installs (
    id uuid PRIMARY KEY,
    token_hash bytea NOT NULL UNIQUE CHECK (length(token_hash) = 32),
    first_seen integer NOT NULL,
    decided_votes integer NOT NULL DEFAULT 0 CHECK (decided_votes >= 0),
    matching_votes integer NOT NULL DEFAULT 0
        CHECK (matching_votes >= 0 AND matching_votes <= decided_votes)
);

-- The current shared price of each offer.
CREATE TABLE offers (
    id text PRIMARY KEY,
    provider_id text NOT NULL,
    name text NOT NULL,
    cards jsonb NOT NULL,
    confirmed_on integer,
    locked boolean NOT NULL DEFAULT false
);

-- A price report targets an existing offer, or proposes a new offer of a provider (spec §5.1);
-- an accepted proposal gets its offer_id when the offer is created.
CREATE TABLE reports (
    id uuid PRIMARY KEY,
    offer_id text REFERENCES offers (id),
    proposed_provider text,
    proposed_name text,
    reporter uuid NOT NULL REFERENCES installs (id),
    reporter_weight integer NOT NULL CHECK (reporter_weight BETWEEN 0 AND 10000),
    cards jsonb NOT NULL,
    created_on integer NOT NULL,
    status text NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'accepted', 'superseded', 'reverted')),
    decided_on integer,
    CHECK (offer_id IS NOT NULL OR (proposed_provider IS NOT NULL AND proposed_name IS NOT NULL))
);
CREATE INDEX reports_pending_by_offer ON reports (offer_id, created_on) WHERE status = 'pending';
CREATE INDEX reports_pending_proposals ON reports (proposed_provider, lower(proposed_name))
    WHERE status = 'pending' AND offer_id IS NULL;

-- Votes in the order they were cast; the weight is the voter's weight at that moment.
CREATE TABLE votes (
    seq bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    report_id uuid NOT NULL REFERENCES reports (id),
    install_id uuid NOT NULL REFERENCES installs (id),
    answer text NOT NULL CHECK (answer IN ('yes', 'no')),
    weight integer NOT NULL CHECK (weight BETWEEN 0 AND 10000)
);
CREATE INDEX votes_by_report ON votes (report_id, seq);

-- Every value an offer has had, newest last, so the owner can revert (spec §7.4). Rows are
-- never deleted; a revert marks the row it undoes.
CREATE TABLE price_history (
    seq bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    offer_id text NOT NULL REFERENCES offers (id),
    cards jsonb NOT NULL,
    report_id uuid REFERENCES reports (id), -- NULL: from the seed catalogue
    from_day integer NOT NULL,
    reverted boolean NOT NULL DEFAULT false
);
CREATE INDEX price_history_by_offer ON price_history (offer_id, seq);
```

- [ ] **Step 2: OWNER — enter the starting prices**

Stop and ask the owner for the current rate cards from the providers' apps (spec §5: "The
owner enters the initial prices"). Show them this template of `crates/server/catalogue.json`
and ask them to fill in or correct every value, add one `offers` entry per Bolt Drive and
Free2move car model they see in the apps, and confirm the MILES classes:

```json
{
  "providers": [
    {
      "id": "miles",
      "name": "MILES",
      "android_package": "com.driveby.app",
      "refuel_credit": null,
      "offers": [
        { "id": "miles.s-m", "name": "S/M", "cards": [
          { "label": "Standard", "kind": "standard",
            "base_fee": 0, "per_km": 0, "per_minute": 0, "minimum": 0,
            "km_rounding": "started_unit", "minute_rounding": "started_unit" } ] },
        { "id": "miles.premium", "name": "Premium", "cards": [ … ] },
        { "id": "miles.l", "name": "L", "cards": [ … ] },
        { "id": "miles.xl", "name": "XL", "cards": [ … ] }
      ]
    },
    {
      "id": "bolt",
      "name": "Bolt Drive",
      "android_package": "ee.mtakso.client",
      "refuel_credit": null,
      "offers": [ { "id": "bolt.<model>", "name": "<Model>", "cards": [ … ] } ]
    },
    {
      "id": "free2move",
      "name": "Free2move",
      "android_package": "com.car2go",
      "refuel_credit": null,
      "offers": [ { "id": "free2move.<model>", "name": "<Model>", "cards": [ … ] } ]
    }
  ]
}
```

Amounts are euro **cents** (`"per_km": 29` is €0.29/km). Packages are extra cards with
`"kind": "package"`, the package price in `base_fee` and its allowance in `included_km` /
`included_minutes`. Offer IDs are lowercase `provider.name-with-dashes`. Write the file exactly
as the owner confirms it; do not invent or look up prices.

Until the owner answers, continue with Step 3 onwards (the tests below use their own fixture,
not this file); the seed test in Step 3 stays failing until the file exists, and this task is
not committed before it passes.

- [ ] **Step 3: Write the failing tests**

`crates/server/tests/common/catalogue.json` — a fixture with made-up prices, used by every
HTTP test:

```json
{
  "providers": [
    {
      "id": "miles",
      "name": "MILES",
      "android_package": "com.driveby.app",
      "refuel_credit": null,
      "offers": [
        { "id": "miles.s", "name": "S", "cards": [
          { "label": "Standard", "kind": "standard", "base_fee": 100, "per_km": 79,
            "per_minute": 0, "minimum": 0,
            "km_rounding": "started_unit", "minute_rounding": "started_unit" } ] },
        { "id": "miles.l", "name": "L", "cards": [
          { "label": "Standard", "kind": "standard", "base_fee": 100, "per_km": 109,
            "per_minute": 0, "minimum": 500,
            "km_rounding": "started_unit", "minute_rounding": "started_unit" } ] }
      ]
    },
    {
      "id": "bolt",
      "name": "Bolt Drive",
      "android_package": "ee.mtakso.client",
      "refuel_credit": null,
      "offers": [
        { "id": "bolt.corsa", "name": "Opel Corsa", "cards": [
          { "label": "Standard", "kind": "standard", "base_fee": 0, "per_km": 18,
            "per_minute": 25, "minimum": 300,
            "km_rounding": "started_unit", "minute_rounding": "started_unit" } ] }
      ]
    }
  ]
}
```

`crates/server/tests/common/mod.rs`:

```rust
//! Shared by the HTTP tests: a fresh database per test on `TEST_POSTGRES_URL`, the app wired
//! to it with a fixed clock, and request helpers.
#![allow(dead_code)] // each test file uses a different subset

use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use carsharing_core::{Catalogue, Day};
use serde_json::Value;
use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
use tower::ServiceExt;
use whichride_server::app::router;
use whichride_server::db;
use whichride_server::state::AppState;

/// 2026-10-19.
pub const TODAY: i32 = 20_745;

static NEXT: AtomicU32 = AtomicU32::new(0);

/// A database that lives as long as this value; dropping it drops the database.
pub struct TestDatabase {
    pub url: String,
    admin_url: String,
    name: String,
}

impl Drop for TestDatabase {
    /// Runs on its own thread and runtime: the test's runtime may be the one dropping this
    /// value. A failure is reported, never a panic inside `drop`.
    fn drop(&mut self) {
        let admin_url = self.admin_url.clone();
        let sql = format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", self.name);
        let dropped = std::thread::spawn(move || -> Result<(), String> {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?;
            runtime.block_on(async {
                let mut admin = PgConnection::connect(&admin_url)
                    .await
                    .map_err(|e| e.to_string())?;
                sqlx::raw_sql(AssertSqlSafe(sql))
                    .execute(&mut admin)
                    .await
                    .map_err(|e| e.to_string())?;
                admin.close().await.map_err(|e| e.to_string())
            })
        })
        .join()
        .unwrap_or_else(|_| Err("the thread dropping it panicked".to_owned()));
        if let Err(error) = dropped {
            eprintln!("could not drop test database {}: {error}", self.name);
        }
    }
}

/// A new, empty database, or `None` outside CI when `TEST_POSTGRES_URL` is not set.
/// The URL must end with a database name (e.g. `/postgres`) and have no query string.
pub async fn fresh_database() -> Option<TestDatabase> {
    let Ok(admin_url) = std::env::var("TEST_POSTGRES_URL") else {
        assert!(
            std::env::var_os("CI").is_none(),
            "TEST_POSTGRES_URL must be set in CI"
        );
        eprintln!("skipped: set TEST_POSTGRES_URL to run the Postgres tests");
        return None;
    };
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after 1970")
        .as_micros();
    let name = format!(
        "whichride_test_{}_{stamp}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let mut admin = PgConnection::connect(&admin_url)
        .await
        .expect("connect to TEST_POSTGRES_URL");
    sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&mut admin)
        .await
        .expect("create a test database");
    admin.close().await.expect("close the admin connection");
    let (server, _) = admin_url
        .rsplit_once('/')
        .expect("TEST_POSTGRES_URL ends with /<database>");
    Some(TestDatabase {
        url: format!("{server}/{name}"),
        admin_url,
        name,
    })
}

pub fn fixture_catalogue() -> Catalogue {
    Catalogue::from_json(include_str!("catalogue.json")).expect("valid fixture")
}

pub struct TestApp {
    pub router: Router,
    pub pool: PgPool,
    today: Arc<AtomicI32>,
    // Last, so the pool closes before the database is dropped.
    _db: TestDatabase,
}

impl TestApp {
    /// The app on a fresh, migrated and seeded database, with today = [`TODAY`].
    pub async fn start() -> Option<TestApp> {
        let database = fresh_database().await?;
        let pool = db::connect(&database.url).await.expect("connect");
        let catalogue = fixture_catalogue();
        db::prepare(&pool, &catalogue, Day(TODAY)).await.expect("prepare");
        let today = Arc::new(AtomicI32::new(TODAY));
        let clock = Arc::clone(&today);
        let state = AppState {
            today: Arc::new(move || Day(clock.load(Ordering::Relaxed))),
            db: pool.clone(),
            providers: Arc::new(catalogue),
        };
        Some(TestApp {
            router: router(state),
            pool,
            today,
            _db: database,
        })
    }

    pub fn set_today(&self, day: i32) {
        self.today.store(day, Ordering::Relaxed);
    }

    pub async fn get(&self, path: &str) -> (StatusCode, Value) {
        self.send(Request::get(path).body(Body::empty()).unwrap()).await
    }

    pub async fn post(&self, path: &str, body: &Value) -> (StatusCode, Value) {
        self.post_with(path, body, &[]).await
    }

    pub async fn post_with(
        &self,
        path: &str,
        body: &Value,
        headers: &[(&str, &str)],
    ) -> (StatusCode, Value) {
        let mut request = Request::post(path).header("content-type", "application/json");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        self.send(request.body(Body::from(body.to_string())).unwrap())
            .await
    }

    pub async fn send(&self, request: Request<Body>) -> (StatusCode, Value) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 1 << 20).await.unwrap();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or_else(|_| {
                Value::String(String::from_utf8_lossy(&bytes).into_owned())
            })
        };
        (status, json)
    }
}
```

`crates/server/tests/database.rs`:

```rust
mod common;

use carsharing_core::Day;
use common::{TODAY, TestApp, fixture_catalogue};
use whichride_server::db;

#[tokio::test]
async fn seeding_adds_every_offer_with_its_history() {
    let Some(app) = TestApp::start().await else { return };
    let offers = db::list_offers(&app.pool).await.unwrap();
    let ids: Vec<&str> = offers.iter().map(|offer| offer.id.as_str()).collect();
    assert_eq!(ids, ["bolt.corsa", "miles.l", "miles.s"]);
    assert!(offers.iter().all(|offer| offer.confirmed_on == Some(TODAY) && !offer.locked));
    let history: i64 = sqlx::query_scalar("SELECT count(*) FROM price_history")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(history, 3);
}

#[tokio::test]
async fn seeding_again_never_overwrites_crowd_prices() {
    let Some(app) = TestApp::start().await else { return };
    sqlx::query("UPDATE offers SET cards = '[]'::jsonb WHERE id = 'miles.s'")
        .execute(&app.pool)
        .await
        .unwrap();
    let added = db::seed_offers(&app.pool, &fixture_catalogue(), Day(TODAY + 5))
        .await
        .unwrap();
    assert_eq!(added, 0);
    let cards: serde_json::Value =
        sqlx::query_scalar("SELECT cards FROM offers WHERE id = 'miles.s'")
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(cards, serde_json::json!([]));
}

#[tokio::test]
async fn a_new_seed_offer_is_added_on_the_next_start() {
    let Some(app) = TestApp::start().await else { return };
    let mut catalogue = fixture_catalogue();
    let mut offer = catalogue.providers[0].offers[0].clone();
    offer.id = carsharing_core::OfferId("miles.xl".into());
    catalogue.providers[0].offers.push(offer);
    let added = db::seed_offers(&app.pool, &catalogue, Day(TODAY + 1)).await.unwrap();
    assert_eq!(added, 1);
}

#[tokio::test]
async fn migrations_run_twice_without_error() {
    let Some(app) = TestApp::start().await else { return };
    db::MIGRATOR.run(&app.pool).await.unwrap();
}

#[test]
fn the_real_seed_catalogue_is_valid() {
    let catalogue = db::seed_catalogue().expect("crates/server/catalogue.json is valid");
    let providers: Vec<(&str, &str)> = catalogue
        .providers
        .iter()
        .map(|p| (p.id.0.as_str(), p.android_package.as_str()))
        .collect();
    // Spec §5: v1 = MILES, Bolt Drive, Free2move, with these Android packages.
    assert_eq!(
        providers,
        [
            ("miles", "com.driveby.app"),
            ("bolt", "ee.mtakso.client"),
            ("free2move", "com.car2go"),
        ]
    );
    for provider in &catalogue.providers {
        for offer in &provider.offers {
            assert!(
                offer.id.0.starts_with(&format!("{}.", provider.id.0)),
                "offer {} must start with its provider's ID",
                offer.id.0
            );
        }
    }
}
```

In `crates/server/tests/health.rs`, replace both literal states with the shared app (and add
`mod common;` at the top):

```rust
mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use common::TestApp;
use tower::ServiceExt;

#[tokio::test]
async fn health_answers_ok() {
    let Some(app) = TestApp::start().await else { return };
    let response = app
        .router
        .clone()
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test]
async fn unknown_paths_are_json_404s() {
    let Some(app) = TestApp::start().await else { return };
    let (status, body) = app.get("/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body, serde_json::json!({"error": "not_found"}));
}
```

- [ ] **Step 4: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:5432/postgres cargo test -p whichride-server`
Expected: compile errors (`whichride_server::db` not found, `AppState` has no field `db`).

- [ ] **Step 5: Implement**

`crates/server/src/db.rs` (add `pub mod db;` to `lib.rs`):

```rust
//! The Postgres connection, migrations and the seed catalogue.

use carsharing_core::{Catalogue, CatalogueError, Day, RateCard};
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::Json;
use sqlx::{FromRow, PgPool};

pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// Providers and starting prices, edited by pull request (spec §5.1).
const SEED_CATALOGUE: &str = include_str!("../catalogue.json");

pub fn seed_catalogue() -> Result<Catalogue, CatalogueError> {
    Catalogue::from_json(SEED_CATALOGUE)
}

/// A small pool: one Cloud Run instance, and Neon's free plan limits connections.
pub async fn connect(url: &str) -> sqlx::Result<PgPool> {
    PgPoolOptions::new().max_connections(5).connect(url).await
}

/// Migrates, then adds the seed catalogue's new offers.
pub async fn prepare(pool: &PgPool, seed: &Catalogue, today: Day) -> anyhow::Result<()> {
    MIGRATOR.run(pool).await?;
    let added = seed_offers(pool, seed, today).await?;
    if added > 0 {
        tracing::info!(added, "offers added from the seed catalogue");
    }
    Ok(())
}

/// Inserts every seed offer the database does not have yet, confirmed today (the owner just
/// read the prices from the apps). Existing offers keep their crowd prices.
pub async fn seed_offers(pool: &PgPool, seed: &Catalogue, today: Day) -> sqlx::Result<u64> {
    let mut tx = pool.begin().await?;
    let mut added = 0;
    for provider in &seed.providers {
        for offer in &provider.offers {
            let inserted = sqlx::query(
                "INSERT INTO offers (id, provider_id, name, cards, confirmed_on)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (id) DO NOTHING",
            )
            .bind(&offer.id.0)
            .bind(&provider.id.0)
            .bind(&offer.name)
            .bind(Json(&offer.cards))
            .bind(today.0)
            .execute(&mut *tx)
            .await?
            .rows_affected();
            if inserted == 1 {
                sqlx::query(
                    "INSERT INTO price_history (offer_id, cards, from_day) VALUES ($1, $2, $3)",
                )
                .bind(&offer.id.0)
                .bind(Json(&offer.cards))
                .bind(today.0)
                .execute(&mut *tx)
                .await?;
                added += 1;
            }
        }
    }
    tx.commit().await?;
    Ok(added)
}

#[derive(Debug, Clone, FromRow)]
pub struct OfferRow {
    pub id: String,
    pub provider_id: String,
    pub name: String,
    pub cards: Json<Vec<RateCard>>,
    pub confirmed_on: Option<i32>,
    pub locked: bool,
}

pub async fn list_offers(pool: &PgPool) -> sqlx::Result<Vec<OfferRow>> {
    sqlx::query_as(
        "SELECT id, provider_id, name, cards, confirmed_on, locked FROM offers ORDER BY id",
    )
    .fetch_all(pool)
    .await
}
```

`crates/server/src/state.rs` becomes:

```rust
//! What every request handler can reach.

use std::sync::Arc;

use carsharing_core::{Catalogue, Day};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    /// Today's date; tests replace the clock.
    pub today: Arc<dyn Fn() -> Day + Send + Sync>,
    pub db: PgPool,
    /// The seed catalogue: provider names, packages and refuel credits. Offers and prices come
    /// from the database.
    pub providers: Arc<Catalogue>,
}
```

In `crates/server/src/commands.rs`, `serve` and `migrate` now use the database:

```rust
pub async fn serve() -> anyhow::Result<()> {
    let get = |name: &str| std::env::var(name).ok();
    let config = ServeConfig::from_lookup(get)?;
    let pool = db::connect(&config::database_url(get)?)
        .await
        .context("cannot connect to the database")?;
    let providers = db::seed_catalogue().context("crates/server/catalogue.json is invalid")?;
    db::prepare(&pool, &providers, crate::system_today()).await?;
    let state = AppState {
        today: Arc::new(crate::system_today),
        db: pool,
        providers: Arc::new(providers),
    };
    // … bind and serve exactly as before
}

pub async fn migrate() -> anyhow::Result<()> {
    let get = |name: &str| std::env::var(name).ok();
    let pool = db::connect(&config::database_url(get)?)
        .await
        .context("cannot connect to the database")?;
    let providers = db::seed_catalogue().context("crates/server/catalogue.json is invalid")?;
    db::prepare(&pool, &providers, crate::system_today()).await?;
    pool.close().await;
    Ok(())
}
```

(with `use crate::{config, db};`).

In `.github/workflows/ci.yml`, give the `check` job a Postgres service and the URL, as in
smoke-tracker:

```yaml
env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings
  TEST_POSTGRES_URL: postgres://postgres@localhost:5432/postgres

jobs:
  check:
    name: fmt · clippy · test
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:17-alpine
        env:
          POSTGRES_HOST_AUTH_METHOD: trust
        ports: ["5432:5432"]
        options: >-
          --health-cmd pg_isready
          --health-interval 5s
          --health-timeout 5s
          --health-retries 10
    steps:
      # unchanged
```

- [ ] **Step 6: Run the tests to verify they pass**

Start a local Postgres if none is running (Global Constraints), then:

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:5432/postgres cargo test -p whichride-server`
Expected: PASS once `catalogue.json` exists (Step 2). Also check that the tests really ran:
the output must not contain `skipped: set TEST_POSTGRES_URL`.

Run: `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`

- [ ] **Step 7: Commit**

```bash
git add crates/server .github/workflows/ci.yml Cargo.lock
git commit -m "feat(server): Postgres schema, seed catalogue with the owner's prices, test harness

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: `GET /v1/catalogue`

The public read endpoint used by the app and, later, the website: providers from the seed,
offers and prices from the database, freshness and the pending change to ask about.

**Files:**
- Create: `crates/server/src/catalogue.rs`
- Modify: `crates/server/src/lib.rs`, `crates/server/src/app.rs`
- Create: `crates/server/tests/catalogue.rs`

**Interfaces:**
- Consumes: `AppState { db, providers }`, `db::list_offers`, `OfferRow` (Task 4).
- Produces: `catalogue::get_catalogue` handler and these response types (all `Serialize`):

```rust
pub struct CatalogueResponse {
    /// Ready for carsharing_core::compare: the seed's providers with the database's offers.
    pub catalogue: Catalogue,
    /// One entry per offer, in the same order as they appear in `catalogue`.
    pub offers: Vec<OfferStatus>,
    /// Pending proposals of new offers (spec §5.1), oldest first.
    pub proposals: Vec<Proposal>,
}
pub struct OfferStatus {
    pub offer: OfferId,
    /// For "confirmed 2 days ago" (spec §7.5).
    pub confirmed_on: Option<Day>,
    /// Owner-locked: the app asks no questions about it.
    pub locked: bool,
    /// The oldest pending report, the one the confirmation question is about (decision 37).
    pub pending: Option<PendingReport>,
}
pub struct PendingReport { pub report: Uuid, pub cards: Vec<RateCard> }
pub struct Proposal { pub report: Uuid, pub provider: ProviderId, pub name: String, pub cards: Vec<RateCard> }
```

- [ ] **Step 1: Write the failing tests**

`crates/server/tests/catalogue.rs`:

```rust
mod common;

use axum::http::StatusCode;
use common::{TODAY, TestApp};
use serde_json::json;

#[tokio::test]
async fn the_catalogue_combines_seed_providers_and_database_offers() {
    let Some(app) = TestApp::start().await else { return };
    let (status, body) = app.get("/v1/catalogue").await;
    assert_eq!(status, StatusCode::OK);
    let providers = body["catalogue"]["providers"].as_array().unwrap();
    assert_eq!(providers.len(), 2);
    assert_eq!(providers[0]["id"], "miles");
    assert_eq!(providers[0]["android_package"], "com.driveby.app");
    let miles_offers: Vec<&str> = providers[0]["offers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|offer| offer["id"].as_str().unwrap())
        .collect();
    assert_eq!(miles_offers, ["miles.l", "miles.s"]);
    assert_eq!(
        body["offers"][0],
        json!({"offer": "miles.l", "confirmed_on": TODAY, "locked": false, "pending": null})
    );
    assert_eq!(body["proposals"], json!([]));
}

#[tokio::test]
async fn the_catalogue_serves_crowd_prices_not_seed_prices() {
    let Some(app) = TestApp::start().await else { return };
    sqlx::query(
        "UPDATE offers SET cards = jsonb_set(cards, '{0,per_km}', '99') WHERE id = 'miles.s'",
    )
    .execute(&app.pool)
    .await
    .unwrap();
    let (_, body) = app.get("/v1/catalogue").await;
    assert_eq!(body["catalogue"]["providers"][0]["offers"][1]["cards"][0]["per_km"], 99);
}

#[tokio::test]
async fn the_catalogue_parses_with_the_core() {
    let Some(app) = TestApp::start().await else { return };
    let (_, body) = app.get("/v1/catalogue").await;
    let catalogue: carsharing_core::Catalogue =
        serde_json::from_value(body["catalogue"].clone()).unwrap();
    catalogue.validate().unwrap();
}

#[tokio::test]
async fn offers_of_a_provider_no_longer_in_the_seed_are_hidden() {
    let Some(app) = TestApp::start().await else { return };
    sqlx::query(
        "INSERT INTO offers (id, provider_id, name, cards) SELECT 'gone.x', 'gone', 'X', cards
         FROM offers WHERE id = 'miles.s'",
    )
    .execute(&app.pool)
    .await
    .unwrap();
    let (_, body) = app.get("/v1/catalogue").await;
    assert!(!body.to_string().contains("gone.x"));
}
```

Tests for `pending` and `proposals` are added in Tasks 8 and 9, where reports can be created.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p whichride-server --test catalogue`
Expected: FAIL — `404` instead of `200`.

- [ ] **Step 3: Implement**

`crates/server/src/catalogue.rs` (add `pub mod catalogue;` to `lib.rs`):

```rust
//! `GET /v1/catalogue`: what the app and the website compare with.

use std::collections::HashMap;

use axum::Json;
use axum::extract::State;
use carsharing_core::{Catalogue, Day, Offer, OfferId, Provider, ProviderId, RateCard};
use serde::Serialize;
use sqlx::types::Json as SqlJson;
use uuid::Uuid;

use crate::db;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct CatalogueResponse {
    pub catalogue: Catalogue,
    pub offers: Vec<OfferStatus>,
    pub proposals: Vec<Proposal>,
}

#[derive(Debug, Serialize)]
pub struct OfferStatus {
    pub offer: OfferId,
    pub confirmed_on: Option<Day>,
    pub locked: bool,
    pub pending: Option<PendingReport>,
}

#[derive(Debug, Serialize)]
pub struct PendingReport {
    pub report: Uuid,
    pub cards: Vec<RateCard>,
}

#[derive(Debug, Serialize)]
pub struct Proposal {
    pub report: Uuid,
    pub provider: ProviderId,
    pub name: String,
    pub cards: Vec<RateCard>,
}

pub async fn get_catalogue(
    State(state): State<AppState>,
) -> Result<Json<CatalogueResponse>, ApiError> {
    let offers = db::list_offers(&state.db).await?;
    let mut pending = oldest_pending_reports(&state).await?;
    let proposals = pending_proposals(&state).await?;

    let mut providers = Vec::new();
    let mut statuses = Vec::new();
    for seed in &state.providers.providers {
        let mut provider = Provider {
            offers: Vec::new(),
            ..seed.clone()
        };
        for row in offers.iter().filter(|row| row.provider_id == seed.id.0) {
            provider.offers.push(Offer {
                id: OfferId(row.id.clone()),
                name: row.name.clone(),
                cards: row.cards.0.clone(),
            });
            statuses.push(OfferStatus {
                offer: OfferId(row.id.clone()),
                confirmed_on: row.confirmed_on.map(Day),
                locked: row.locked,
                pending: pending.remove(&row.id),
            });
        }
        if !provider.offers.is_empty() {
            providers.push(provider);
        }
    }
    Ok(Json(CatalogueResponse {
        catalogue: Catalogue { providers },
        offers: statuses,
        proposals,
    }))
}

/// The oldest pending report of each offer.
async fn oldest_pending_reports(
    state: &AppState,
) -> Result<HashMap<String, PendingReport>, ApiError> {
    let rows: Vec<(String, Uuid, SqlJson<Vec<RateCard>>)> = sqlx::query_as(
        "SELECT DISTINCT ON (offer_id) offer_id, id, cards FROM reports
         WHERE status = 'pending' AND offer_id IS NOT NULL
         ORDER BY offer_id, created_on, id",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(offer, report, cards)| {
            (offer, PendingReport { report, cards: cards.0 })
        })
        .collect())
}

async fn pending_proposals(state: &AppState) -> Result<Vec<Proposal>, ApiError> {
    let rows: Vec<(Uuid, String, String, SqlJson<Vec<RateCard>>)> = sqlx::query_as(
        "SELECT id, proposed_provider, proposed_name, cards FROM reports
         WHERE status = 'pending' AND offer_id IS NULL
         ORDER BY created_on, id",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(rows
        .into_iter()
        .filter(|(_, provider, _, _)| {
            state.providers.provider(&ProviderId(provider.clone())).is_some()
        })
        .map(|(report, provider, name, cards)| Proposal {
            report,
            provider: ProviderId(provider),
            name,
            cards: cards.0,
        })
        .collect())
}
```

A provider with no offers in the database is left out, because `Catalogue::validate` rejects a
provider without offers.

In `app.rs`, add the route above the fallback:

```rust
        .route("/v1/catalogue", get(crate::catalogue::get_catalogue))
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p whichride-server && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/server
git commit -m "feat(server): GET /v1/catalogue with crowd prices, freshness and pending changes

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 6: Play Integrity verification

Spec §7.4: "Every report or vote carries a Play Integrity token bound to the request (request
hash); the server verifies app integrity, device integrity and the request hash with Google."
The app uses **standard requests** (Google's recommended mode): it sets `requestHash` when it
asks for a token, and the server decodes the token through Google's
`decodeIntegrityToken` endpoint and compares.

Google reference (read it before implementing; check the field names below against it):
<https://developer.android.com/google/play/integrity/standard> and
<https://developer.android.com/google/play/integrity/verdicts>.

**Files:**
- Create: `crates/server/src/http.rs`
- Create: `crates/server/src/integrity.rs`
- Modify: `crates/server/src/lib.rs`, `state.rs`, `commands.rs`, `error.rs`
- Modify: `crates/server/tests/common/mod.rs`

**Interfaces:**
- Consumes: `IntegrityMode` (Task 3), `ApiError` (Task 3).
- Produces:
  - `http::client(timeout: Duration) -> reqwest::Client` (installs the ring crypto provider
    once) and `http::client_without_redirects(timeout: Duration) -> reqwest::Client`.
  - `integrity::TOKEN_HEADER: &str = "x-integrity-token"`.
  - `integrity::request_hash(method: &str, path: &str, body: &[u8]) -> String` — lowercase hex
    SHA-256 of `"{method} {path}\n"` followed by the body bytes. **The Android app must compute
    exactly this** (milestone 5).
  - `integrity::Verdict` (Deserialize), `integrity::check(&Verdict, package: &str, expected_hash: &str, now_millis: i64) -> Result<(), IntegrityError>`.
  - `integrity::IntegrityError::{Missing, Refused(&'static str), Unavailable}`;
    `impl From<IntegrityError> for ApiError`.
  - `integrity::Integrity::{Google(GoogleIntegrity), Off}` with
    `async fn verify(&self, token: Option<&str>, expected_hash: &str) -> Result<(), IntegrityError>`.
  - `GoogleIntegrity::new(package: String) -> GoogleIntegrity` and
    `GoogleIntegrity::with_endpoints(package: String, api_base: Url, metadata_base: Url) -> GoogleIntegrity` (tests).
  - `AppState` gains `integrity: Arc<Integrity>`; `TestApp::start()` uses `Integrity::Off`;
    `TestApp::start_with_integrity(Integrity) -> Option<TestApp>`.

- [ ] **Step 1: Write the failing tests**

Tests at the bottom of `crates/server/src/integrity.rs`:

```rust
#[cfg(test)]
mod tests {
    use wiremock::matchers::{body_json, header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    const PACKAGE: &str = "app.whichride";
    const NOW: i64 = 1_792_411_200_000;

    fn verdict_json(hash: &str, timestamp: i64) -> serde_json::Value {
        serde_json::json!({
            "requestDetails": {
                "requestPackageName": PACKAGE,
                "requestHash": hash,
                "timestampMillis": timestamp.to_string()
            },
            "appIntegrity": {
                "appRecognitionVerdict": "PLAY_RECOGNIZED",
                "packageName": PACKAGE,
                "certificateSha256Digest": ["abc"],
                "versionCode": "1"
            },
            "deviceIntegrity": {
                "deviceRecognitionVerdict": ["MEETS_DEVICE_INTEGRITY"]
            },
            "accountDetails": { "appLicensingVerdict": "LICENSED" }
        })
    }

    fn verdict(hash: &str, timestamp: i64) -> Verdict {
        serde_json::from_value(verdict_json(hash, timestamp)).unwrap()
    }

    #[test]
    fn the_request_hash_covers_method_path_and_body() {
        let hash = request_hash("POST", "/v1/reports", br#"{"a":1}"#);
        assert_eq!(hash.len(), 64);
        assert_eq!(hash, request_hash("POST", "/v1/reports", br#"{"a":1}"#));
        assert_ne!(hash, request_hash("POST", "/v1/reports", br#"{"a":2}"#));
        assert_ne!(hash, request_hash("POST", "/v1/installs", br#"{"a":1}"#));
        // Known value, so the Android implementation can be checked against it.
        assert_eq!(
            request_hash("POST", "/v1/installs", b"{}"),
            "9bc58881cd740b3aa0d48e5698ed16bb88cd9949784c69f48fd4f918787a1099"
        );
    }

    #[test]
    fn a_good_verdict_passes() {
        assert_eq!(check(&verdict("h", NOW), PACKAGE, "h", NOW), Ok(()));
        assert_eq!(check(&verdict("h", NOW - 9 * 60_000), PACKAGE, "h", NOW), Ok(()));
    }

    #[test]
    fn a_different_request_is_refused() {
        assert_eq!(
            check(&verdict("other", NOW), PACKAGE, "h", NOW),
            Err(IntegrityError::Refused("request hash"))
        );
    }

    #[test]
    fn another_package_is_refused() {
        assert_eq!(
            check(&verdict("h", NOW), "com.example.fake", "h", NOW),
            Err(IntegrityError::Refused("package"))
        );
    }

    #[test]
    fn an_app_not_from_play_is_refused() {
        let mut json = verdict_json("h", NOW);
        json["appIntegrity"]["appRecognitionVerdict"] = "UNRECOGNIZED_VERSION".into();
        let verdict: Verdict = serde_json::from_value(json).unwrap();
        assert_eq!(
            check(&verdict, PACKAGE, "h", NOW),
            Err(IntegrityError::Refused("app"))
        );
    }

    #[test]
    fn a_device_without_integrity_is_refused() {
        let mut json = verdict_json("h", NOW);
        json["deviceIntegrity"] = serde_json::json!({});
        let verdict: Verdict = serde_json::from_value(json).unwrap();
        assert_eq!(
            check(&verdict, PACKAGE, "h", NOW),
            Err(IntegrityError::Refused("device"))
        );
    }

    #[test]
    fn old_or_future_verdicts_are_refused() {
        assert_eq!(
            check(&verdict("h", NOW - 11 * 60_000), PACKAGE, "h", NOW),
            Err(IntegrityError::Refused("stale"))
        );
        assert_eq!(
            check(&verdict("h", NOW + 2 * 60_000), PACKAGE, "h", NOW),
            Err(IntegrityError::Refused("stale"))
        );
    }

    #[test]
    fn a_verdict_without_request_details_does_not_parse() {
        assert!(serde_json::from_value::<Verdict>(serde_json::json!({})).is_err());
    }

    async fn google(server: &MockServer) -> GoogleIntegrity {
        let base = Url::parse(&format!("{}/", server.uri())).unwrap();
        GoogleIntegrity::with_endpoints(PACKAGE.into(), base.clone(), base)
    }

    async fn mock_metadata(server: &MockServer, times: u64) {
        Mock::given(method("GET"))
            .and(path("/computeMetadata/v1/instance/service-accounts/default/token"))
            .and(query_param("scopes", "https://www.googleapis.com/auth/playintegrity"))
            .and(header("metadata-flavor", "Google"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "ya29.test", "expires_in": 3599, "token_type": "Bearer"
            })))
            .expect(times)
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn google_decodes_the_token_with_the_service_account() {
        let server = MockServer::start().await;
        mock_metadata(&server, 1).await;
        let now = now_millis();
        Mock::given(method("POST"))
            .and(path(format!("/v1/{PACKAGE}:decodeIntegrityToken")))
            .and(header("authorization", "Bearer ya29.test"))
            .and(body_json(serde_json::json!({"integrityToken": "tok"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"tokenPayloadExternal": verdict_json("h", now)}),
            ))
            .expect(2)
            .mount(&server)
            .await;
        let integrity = Integrity::Google(google(&server).await);
        assert_eq!(integrity.verify(Some("tok"), "h").await, Ok(()));
        // The access token is cached: the metadata server is asked once.
        assert_eq!(integrity.verify(Some("tok"), "h").await, Ok(()));
    }

    #[tokio::test]
    async fn a_missing_token_is_refused_without_asking_google() {
        let server = MockServer::start().await;
        mock_metadata(&server, 0).await;
        let integrity = Integrity::Google(google(&server).await);
        assert_eq!(integrity.verify(None, "h").await, Err(IntegrityError::Missing));
        assert_eq!(integrity.verify(Some(""), "h").await, Err(IntegrityError::Missing));
        let huge = "x".repeat(MAX_TOKEN_BYTES + 1);
        assert_eq!(integrity.verify(Some(&huge), "h").await, Err(IntegrityError::Missing));
    }

    #[tokio::test]
    async fn an_invalid_token_is_refused() {
        let server = MockServer::start().await;
        mock_metadata(&server, 1).await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(400))
            .mount(&server)
            .await;
        let integrity = Integrity::Google(google(&server).await);
        assert_eq!(
            integrity.verify(Some("bad"), "h").await,
            Err(IntegrityError::Refused("token"))
        );
    }

    #[tokio::test]
    async fn google_being_down_is_not_the_apps_fault() {
        let server = MockServer::start().await;
        mock_metadata(&server, 1).await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let integrity = Integrity::Google(google(&server).await);
        assert_eq!(
            integrity.verify(Some("tok"), "h").await,
            Err(IntegrityError::Unavailable)
        );
    }

    #[tokio::test]
    async fn off_accepts_everything() {
        assert_eq!(Integrity::Off.verify(None, "h").await, Ok(()));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p whichride-server --lib integrity`
Expected: compile errors (module not found).

- [ ] **Step 3: Implement**

`crates/server/src/http.rs` (add `pub mod http;` to `lib.rs`):

```rust
//! Outgoing HTTP clients: rustls with the ring provider (as sqlx), short timeouts.

use std::time::Duration;

use reqwest::redirect::Policy;

fn install_crypto_provider() {
    // Fails only if a provider is already installed, which is fine.
    let _ = rustls::crypto::ring::default_provider().install_default();
}

pub fn client(timeout: Duration) -> reqwest::Client {
    install_crypto_provider();
    reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .expect("a client with only a timeout always builds")
}

/// For short links: each redirect is inspected before it is followed (Task 11).
pub fn client_without_redirects(timeout: Duration) -> reqwest::Client {
    install_crypto_provider();
    reqwest::Client::builder()
        .timeout(timeout)
        .redirect(Policy::none())
        .build()
        .expect("a client with a timeout and no redirects always builds")
}
```

`crates/server/src/integrity.rs` (add `pub mod integrity;` to `lib.rs`), above the tests:

```rust
//! Play Integrity (spec §7.4): a write is accepted only from a genuine app on a genuine device,
//! and only for the exact request the app asked Google to vouch for.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::sync::Mutex;
use url::Url;

use crate::error::ApiError;

pub const TOKEN_HEADER: &str = "x-integrity-token";

/// Real tokens are a few kilobytes; anything longer is not sent to Google.
pub const MAX_TOKEN_BYTES: usize = 8 * 1024;

/// A verdict older than this is refused; the app asks for a token right before each write.
const MAX_AGE_MILLIS: i64 = 10 * 60 * 1000;
/// Tolerated difference between Google's clock and ours.
const MAX_FUTURE_MILLIS: i64 = 60 * 1000;

const SCOPE: &str = "https://www.googleapis.com/auth/playintegrity";

/// Lowercase hex SHA-256 of `"{method} {path}\n"` followed by the body. The app sets it as the
/// token's `requestHash`, so a token is good for this one request only.
pub fn request_hash(method: &str, path: &str, body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(method.as_bytes());
    hasher.update(b" ");
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
    hasher.update(body);
    hex::encode(hasher.finalize())
}

/// The parts of Google's decoded token this server checks.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verdict {
    pub request_details: RequestDetails,
    pub app_integrity: AppIntegrity,
    pub device_integrity: DeviceIntegrity,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestDetails {
    pub request_package_name: String,
    #[serde(default)]
    pub request_hash: Option<String>,
    /// Google sends 64-bit integers as JSON strings.
    pub timestamp_millis: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppIntegrity {
    pub app_recognition_verdict: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceIntegrity {
    #[serde(default)]
    pub device_recognition_verdict: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IntegrityError {
    #[error("no integrity token")]
    Missing,
    #[error("integrity verdict refused: {0}")]
    Refused(&'static str),
    #[error("the integrity service is unavailable")]
    Unavailable,
}

impl From<IntegrityError> for ApiError {
    fn from(error: IntegrityError) -> ApiError {
        match error {
            IntegrityError::Missing | IntegrityError::Refused(_) => {
                tracing::info!(%error, "integrity check failed");
                ApiError::IntegrityFailed
            }
            IntegrityError::Unavailable => {
                tracing::warn!(%error, "integrity check unavailable");
                ApiError::IntegrityUnavailable
            }
        }
    }
}

/// Checks a decoded verdict: our package, recognised by Play, a device with integrity, this
/// request, and recent.
pub fn check(
    verdict: &Verdict,
    package: &str,
    expected_hash: &str,
    now_millis: i64,
) -> Result<(), IntegrityError> {
    let details = &verdict.request_details;
    if details.request_package_name != package {
        return Err(IntegrityError::Refused("package"));
    }
    if verdict.app_integrity.app_recognition_verdict != "PLAY_RECOGNIZED" {
        return Err(IntegrityError::Refused("app"));
    }
    let device = &verdict.device_integrity.device_recognition_verdict;
    if !device.iter().any(|label| label == "MEETS_DEVICE_INTEGRITY") {
        return Err(IntegrityError::Refused("device"));
    }
    if details.request_hash.as_deref() != Some(expected_hash) {
        return Err(IntegrityError::Refused("request hash"));
    }
    let issued: i64 = details
        .timestamp_millis
        .parse()
        .map_err(|_| IntegrityError::Refused("stale"))?;
    let age = now_millis.saturating_sub(issued);
    if age > MAX_AGE_MILLIS || age < -MAX_FUTURE_MILLIS {
        return Err(IntegrityError::Refused("stale"));
    }
    Ok(())
}

pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX))
}

pub enum Integrity {
    Google(GoogleIntegrity),
    /// Local development only; the configuration refuses it on Cloud Run.
    Off,
}

impl Integrity {
    pub async fn verify(&self, token: Option<&str>, expected_hash: &str) -> Result<(), IntegrityError> {
        let Integrity::Google(google) = self else {
            return Ok(());
        };
        let token = token
            .filter(|token| !token.is_empty() && token.len() <= MAX_TOKEN_BYTES)
            .ok_or(IntegrityError::Missing)?;
        let verdict = google.decode(token).await?;
        check(&verdict, &google.package, expected_hash, now_millis())
    }
}

/// Decodes tokens with Google, authenticated as the Cloud Run service account (whose access
/// token comes from the metadata server; no key file exists).
pub struct GoogleIntegrity {
    http: reqwest::Client,
    package: String,
    api_base: Url,
    metadata_base: Url,
    access: Mutex<Option<(String, Instant)>>,
}

#[derive(Deserialize)]
struct AccessToken {
    access_token: String,
    expires_in: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecodeResponse {
    token_payload_external: Verdict,
}

impl GoogleIntegrity {
    pub fn new(package: String) -> GoogleIntegrity {
        GoogleIntegrity::with_endpoints(
            package,
            Url::parse("https://playintegrity.googleapis.com/").expect("valid URL"),
            Url::parse("http://metadata.google.internal/").expect("valid URL"),
        )
    }

    /// With other base URLs, for tests.
    pub fn with_endpoints(package: String, api_base: Url, metadata_base: Url) -> GoogleIntegrity {
        GoogleIntegrity {
            http: crate::http::client(Duration::from_secs(5)),
            package,
            api_base,
            metadata_base,
            access: Mutex::new(None),
        }
    }

    async fn access_token(&self) -> Result<String, IntegrityError> {
        let mut cached = self.access.lock().await;
        if let Some((token, expires)) = cached.as_ref()
            && *expires > Instant::now() + Duration::from_secs(60)
        {
            return Ok(token.clone());
        }
        let url = self
            .metadata_base
            .join("computeMetadata/v1/instance/service-accounts/default/token")
            .map_err(|_| IntegrityError::Unavailable)?;
        let fresh: AccessToken = self
            .http
            .get(url)
            .query(&[("scopes", SCOPE)])
            .header("Metadata-Flavor", "Google")
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|_| IntegrityError::Unavailable)?
            .json()
            .await
            .map_err(|_| IntegrityError::Unavailable)?;
        let expires = Instant::now() + Duration::from_secs(fresh.expires_in);
        *cached = Some((fresh.access_token.clone(), expires));
        Ok(fresh.access_token)
    }

    async fn decode(&self, token: &str) -> Result<Verdict, IntegrityError> {
        let access = self.access_token().await?;
        let url = self
            .api_base
            .join(&format!("v1/{}:decodeIntegrityToken", self.package))
            .map_err(|_| IntegrityError::Unavailable)?;
        let response = self
            .http
            .post(url)
            .bearer_auth(access)
            .json(&serde_json::json!({ "integrityToken": token }))
            .send()
            .await
            .map_err(|_| IntegrityError::Unavailable)?;
        if response.status().is_client_error() {
            return Err(IntegrityError::Refused("token"));
        }
        let decoded: DecodeResponse = response
            .error_for_status()
            .map_err(|_| IntegrityError::Unavailable)?
            .json()
            .await
            .map_err(|_| IntegrityError::Refused("token"))?;
        Ok(decoded.token_payload_external)
    }
}
```

The `if let … && …` chain needs edition 2024 (the workspace uses it). Errors from `reqwest` are
dropped on purpose: their messages contain URLs, and Google's error bodies can echo the token.

`state.rs`: add `pub integrity: Arc<Integrity>,` (with `use crate::integrity::Integrity;`).

`commands.rs` `serve`: build it from the configuration:

```rust
    let integrity = match &config.integrity {
        IntegrityMode::Google { package } => Integrity::Google(GoogleIntegrity::new(package.clone())),
        IntegrityMode::Off => {
            warn!("PLAY_INTEGRITY=off: writes are not checked (local development only)");
            Integrity::Off
        }
    };
```

and pass `integrity: Arc::new(integrity)` into `AppState`.

`tests/common/mod.rs`: `TestApp::start()` becomes `TestApp::start_with_integrity(Integrity::Off)`,
and `start_with_integrity(integrity: Integrity)` holds the old body with
`integrity: Arc::new(integrity)` added to the state.

- [ ] **Step 4: Run the tests to verify they pass**

The known hash in `the_request_hash_covers_method_path_and_body` was computed with
`printf 'POST /v1/installs\n{}' | shasum -a 256`; if the test disagrees, the implementation is
wrong, not the constant.

Run: `cargo test -p whichride-server && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/server Cargo.lock
git commit -m "feat(server): verify Play Integrity verdicts bound to each request

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 7: Install registration and request authentication

Decision 32: an install registers once (with an integrity token) and gets an ID and a bearer
token; the server stores only the token's SHA-256.

**Files:**
- Create: `crates/server/src/auth.rs`
- Create: `crates/server/src/installs.rs`
- Modify: `crates/server/src/lib.rs`, `crates/server/src/app.rs`
- Create: `crates/server/tests/installs.rs`

**Interfaces:**
- Consumes: `Integrity`, `request_hash`, `TOKEN_HEADER` (Task 6); `AppState.db` (Task 4);
  `InstallRecord`, `Day` (core).
- Produces:
  - `auth::new_token() -> Result<String, ApiError>` (43-character base64url of 32 random bytes),
    `auth::hash_token(&str) -> [u8; 32]`.
  - `auth::Install { id: Uuid, record: InstallRecord }`;
    `auth::find_install(pool: &PgPool, token: &str) -> sqlx::Result<Option<Install>>`;
    `Install::install_id(&self) -> InstallId` (the UUID as a string, for the core).
  - Extractors (each implements `FromRequest<AppState>`; the request body is read once, at most
    `app::BODY_LIMIT_BYTES`):
    - `auth::Attested<T>(pub T)` — integrity token required; no install.
    - `auth::Signed<T> { install: Install, body: T }` — bearer token **and** integrity token.
    - `auth::Authed<T> { install: Install, body: T }` — bearer token only.
    Order of checks: bearer token (401) → JSON body (400) → integrity (403/503), so garbage never
    costs a call to Google.
  - `installs::register` handler for `POST /v1/installs` →
    `201 {"install_id": "<uuid>", "token": "<43 chars>"}`.
  - `TestApp::register() -> String` test helper returning a fresh install's token.

- [ ] **Step 1: Write the failing tests**

`crates/server/tests/installs.rs`:

```rust
mod common;

use axum::http::StatusCode;
use common::{TODAY, TestApp};
use serde_json::json;
use url::Url;
use whichride_server::auth;
use whichride_server::integrity::{GoogleIntegrity, Integrity, request_hash};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn registering_returns_an_id_and_a_token() {
    let Some(app) = TestApp::start().await else { return };
    let (status, body) = app.post("/v1/installs", &json!({})).await;
    assert_eq!(status, StatusCode::CREATED);
    let token = body["token"].as_str().unwrap();
    assert_eq!(token.len(), 43);
    let id = body["install_id"].as_str().unwrap();
    assert!(uuid::Uuid::parse_str(id).is_ok());

    let install = auth::find_install(&app.pool, token).await.unwrap().unwrap();
    assert_eq!(install.id.to_string(), id);
    assert_eq!(install.record.first_seen.0, TODAY);
    assert_eq!(install.record.decided_votes, 0);
}

#[tokio::test]
async fn only_the_tokens_hash_is_stored() {
    let Some(app) = TestApp::start().await else { return };
    let (_, body) = app.post("/v1/installs", &json!({})).await;
    let token = body["token"].as_str().unwrap();
    let stored: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM installs")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(stored, auth::hash_token(token));
    assert_ne!(stored, token.as_bytes());
}

#[tokio::test]
async fn two_registrations_get_different_tokens() {
    let Some(app) = TestApp::start().await else { return };
    let first = app.register().await;
    let second = app.register().await;
    assert_ne!(first, second);
}

#[tokio::test]
async fn an_unknown_token_finds_nothing() {
    let Some(app) = TestApp::start().await else { return };
    assert!(auth::find_install(&app.pool, "nope").await.unwrap().is_none());
}

#[tokio::test]
async fn a_body_that_is_not_json_is_a_bad_request() {
    let Some(app) = TestApp::start().await else { return };
    let request = axum::http::Request::post("/v1/installs")
        .header("content-type", "application/json")
        .body(axum::body::Body::from("not json"))
        .unwrap();
    let (status, body) = app.send(request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body, json!({"error": "bad_request"}));
}

async fn google_integrity(server: &MockServer, request_hash: &str) -> Integrity {
    Mock::given(method("GET"))
        .and(path("/computeMetadata/v1/instance/service-accounts/default/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"access_token": "ya29.test", "expires_in": 3599}),
        ))
        .mount(server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/app.whichride:decodeIntegrityToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tokenPayloadExternal": {
                "requestDetails": {
                    "requestPackageName": "app.whichride",
                    "requestHash": request_hash,
                    "timestampMillis": whichride_server::integrity::now_millis().to_string()
                },
                "appIntegrity": {"appRecognitionVerdict": "PLAY_RECOGNIZED"},
                "deviceIntegrity": {"deviceRecognitionVerdict": ["MEETS_DEVICE_INTEGRITY"]}
            }
        })))
        .mount(server)
        .await;
    let base = Url::parse(&format!("{}/", server.uri())).unwrap();
    Integrity::Google(GoogleIntegrity::with_endpoints(
        "app.whichride".into(),
        base.clone(),
        base,
    ))
}

#[tokio::test]
async fn with_google_integrity_registration_needs_a_token_for_this_request() {
    let server = MockServer::start().await;
    let hash = request_hash("POST", "/v1/installs", b"{}");
    let integrity = google_integrity(&server, &hash).await;
    let Some(app) = TestApp::start_with_integrity(integrity).await else { return };

    let (status, body) = app.post("/v1/installs", &json!({})).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body, json!({"error": "integrity_failed"}));

    let (status, _) = app
        .post_with("/v1/installs", &json!({}), &[("x-integrity-token", "tok")])
        .await;
    assert_eq!(status, StatusCode::CREATED);
}

#[tokio::test]
async fn a_token_bound_to_another_request_is_refused() {
    let server = MockServer::start().await;
    let integrity = google_integrity(&server, "hash-of-something-else").await;
    let Some(app) = TestApp::start_with_integrity(integrity).await else { return };
    let (status, _) = app
        .post_with("/v1/installs", &json!({}), &[("x-integrity-token", "tok")])
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
```

(Add `url` and `uuid` to `[dev-dependencies]` of the server crate with `.workspace = true` if
the compiler asks; they are already normal dependencies, which integration tests can use.)

Add to `TestApp` in `tests/common/mod.rs`:

```rust
    /// Registers a new install and returns its bearer token.
    pub async fn register(&self) -> String {
        let (status, body) = self.post("/v1/installs", &serde_json::json!({})).await;
        assert_eq!(status, StatusCode::CREATED, "{body}");
        body["token"].as_str().unwrap().to_owned()
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p whichride-server --test installs`
Expected: compile error (`whichride_server::auth` not found).

- [ ] **Step 3: Implement**

`crates/server/src/auth.rs` (add `pub mod auth;` to `lib.rs`):

```rust
//! Who is calling: install tokens (decision 32) and the extractors that check them together
//! with Play Integrity (decision 33).

use axum::body::{Bytes, to_bytes};
use axum::extract::{FromRequest, Request};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use carsharing_core::{Day, InstallId, InstallRecord};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::app::BODY_LIMIT_BYTES;
use crate::error::ApiError;
use crate::integrity::{TOKEN_HEADER, request_hash};
use crate::state::AppState;

const TOKEN_BYTES: usize = 32;

/// A new bearer token: 32 random bytes, base64url without padding (43 characters).
pub fn new_token() -> Result<String, ApiError> {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).map_err(|e| ApiError::internal(format!("random: {e}")))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub fn hash_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// A registered install and its voting history.
#[derive(Debug, Clone)]
pub struct Install {
    pub id: Uuid,
    pub record: InstallRecord,
}

impl Install {
    pub fn install_id(&self) -> InstallId {
        InstallId(self.id.to_string())
    }
}

pub async fn find_install(pool: &PgPool, token: &str) -> sqlx::Result<Option<Install>> {
    let row: Option<(Uuid, i32, i32, i32)> = sqlx::query_as(
        "SELECT id, first_seen, decided_votes, matching_votes FROM installs WHERE token_hash = $1",
    )
    .bind(hash_token(token).as_slice())
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(id, first_seen, decided, matching)| Install {
        id,
        record: InstallRecord {
            first_seen: Day(first_seen),
            decided_votes: u32::try_from(decided).unwrap_or(0),
            matching_votes: u32::try_from(matching).unwrap_or(0),
        },
    }))
}

/// A JSON body with a valid integrity token for this exact request.
pub struct Attested<T>(pub T);

/// A JSON body from a registered install, with a valid integrity token for this exact request.
pub struct Signed<T> {
    pub install: Install,
    pub body: T,
}

/// A JSON body from a registered install.
pub struct Authed<T> {
    pub install: Install,
    pub body: T,
}

impl<T: DeserializeOwned> FromRequest<AppState> for Attested<T> {
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &AppState) -> Result<Self, ApiError> {
        let (parts, bytes) = split(request).await?;
        let body = parse(&bytes)?;
        attest(&parts, &bytes, state).await?;
        Ok(Attested(body))
    }
}

impl<T: DeserializeOwned> FromRequest<AppState> for Signed<T> {
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &AppState) -> Result<Self, ApiError> {
        let (parts, bytes) = split(request).await?;
        let install = authenticate(&parts, state).await?;
        let body = parse(&bytes)?;
        attest(&parts, &bytes, state).await?;
        Ok(Signed { install, body })
    }
}

impl<T: DeserializeOwned> FromRequest<AppState> for Authed<T> {
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &AppState) -> Result<Self, ApiError> {
        let (parts, bytes) = split(request).await?;
        let install = authenticate(&parts, state).await?;
        let body = parse(&bytes)?;
        Ok(Authed { install, body })
    }
}

async fn split(request: Request) -> Result<(Parts, Bytes), ApiError> {
    let (parts, body) = request.into_parts();
    let bytes = to_bytes(body, BODY_LIMIT_BYTES)
        .await
        .map_err(|_| ApiError::BadRequest)?;
    Ok((parts, bytes))
}

fn parse<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ApiError> {
    serde_json::from_slice(bytes).map_err(|_| ApiError::BadRequest)
}

async fn authenticate(parts: &Parts, state: &AppState) -> Result<Install, ApiError> {
    let token = parts
        .headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|token| token.len() <= 64)
        .ok_or(ApiError::Unauthorized)?;
    find_install(&state.db, token)
        .await?
        .ok_or(ApiError::Unauthorized)
}

async fn attest(parts: &Parts, body: &[u8], state: &AppState) -> Result<(), ApiError> {
    let token = parts
        .headers
        .get(TOKEN_HEADER)
        .and_then(|value| value.to_str().ok());
    let hash = request_hash(parts.method.as_str(), parts.uri.path(), body);
    state.integrity.verify(token, &hash).await?;
    Ok(())
}
```

`crates/server/src/installs.rs` (add `pub mod installs;` to `lib.rs`):

```rust
//! `POST /v1/installs`: a genuine app gets its anonymous identity (spec §7.4).

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{Attested, hash_token, new_token};
use crate::error::ApiError;
use crate::state::AppState;

/// Registration carries nothing; the body exists so the integrity token can be bound to it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterRequest {}

#[derive(Debug, Serialize)]
pub struct Registered {
    pub install_id: Uuid,
    pub token: String,
}

pub async fn register(
    State(state): State<AppState>,
    Attested(_): Attested<RegisterRequest>,
) -> Result<(StatusCode, Json<Registered>), ApiError> {
    let id = Uuid::new_v4();
    let token = new_token()?;
    sqlx::query("INSERT INTO installs (id, token_hash, first_seen) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(hash_token(&token).as_slice())
        .bind((state.today)().0)
        .execute(&state.db)
        .await?;
    Ok((StatusCode::CREATED, Json(Registered { install_id: id, token })))
}
```

In `app.rs`, add `use axum::routing::post;` and the route:

```rust
        .route("/v1/installs", post(crate::installs::register))
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p whichride-server && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/server Cargo.lock
git commit -m "feat(server): install registration with integrity, bearer tokens stored as hashes

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 8: Price reports, votes and confirmations

Spec §7.1–7.3 with decisions 28–29 and 36–39. The acceptance and weighting rules are the
core's `is_accepted`, `weight` and `settle`; this task stores, locks and calls them.

Locking, so that concurrent requests cannot accept a report twice (Review Focus 3): every
transaction that can change an offer's reports first locks **the offer row**
(`SELECT … FOR UPDATE`), then the report rows. One lock order everywhere means no deadlocks.
(Proposals, which have no offer row yet, use a per-provider advisory lock instead; Task 9.)

**Files:**
- Create: `crates/server/src/crowd.rs`
- Modify: `crates/server/src/lib.rs`, `crates/server/src/app.rs`
- Modify: `crates/server/tests/common/mod.rs`
- Create: `crates/server/tests/crowd.rs`

**Interfaces:**
- Consumes: `Signed<T>`, `Install` (Task 7); `AppState` (Tasks 3–6); core
  `is_accepted, weight, settle, validate_cards, Vote, Answer, Weight, InstallId, RateCard, Cents, OfferId, ProviderId`.
- Produces:
  - `crowd::ReportRequest { offer: Option<OfferId>, provider: Option<ProviderId>, name: Option<String>, cards: Vec<RateCard> }`
    (`deny_unknown_fields`). Exactly one of `offer` or `provider` + `name`; proposals
    (`provider` + `name`) return `400` until Task 9.
  - `crowd::ReportStatus::{Pending, Accepted, Current}` (JSON `"pending"`, `"accepted"`, `"current"`).
  - `crowd::ReportOutcome { report: Option<Uuid>, status: ReportStatus }`.
  - `crowd::VoteRequest { answer: Answer }`, `crowd::VoteOutcome { status: ReportStatus }`.
  - Handlers `post_report` (`201` for a new report, `200` when it joined an identical pending
    report or equals the current price), `post_vote` (`200`), `post_confirmation` (`204`).
  - `crowd::check_cards(offer: &OfferId, cards: &[RateCard]) -> Result<(), ApiError>` and the
    limits `MAX_CARDS = 5`, `MAX_LABEL_CHARS = 40`.
  - `pub(crate) async fn accept(tx, report: &ReportRow, today: Day) -> Result<(), ApiError>` and
    `pub(crate) struct ReportRow` — reused by Task 9.
  - Test helpers `TestApp::post_as(token, path, body) -> (StatusCode, Value)` and
    `TestApp::make_trusted(token, decided, matching, age_days)`.

Plausibility limits (Review Focus 4), in euro cents: unlock fee or package price ≤ 500_00;
per km and per minute ≤ 5_00; minimum ≤ 50_00; included km and minutes ≤ 20_000. A value
beyond these is a typo or abuse; the app shows the same limits in its form (milestone 5).

- [ ] **Step 1: Write the failing tests**

Add to `TestApp` in `tests/common/mod.rs`:

```rust
    pub async fn post_as(&self, token: &str, path: &str, body: &Value) -> (StatusCode, Value) {
        let bearer = format!("Bearer {token}");
        self.post_with(path, body, &[("authorization", bearer.as_str())])
            .await
    }

    /// Gives an install a history, so its votes weigh more (or less) than a new install's.
    pub async fn make_trusted(&self, token: &str, decided: i32, matching: i32, age_days: i32) {
        sqlx::query(
            "UPDATE installs SET decided_votes = $2, matching_votes = $3, first_seen = $4
             WHERE token_hash = $1",
        )
        .bind(whichride_server::auth::hash_token(token).as_slice())
        .bind(decided)
        .bind(matching)
        .bind(TODAY - age_days)
        .execute(&self.pool)
        .await
        .unwrap();
    }
```

`crates/server/tests/crowd.rs`:

```rust
mod common;

use axum::http::StatusCode;
use common::{TODAY, TestApp};
use serde_json::{Value, json};

/// The fixture's `miles.s` card with another per-km price.
fn card(per_km: i64) -> Value {
    json!({
        "label": "Standard", "kind": "standard", "base_fee": 100, "per_km": per_km,
        "per_minute": 0, "minimum": 0,
        "km_rounding": "started_unit", "minute_rounding": "started_unit"
    })
}

fn report(per_km: i64) -> Value {
    json!({"offer": "miles.s", "cards": [card(per_km)]})
}

async fn per_km(app: &TestApp) -> i64 {
    let (_, body) = app.get("/v1/catalogue").await;
    body["catalogue"]["providers"][0]["offers"][1]["cards"][0]["per_km"]
        .as_i64()
        .unwrap()
}

async fn record(app: &TestApp, token: &str) -> (i32, i32) {
    sqlx::query_as("SELECT decided_votes, matching_votes FROM installs WHERE token_hash = $1")
        .bind(whichride_server::auth::hash_token(token).as_slice())
        .fetch_one(&app.pool)
        .await
        .unwrap()
}

async fn history_rows(app: &TestApp) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM price_history WHERE offer_id = 'miles.s'")
        .fetch_one(&app.pool)
        .await
        .unwrap()
}

async fn vote(app: &TestApp, token: &str, report: &str, answer: &str) -> (StatusCode, Value) {
    app.post_as(token, &format!("/v1/reports/{report}/votes"), &json!({"answer": answer}))
        .await
}

#[tokio::test]
async fn reports_need_a_registered_install() {
    let Some(app) = TestApp::start().await else { return };
    let (status, _) = app.post("/v1/reports", &report(89)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) = app.post_as("made-up", "/v1/reports", &report(89)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_report_alone_changes_nothing_for_others() {
    let Some(app) = TestApp::start().await else { return };
    let a = app.register().await;
    let (status, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["status"], "pending");
    assert_eq!(per_km(&app).await, 79);
    // The confirmation question shows the pending values (decision 37).
    let (_, catalogue) = app.get("/v1/catalogue").await;
    let status = &catalogue["offers"][1];
    assert_eq!(status["offer"], "miles.s");
    assert_eq!(status["pending"]["report"], body["report"]);
    assert_eq!(status["pending"]["cards"][0]["per_km"], 89);
}

#[tokio::test]
async fn one_confirmation_from_another_install_accepts_the_report() {
    let Some(app) = TestApp::start().await else { return };
    let (a, b) = (app.register().await, app.register().await);
    let (_, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let id = body["report"].as_str().unwrap();
    app.set_today(TODAY + 3);
    let (status, body) = vote(&app, &b, id, "yes").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"status": "accepted"}));
    assert_eq!(per_km(&app).await, 89);
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["confirmed_on"], TODAY + 3);
    assert_eq!(catalogue["offers"][1]["pending"], Value::Null);
    assert_eq!(history_rows(&app).await, 2);
    // Both were right.
    assert_eq!(record(&app, &a).await, (1, 1));
    assert_eq!(record(&app, &b).await, (1, 1));
}

#[tokio::test]
async fn weighted_no_votes_keep_the_report_pending() {
    let Some(app) = TestApp::start().await else { return };
    let (a, b, c) = (app.register().await, app.register().await, app.register().await);
    app.make_trusted(&b, 8, 8, 80).await; // weight 9000
    let (_, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let id = body["report"].as_str().unwrap();
    assert_eq!(vote(&app, &b, id, "no").await.1["status"], "pending");
    // 1000 (reporter) + 1000 (c) is not more than 9000.
    assert_eq!(vote(&app, &c, id, "yes").await.1["status"], "pending");
    assert_eq!(per_km(&app).await, 79);
}

#[tokio::test]
async fn the_reporter_cannot_confirm_their_own_report() {
    let Some(app) = TestApp::start().await else { return };
    let a = app.register().await;
    let (_, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let id = body["report"].as_str().unwrap();
    for _ in 0..3 {
        assert_eq!(vote(&app, &a, id, "yes").await.1["status"], "pending");
    }
    assert_eq!(per_km(&app).await, 79);
}

#[tokio::test]
async fn an_identical_report_counts_as_a_yes_vote() {
    let Some(app) = TestApp::start().await else { return };
    let (a, b) = (app.register().await, app.register().await);
    let (_, first) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let (status, second) = app.post_as(&b, "/v1/reports", &report(89)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(second["report"], first["report"]);
    assert_eq!(second["status"], "accepted");
    assert_eq!(per_km(&app).await, 89);
}

#[tokio::test]
async fn reporting_the_current_price_confirms_it() {
    let Some(app) = TestApp::start().await else { return };
    let a = app.register().await;
    app.set_today(TODAY + 10);
    let (status, body) = app.post_as(&a, "/v1/reports", &report(79)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({"report": null, "status": "current"}));
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["confirmed_on"], TODAY + 10);
}

#[tokio::test]
async fn a_new_report_replaces_the_same_installs_pending_one() {
    let Some(app) = TestApp::start().await else { return };
    let a = app.register().await;
    let (_, first) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let (_, second) = app.post_as(&a, "/v1/reports", &report(99)).await;
    assert_ne!(first["report"], second["report"]);
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["pending"]["report"], second["report"]);
    let b = app.register().await;
    let (status, body) = vote(&app, &b, first["report"].as_str().unwrap(), "yes").await;
    assert_eq!((status, body), (StatusCode::CONFLICT, json!({"error": "report_closed"})));
}

#[tokio::test]
async fn accepting_a_report_supersedes_the_others_and_settles_everyone() {
    let Some(app) = TestApp::start().await else { return };
    let (a, b, c, d) = (
        app.register().await,
        app.register().await,
        app.register().await,
        app.register().await,
    );
    let (_, x) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let (_, y) = app.post_as(&c, "/v1/reports", &report(99)).await;
    let (x, y) = (x["report"].as_str().unwrap(), y["report"].as_str().unwrap());
    vote(&app, &d, y, "no").await;
    assert_eq!(vote(&app, &b, x, "yes").await.1["status"], "accepted");
    assert_eq!(vote(&app, &d, y, "yes").await.0, StatusCode::CONFLICT);
    assert_eq!(record(&app, &a).await, (1, 1));
    assert_eq!(record(&app, &b).await, (1, 1));
    assert_eq!(record(&app, &c).await, (1, 0)); // its report lost
    assert_eq!(record(&app, &d).await, (1, 1)); // said no to the loser
}

#[tokio::test]
async fn a_locked_offer_takes_no_reports_or_votes() {
    let Some(app) = TestApp::start().await else { return };
    let (a, b) = (app.register().await, app.register().await);
    let (_, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let id = body["report"].as_str().unwrap();
    sqlx::query("UPDATE offers SET locked = true WHERE id = 'miles.s'")
        .execute(&app.pool)
        .await
        .unwrap();
    let locked = (StatusCode::CONFLICT, json!({"error": "offer_locked"}));
    assert_eq!(app.post_as(&b, "/v1/reports", &report(99)).await, locked);
    assert_eq!(vote(&app, &b, id, "yes").await, locked);
    assert_eq!(per_km(&app).await, 79);
}

#[tokio::test]
async fn implausible_reports_are_rejected() {
    let Some(app) = TestApp::start().await else { return };
    let a = app.register().await;
    let long_label = {
        let mut card = card(89);
        card["label"] = "x".repeat(41).into();
        card
    };
    let empty_label = {
        let mut card = card(89);
        card["label"] = "".into();
        card
    };
    let package = {
        let mut card = card(89);
        card["kind"] = "package".into();
        card["base_fee"] = 50_001.into();
        card
    };
    for body in [
        report(100_000),
        report(-1),
        json!({"offer": "miles.s", "cards": []}),
        json!({"offer": "miles.s", "cards": vec![card(89); 6]}),
        json!({"offer": "miles.s", "cards": [long_label]}),
        json!({"offer": "miles.s", "cards": [empty_label]}),
        json!({"offer": "miles.s", "cards": [package]}),
        json!({"offer": "miles.s", "cards": [card(89)], "extra": 1}),
        json!({"offer": "miles.s", "provider": "miles", "name": "X", "cards": [card(89)]}),
        json!({"cards": [card(89)]}),
    ] {
        let (status, response) = app.post_as(&a, "/v1/reports", &body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body} → {response}");
    }
    let (status, _) = app
        .post_as(&a, "/v1/reports", &json!({"offer": "nope.x", "cards": [card(89)]}))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let reports: i64 = sqlx::query_scalar("SELECT count(*) FROM reports")
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(reports, 0);
}

#[tokio::test]
async fn votes_on_unknown_reports_are_not_found() {
    let Some(app) = TestApp::start().await else { return };
    let a = app.register().await;
    let unknown = uuid::Uuid::new_v4().to_string();
    assert_eq!(vote(&app, &a, &unknown, "yes").await.0, StatusCode::NOT_FOUND);
    assert_eq!(vote(&app, &a, "not-a-uuid", "yes").await.0, StatusCode::NOT_FOUND);
    let (_, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let id = body["report"].as_str().unwrap();
    assert_eq!(vote(&app, &a, id, "maybe").await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_confirmations_accept_once() {
    let Some(app) = TestApp::start().await else { return };
    let a = app.register().await;
    let mut voters = Vec::new();
    for _ in 0..4 {
        voters.push(app.register().await);
    }
    let (_, body) = app.post_as(&a, "/v1/reports", &report(89)).await;
    let id = body["report"].as_str().unwrap().to_owned();
    let results = futures_join_all(&app, &voters, &id).await;
    let accepted = results.iter().filter(|s| **s == StatusCode::OK).count();
    let closed = results.iter().filter(|s| **s == StatusCode::CONFLICT).count();
    assert_eq!((accepted, closed), (1, 3), "{results:?}");
    assert_eq!(history_rows(&app).await, 2);
    assert_eq!(record(&app, &a).await, (1, 1));
}

/// Sends every voter's yes at the same time.
async fn futures_join_all(app: &TestApp, voters: &[String], id: &str) -> Vec<StatusCode> {
    let mut handles = Vec::new();
    for token in voters {
        let router = app.router.clone();
        let request = axum::http::Request::post(format!("/v1/reports/{id}/votes"))
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(axum::body::Body::from(r#"{"answer":"yes"}"#))
            .unwrap();
        handles.push(tokio::spawn(async move {
            use tower::ServiceExt;
            router.oneshot(request).await.unwrap().status()
        }));
    }
    let mut statuses = Vec::new();
    for handle in handles {
        statuses.push(handle.await.unwrap());
    }
    statuses
}

#[tokio::test]
async fn confirming_the_current_price_refreshes_its_date() {
    let Some(app) = TestApp::start().await else { return };
    let a = app.register().await;
    app.set_today(TODAY + 20);
    let path = "/v1/offers/miles.s/confirmations";
    let (status, _) = app.post_as(&a, path, &json!({"answer": "no"})).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["confirmed_on"], TODAY); // decision 36
    let (status, _) = app.post_as(&a, path, &json!({"answer": "yes"})).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["confirmed_on"], TODAY + 20);
}

#[tokio::test]
async fn a_confirmation_without_weight_changes_nothing() {
    let Some(app) = TestApp::start().await else { return };
    let a = app.register().await;
    app.make_trusted(&a, 1_000, 0, 0).await; // weight 0
    app.set_today(TODAY + 20);
    app.post_as(&a, "/v1/offers/miles.s/confirmations", &json!({"answer": "yes"}))
        .await;
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["confirmed_on"], TODAY);
}

#[tokio::test]
async fn confirming_an_unknown_offer_is_not_found() {
    let Some(app) = TestApp::start().await else { return };
    let a = app.register().await;
    let (status, _) = app
        .post_as(&a, "/v1/offers/nope.x/confirmations", &json!({"answer": "yes"}))
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
```

(`a_confirmation_without_weight_changes_nothing`: with 1000 decided and 0 matching votes the
smoothed accuracy is `1 × 100 / 1002 = 0`, so the weight is 0.)

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p whichride-server --test crowd`
Expected: FAIL — the report routes answer `404`.

- [ ] **Step 3: Implement**

`crates/server/src/crowd.rs` (add `pub mod crowd;` to `lib.rs`):

```rust
//! Crowd prices (spec §7): reports, votes and confirmations. The rules are the core's
//! (`weight`, `is_accepted`, `settle`); this module stores them and serialises concurrent
//! requests by locking the offer row first (see the plan's Task 8).

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use carsharing_core::{
    Answer, Cents, Day, InstallId, OfferId, ProviderId, RateCard, Vote, Weight,
    is_accepted, settle, validate_cards, weight,
};
use serde::{Deserialize, Serialize};
use sqlx::types::Json as SqlJson;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::auth::{Install, Signed};
use crate::error::ApiError;
use crate::state::AppState;

pub const MAX_CARDS: usize = 5;
pub const MAX_LABEL_CHARS: usize = 40;
const MAX_BASE_FEE: Cents = Cents(500_00);
const MAX_PER_UNIT: Cents = Cents(5_00);
const MAX_MINIMUM: Cents = Cents(50_00);
const MAX_INCLUDED: u32 = 20_000;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportRequest {
    #[serde(default)]
    pub offer: Option<OfferId>,
    #[serde(default)]
    pub provider: Option<ProviderId>,
    #[serde(default)]
    pub name: Option<String>,
    pub cards: Vec<RateCard>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Pending,
    Accepted,
    /// The reported values are the current price; it was confirmed instead.
    Current,
}

#[derive(Debug, Serialize)]
pub struct ReportOutcome {
    pub report: Option<Uuid>,
    pub status: ReportStatus,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoteRequest {
    pub answer: Answer,
}

#[derive(Debug, Serialize)]
pub struct VoteOutcome {
    pub status: ReportStatus,
}

/// The rate cards of a report: valid for the core, labelled, and believable.
pub fn check_cards(offer: &OfferId, cards: &[RateCard]) -> Result<(), ApiError> {
    if cards.is_empty() || cards.len() > MAX_CARDS {
        return Err(ApiError::BadRequest);
    }
    validate_cards(offer, cards).map_err(|_| ApiError::BadRequest)?;
    for card in cards {
        let label = card.label.trim();
        let plausible = !label.is_empty()
            && label.chars().count() <= MAX_LABEL_CHARS
            && card.base_fee <= MAX_BASE_FEE
            && card.per_km <= MAX_PER_UNIT
            && card.per_minute <= MAX_PER_UNIT
            && card.minimum <= MAX_MINIMUM
            && card.included_km <= MAX_INCLUDED
            && card.included_minutes <= MAX_INCLUDED;
        if !plausible {
            return Err(ApiError::BadRequest);
        }
    }
    Ok(())
}

pub async fn post_report(
    State(state): State<AppState>,
    Signed { install, body }: Signed<ReportRequest>,
) -> Result<(StatusCode, Json<ReportOutcome>), ApiError> {
    let today = (state.today)();
    match (body.offer, body.provider, body.name) {
        (Some(offer), None, None) => {
            check_cards(&offer, &body.cards)?;
            report_on_offer(&state, &install, &offer, &body.cards, today).await
        }
        // Proposals of new offers: Task 9.
        _ => Err(ApiError::BadRequest),
    }
}

async fn report_on_offer(
    state: &AppState,
    install: &Install,
    offer: &OfferId,
    cards: &[RateCard],
    today: Day,
) -> Result<(StatusCode, Json<ReportOutcome>), ApiError> {
    let mut tx = state.db.begin().await?;
    let current = lock_offer(&mut tx, &offer.0).await?;
    let voter_weight = weight(&install.record, today);

    if current.cards.0.as_slice() == cards {
        if voter_weight > Weight(0) {
            confirm(&mut tx, &offer.0, today).await?;
        }
        tx.commit().await?;
        return Ok((StatusCode::OK, Json(ReportOutcome { report: None, status: ReportStatus::Current })));
    }

    let matching: Option<ReportRow> = sqlx::query_as(
        "SELECT id, offer_id, proposed_provider, proposed_name, reporter, reporter_weight,
             cards, status
         FROM reports WHERE offer_id = $1 AND status = 'pending' AND cards = $2
         ORDER BY created_on, id LIMIT 1 FOR UPDATE",
    )
    .bind(&offer.0)
    .bind(SqlJson(cards))
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(report) = matching {
        let status = vote_on(&mut tx, &report, install, Answer::Yes, voter_weight, today).await?;
        tx.commit().await?;
        return Ok((StatusCode::OK, Json(ReportOutcome { report: Some(report.id), status })));
    }

    // An install's new report on an offer replaces its own earlier pending one.
    sqlx::query(
        "UPDATE reports SET status = 'superseded', decided_on = $3
         WHERE offer_id = $1 AND reporter = $2 AND status = 'pending'",
    )
    .bind(&offer.0)
    .bind(install.id)
    .bind(today.0)
    .execute(&mut *tx)
    .await?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO reports (id, offer_id, reporter, reporter_weight, cards, created_on)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(&offer.0)
    .bind(install.id)
    .bind(weight_column(voter_weight))
    .bind(SqlJson(cards))
    .bind(today.0)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(ReportOutcome { report: Some(id), status: ReportStatus::Pending })))
}

pub async fn post_vote(
    State(state): State<AppState>,
    Path(report_id): Path<String>,
    Signed { install, body }: Signed<VoteRequest>,
) -> Result<Json<VoteOutcome>, ApiError> {
    let report_id = Uuid::parse_str(&report_id).map_err(|_| ApiError::NotFound)?;
    let today = (state.today)();
    let mut tx = state.db.begin().await?;
    // Lock order: the offer first, then the report (see the module docs).
    let target: Option<(Option<String>, Option<String>)> =
        sqlx::query_as("SELECT offer_id, proposed_provider FROM reports WHERE id = $1")
            .bind(report_id)
            .fetch_optional(&mut *tx)
            .await?;
    let (offer_id, proposed_provider) = target.ok_or(ApiError::NotFound)?;
    match (&offer_id, &proposed_provider) {
        (Some(offer), _) => {
            lock_offer(&mut tx, offer).await?;
        }
        (None, Some(provider)) => lock_proposals(&mut tx, provider).await?,
        (None, None) => return Err(ApiError::internal("report without a target")),
    }
    let report: ReportRow = sqlx::query_as(
        "SELECT id, offer_id, proposed_provider, proposed_name, reporter, reporter_weight,
             cards, status
         FROM reports WHERE id = $1 FOR UPDATE",
    )
        .bind(report_id)
        .fetch_one(&mut *tx)
        .await?;
    if report.status != "pending" {
        return Err(ApiError::ReportClosed);
    }
    let voter_weight = weight(&install.record, today);
    let status = vote_on(&mut tx, &report, &install, body.answer, voter_weight, today).await?;
    tx.commit().await?;
    Ok(Json(VoteOutcome { status }))
}

pub async fn post_confirmation(
    State(state): State<AppState>,
    Path(offer_id): Path<String>,
    Signed { install, body }: Signed<VoteRequest>,
) -> Result<StatusCode, ApiError> {
    let today = (state.today)();
    let mut tx = state.db.begin().await?;
    lock_offer(&mut tx, &offer_id).await?;
    // Decision 36: a "no" without a value changes nothing.
    if body.answer == Answer::Yes && weight(&install.record, today) > Weight(0) {
        confirm(&mut tx, &offer_id, today).await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct ReportRow {
    pub id: Uuid,
    pub offer_id: Option<String>,
    pub proposed_provider: Option<String>,
    pub proposed_name: Option<String>,
    pub reporter: Uuid,
    pub reporter_weight: i32,
    pub cards: SqlJson<Vec<RateCard>>,
    pub status: String,
}

#[derive(sqlx::FromRow)]
struct LockedOffer {
    cards: SqlJson<Vec<RateCard>>,
    locked: bool,
}

/// Locks an offer row for the rest of the transaction; `NotFound` or `OfferLocked` otherwise.
async fn lock_offer(tx: &mut Transaction<'_, Postgres>, id: &str) -> Result<LockedOffer, ApiError> {
    let offer: LockedOffer = sqlx::query_as("SELECT cards, locked FROM offers WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(ApiError::NotFound)?;
    if offer.locked {
        return Err(ApiError::OfferLocked);
    }
    Ok(offer)
}

/// Serialises everything that touches one provider's pending proposals (Task 9).
pub(crate) async fn lock_proposals(
    tx: &mut Transaction<'_, Postgres>,
    provider: &str,
) -> Result<(), ApiError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('proposals:' || $1))")
        .bind(provider)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn confirm(tx: &mut Transaction<'_, Postgres>, offer: &str, today: Day) -> Result<(), ApiError> {
    sqlx::query("UPDATE offers SET confirmed_on = $2 WHERE id = $1")
        .bind(offer)
        .bind(today.0)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Records a vote (the reporter's own votes are not stored: they never count) and accepts the
/// report if the core says so.
async fn vote_on(
    tx: &mut Transaction<'_, Postgres>,
    report: &ReportRow,
    voter: &Install,
    answer: Answer,
    voter_weight: Weight,
    today: Day,
) -> Result<ReportStatus, ApiError> {
    if voter.id != report.reporter {
        sqlx::query("INSERT INTO votes (report_id, install_id, answer, weight) VALUES ($1, $2, $3, $4)")
            .bind(report.id)
            .bind(voter.id)
            .bind(answer_column(answer))
            .bind(weight_column(voter_weight))
            .execute(&mut **tx)
            .await?;
    }
    let votes = votes_of(tx, report.id).await?;
    let reporter = InstallId(report.reporter.to_string());
    if is_accepted(&reporter, weight_from_column(report.reporter_weight), &votes) {
        accept(tx, report, today).await?;
        Ok(ReportStatus::Accepted)
    } else {
        Ok(ReportStatus::Pending)
    }
}

async fn votes_of(tx: &mut Transaction<'_, Postgres>, report: Uuid) -> Result<Vec<Vote>, ApiError> {
    let rows: Vec<(Uuid, String, i32)> = sqlx::query_as(
        "SELECT install_id, answer, weight FROM votes WHERE report_id = $1 ORDER BY seq",
    )
    .bind(report)
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(install, answer, weight)| Vote {
            install: InstallId(install.to_string()),
            answer: if answer == "yes" { Answer::Yes } else { Answer::No },
            weight: weight_from_column(weight),
        })
        .collect())
}

/// Makes a report the offer's price (Task 9 extends this to proposals), supersedes the other
/// pending reports of the same offer, and settles every install involved (decision 39).
pub(crate) async fn accept(
    tx: &mut Transaction<'_, Postgres>,
    report: &ReportRow,
    today: Day,
) -> Result<(), ApiError> {
    let Some(offer) = &report.offer_id else {
        return Err(ApiError::internal("accepting a proposal before Task 9"));
    };
    sqlx::query("UPDATE reports SET status = 'accepted', decided_on = $2 WHERE id = $1")
        .bind(report.id)
        .bind(today.0)
        .execute(&mut **tx)
        .await?;
    sqlx::query("UPDATE offers SET cards = $2, confirmed_on = $3 WHERE id = $1")
        .bind(offer)
        .bind(&report.cards)
        .bind(today.0)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO price_history (offer_id, cards, report_id, from_day) VALUES ($1, $2, $3, $4)")
        .bind(offer)
        .bind(&report.cards)
        .bind(report.id)
        .bind(today.0)
        .execute(&mut **tx)
        .await?;
    settle_report(tx, report.id, report.reporter, true).await?;

    let losers: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "UPDATE reports SET status = 'superseded', decided_on = $2
         WHERE offer_id = $1 AND status = 'pending' RETURNING id, reporter",
    )
    .bind(offer)
    .bind(today.0)
    .fetch_all(&mut **tx)
    .await?;
    for (loser, reporter) in losers {
        settle_report(tx, loser, reporter, false).await?;
    }
    Ok(())
}

/// Adds one decided vote to every install involved in a report, and a matching one to those
/// who were right (core `settle`).
pub(crate) async fn settle_report(
    tx: &mut Transaction<'_, Postgres>,
    report: Uuid,
    reporter: Uuid,
    accepted: bool,
) -> Result<(), ApiError> {
    let votes = votes_of(tx, report).await?;
    for (install, was_right) in settle(&InstallId(reporter.to_string()), &votes, accepted) {
        let install = Uuid::parse_str(&install.0).map_err(|e| ApiError::internal(e.to_string()))?;
        sqlx::query(
            "UPDATE installs SET decided_votes = decided_votes + 1,
                 matching_votes = matching_votes + $2
             WHERE id = $1",
        )
        .bind(install)
        .bind(i32::from(was_right))
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

fn answer_column(answer: Answer) -> &'static str {
    match answer {
        Answer::Yes => "yes",
        Answer::No => "no",
    }
}

fn weight_column(weight: Weight) -> i32 {
    i32::try_from(weight.0).unwrap_or(i32::MAX)
}

/// The schema's CHECK keeps stored weights within 0–10000.
fn weight_from_column(value: i32) -> Weight {
    Weight::try_from(u32::try_from(value).unwrap_or(0)).unwrap_or(Weight(0))
}
```

Notes for the implementer:
- `current.cards.0 == cards` and the `cards = $2` SQL comparison both need the reported cards in
  the same JSON shape as stored ones; both come from serialising `RateCard`, so they match.
  `jsonb` equality ignores key order.
- `accept` runs while the offer row is locked, so a second, concurrent vote waits, then finds
  the report no longer pending and gets `409 report_closed`.
- The `superseded` update in `report_on_offer` does not settle anyone: the reporter changed
  their own mind, nobody was proven wrong.

In `app.rs`, add the routes:

```rust
        .route("/v1/reports", post(crate::crowd::post_report))
        .route("/v1/reports/{id}/votes", post(crate::crowd::post_vote))
        .route("/v1/offers/{id}/confirmations", post(crate::crowd::post_confirmation))
```

Every query string is a literal: sqlx 0.9 accepts a `String` only wrapped in `AssertSqlSafe`,
and literals make that question disappear.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p whichride-server && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`
Expected: PASS. Run `concurrent_confirmations_accept_once` ten times
(`for i in $(seq 10); do cargo test -p whichride-server --test crowd concurrent -q || break; done`)
to make sure it is not passing by luck.

- [ ] **Step 5: Commit**

```bash
git add crates/server
git commit -m "feat(server): crowd price reports, weighted votes and confirmations

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 9: Proposals of new offers

Spec §5.1: "New offer of an existing provider (e.g. a new Bolt car model) — any user — proposed
in the app with its rate card; same confirmation rule as a price report."

**Files:**
- Modify: `crates/server/src/crowd.rs`
- Modify: `crates/server/src/catalogue.rs` (no code change expected; its `proposals` list now
  has data — the tests below cover it)
- Create: `crates/server/tests/proposals.rs`

**Interfaces:**
- Consumes: everything of Task 8; `AppState.providers` (Task 4).
- Produces:
  - `POST /v1/reports` with `{"provider": "<id>", "name": "<model>", "cards": [...]}`.
    Unknown provider → `404`; name empty or longer than `MAX_NAME_CHARS = 60` characters →
    `400`. A name that matches an existing offer of that provider (ignoring case and
    surrounding spaces) is treated as a report on that offer.
  - `crowd::offer_id_for(provider: &str, name: &str) -> String`, e.g.
    `("bolt", "Fiat 500e") → "bolt.fiat-500e"`.
  - On acceptance a new offer row, ID `offer_id_for(…)` or, if taken, with `-2`, `-3`, … appended.

- [ ] **Step 1: Write the failing tests**

Unit tests appended to `crates/server/src/crowd.rs` (create the `tests` module):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offer_ids_are_lowercase_with_dashes() {
        assert_eq!(offer_id_for("bolt", "Fiat 500e"), "bolt.fiat-500e");
        assert_eq!(offer_id_for("bolt", "  VW  ID.3 "), "bolt.vw-id-3");
        assert_eq!(offer_id_for("free2move", "Peugeot e-208"), "free2move.peugeot-e-208");
    }

    #[test]
    fn offer_ids_survive_odd_names() {
        assert_eq!(offer_id_for("bolt", "Škoda Enyaq"), "bolt.koda-enyaq");
        assert_eq!(offer_id_for("bolt", "!!!"), "bolt.offer");
        assert_eq!(offer_id_for("bolt", &"a".repeat(100)).len(), "bolt.".len() + 40);
    }
}
```

`crates/server/tests/proposals.rs`:

```rust
mod common;

use axum::http::StatusCode;
use common::{TODAY, TestApp};
use serde_json::{Value, json};

fn card() -> Value {
    json!({
        "label": "Standard", "kind": "standard", "base_fee": 0, "per_km": 20,
        "per_minute": 22, "minimum": 300,
        "km_rounding": "started_unit", "minute_rounding": "started_unit"
    })
}

fn proposal(name: &str) -> Value {
    json!({"provider": "bolt", "name": name, "cards": [card()]})
}

async fn bolt_offers(app: &TestApp) -> Vec<(String, String)> {
    let (_, body) = app.get("/v1/catalogue").await;
    body["catalogue"]["providers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == "bolt")
        .unwrap()["offers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| (o["id"].as_str().unwrap().into(), o["name"].as_str().unwrap().into()))
        .collect()
}

#[tokio::test]
async fn a_proposal_is_listed_but_not_offered_until_confirmed() {
    let Some(app) = TestApp::start().await else { return };
    let a = app.register().await;
    let (status, body) = app.post_as(&a, "/v1/reports", &proposal("Fiat 500")).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["status"], "pending");
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["proposals"][0]["report"], body["report"]);
    assert_eq!(catalogue["proposals"][0]["provider"], "bolt");
    assert_eq!(catalogue["proposals"][0]["name"], "Fiat 500");
    assert_eq!(bolt_offers(&app).await.len(), 1);
}

#[tokio::test]
async fn a_confirmed_proposal_becomes_an_offer() {
    let Some(app) = TestApp::start().await else { return };
    let (a, b) = (app.register().await, app.register().await);
    let (_, body) = app.post_as(&a, "/v1/reports", &proposal("Fiat 500")).await;
    let id = body["report"].as_str().unwrap();
    let (_, vote) = app
        .post_as(&b, &format!("/v1/reports/{id}/votes"), &json!({"answer": "yes"}))
        .await;
    assert_eq!(vote["status"], "accepted");
    assert!(bolt_offers(&app).await.contains(&("bolt.fiat-500".into(), "Fiat 500".into())));
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["proposals"], json!([]));
    let offer = catalogue["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["offer"] == "bolt.fiat-500")
        .unwrap();
    assert_eq!(offer["confirmed_on"], TODAY);
}

#[tokio::test]
async fn the_same_proposal_from_another_install_confirms_it() {
    let Some(app) = TestApp::start().await else { return };
    let (a, b) = (app.register().await, app.register().await);
    app.post_as(&a, "/v1/reports", &proposal("Fiat 500")).await;
    let (status, body) = app.post_as(&b, "/v1/reports", &proposal("  fiat 500 ")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "accepted");
}

#[tokio::test]
async fn a_proposal_named_like_an_existing_offer_reports_on_it() {
    let Some(app) = TestApp::start().await else { return };
    let a = app.register().await;
    let (status, _) = app.post_as(&a, "/v1/reports", &proposal("opel corsa")).await;
    assert_eq!(status, StatusCode::CREATED);
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["proposals"], json!([]));
    let corsa = catalogue["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["offer"] == "bolt.corsa")
        .unwrap()
        .clone();
    assert_eq!(corsa["pending"]["cards"][0]["per_km"], 20);
}

#[tokio::test]
async fn a_taken_offer_id_gets_a_number() {
    let Some(app) = TestApp::start().await else { return };
    sqlx::query(
        "INSERT INTO offers (id, provider_id, name, cards)
         SELECT 'bolt.fiat-500', 'bolt', 'Fiat 500 (old)', cards FROM offers WHERE id = 'bolt.corsa'",
    )
    .execute(&app.pool)
    .await
    .unwrap();
    let (a, b) = (app.register().await, app.register().await);
    app.post_as(&a, "/v1/reports", &proposal("Fiat 500")).await;
    app.post_as(&b, "/v1/reports", &proposal("Fiat 500")).await;
    assert!(bolt_offers(&app).await.contains(&("bolt.fiat-500-2".into(), "Fiat 500".into())));
}

#[tokio::test]
async fn bad_proposals_are_rejected() {
    let Some(app) = TestApp::start().await else { return };
    let a = app.register().await;
    let unknown = json!({"provider": "nope", "name": "X", "cards": [card()]});
    assert_eq!(app.post_as(&a, "/v1/reports", &unknown).await.0, StatusCode::NOT_FOUND);
    for name in ["", "   ", &"x".repeat(61)] {
        let (status, _) = app.post_as(&a, "/v1/reports", &proposal(name)).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{name:?}");
    }
    let no_name = json!({"provider": "bolt", "cards": [card()]});
    assert_eq!(app.post_as(&a, "/v1/reports", &no_name).await.0, StatusCode::BAD_REQUEST);
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p whichride-server --test proposals && cargo test -p whichride-server --lib crowd`
Expected: compile error (`offer_id_for` not found); after a stub, the proposal tests FAIL with
`400`.

- [ ] **Step 3: Implement**

In `crates/server/src/crowd.rs`:

Add `pub const MAX_NAME_CHARS: usize = 60;` and `const MAX_ID_SLUG_CHARS: usize = 40;`.

Replace the `_ => Err(ApiError::BadRequest)` arm of `post_report` with:

```rust
        (None, Some(provider), Some(name)) => {
            let name = name.trim();
            if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
                return Err(ApiError::BadRequest);
            }
            if state.providers.provider(&provider).is_none() {
                return Err(ApiError::NotFound);
            }
            let existing: Option<String> = sqlx::query_scalar(
                "SELECT id FROM offers WHERE provider_id = $1 AND lower(name) = lower($2)",
            )
            .bind(&provider.0)
            .bind(name)
            .fetch_optional(&state.db)
            .await?;
            if let Some(offer) = existing {
                let offer = OfferId(offer);
                check_cards(&offer, &body.cards)?;
                return report_on_offer(&state, &install, &offer, &body.cards, today).await;
            }
            check_cards(&OfferId(offer_id_for(&provider.0, name)), &body.cards)?;
            propose(&state, &install, &provider, name, &body.cards, today).await
        }
        _ => Err(ApiError::BadRequest),
```

Add:

```rust
/// A stable offer ID from a provider and a model name: lowercase ASCII letters and digits,
/// everything else as single dashes, at most 40 characters after the provider.
pub fn offer_id_for(provider: &str, name: &str) -> String {
    let mut slug = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug: String = slug.trim_end_matches('-').chars().take(MAX_ID_SLUG_CHARS).collect();
    let slug = slug.trim_end_matches('-');
    format!("{provider}.{}", if slug.is_empty() { "offer" } else { slug })
}

async fn propose(
    state: &AppState,
    install: &Install,
    provider: &ProviderId,
    name: &str,
    cards: &[RateCard],
    today: Day,
) -> Result<(StatusCode, Json<ReportOutcome>), ApiError> {
    let mut tx = state.db.begin().await?;
    lock_proposals(&mut tx, &provider.0).await?;
    let voter_weight = weight(&install.record, today);
    let matching: Option<ReportRow> = sqlx::query_as(
        "SELECT id, offer_id, proposed_provider, proposed_name, reporter, reporter_weight,
             cards, status
         FROM reports
         WHERE offer_id IS NULL AND status = 'pending' AND proposed_provider = $1
             AND lower(proposed_name) = lower($2) AND cards = $3
         ORDER BY created_on, id LIMIT 1 FOR UPDATE",
    )
    .bind(&provider.0)
    .bind(name)
    .bind(SqlJson(cards))
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(report) = matching {
        let status = vote_on(&mut tx, &report, install, Answer::Yes, voter_weight, today).await?;
        tx.commit().await?;
        return Ok((StatusCode::OK, Json(ReportOutcome { report: Some(report.id), status })));
    }
    sqlx::query(
        "UPDATE reports SET status = 'superseded', decided_on = $4
         WHERE offer_id IS NULL AND status = 'pending' AND reporter = $1
             AND proposed_provider = $2 AND lower(proposed_name) = lower($3)",
    )
    .bind(install.id)
    .bind(&provider.0)
    .bind(name)
    .bind(today.0)
    .execute(&mut *tx)
    .await?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO reports (id, proposed_provider, proposed_name, reporter, reporter_weight,
             cards, created_on)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(&provider.0)
    .bind(name)
    .bind(install.id)
    .bind(weight_column(voter_weight))
    .bind(SqlJson(cards))
    .bind(today.0)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(ReportOutcome { report: Some(id), status: ReportStatus::Pending })))
}

/// Creates the offer of an accepted proposal, with a free ID.
async fn create_offer(
    tx: &mut Transaction<'_, Postgres>,
    report: &ReportRow,
    today: Day,
) -> Result<String, ApiError> {
    let (Some(provider), Some(name)) = (&report.proposed_provider, &report.proposed_name) else {
        return Err(ApiError::internal("proposal without provider or name"));
    };
    let base = offer_id_for(provider, name);
    for attempt in 1..=100 {
        let id = if attempt == 1 { base.clone() } else { format!("{base}-{attempt}") };
        let inserted = sqlx::query(
            "INSERT INTO offers (id, provider_id, name, cards, confirmed_on)
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT (id) DO NOTHING",
        )
        .bind(&id)
        .bind(provider)
        .bind(name)
        .bind(&report.cards)
        .bind(today.0)
        .execute(&mut **tx)
        .await?
        .rows_affected();
        if inserted == 1 {
            sqlx::query("UPDATE reports SET offer_id = $2 WHERE id = $1")
                .bind(report.id)
                .bind(&id)
                .execute(&mut **tx)
                .await?;
            return Ok(id);
        }
    }
    Err(ApiError::internal("no free offer ID"))
}
```

Replace `accept` with this version, which handles both kinds of report:

```rust
/// Makes a report the price of its offer — creating the offer for a proposal — supersedes the
/// other pending reports about the same thing, and settles every install involved
/// (decision 39).
pub(crate) async fn accept(
    tx: &mut Transaction<'_, Postgres>,
    report: &ReportRow,
    today: Day,
) -> Result<(), ApiError> {
    sqlx::query("UPDATE reports SET status = 'accepted', decided_on = $2 WHERE id = $1")
        .bind(report.id)
        .bind(today.0)
        .execute(&mut **tx)
        .await?;
    let offer = match &report.offer_id {
        Some(offer) => {
            sqlx::query("UPDATE offers SET cards = $2, confirmed_on = $3 WHERE id = $1")
                .bind(offer)
                .bind(&report.cards)
                .bind(today.0)
                .execute(&mut **tx)
                .await?;
            offer.clone()
        }
        None => create_offer(tx, report, today).await?,
    };
    sqlx::query(
        "INSERT INTO price_history (offer_id, cards, report_id, from_day) VALUES ($1, $2, $3, $4)",
    )
    .bind(&offer)
    .bind(&report.cards)
    .bind(report.id)
    .bind(today.0)
    .execute(&mut **tx)
    .await?;
    settle_report(tx, report.id, report.reporter, true).await?;

    let losers: Vec<(Uuid, Uuid)> = match &report.offer_id {
        Some(offer) => {
            sqlx::query_as(
                "UPDATE reports SET status = 'superseded', decided_on = $2
                 WHERE offer_id = $1 AND status = 'pending' RETURNING id, reporter",
            )
            .bind(offer)
            .bind(today.0)
            .fetch_all(&mut **tx)
            .await?
        }
        None => {
            sqlx::query_as(
                "UPDATE reports SET status = 'superseded', decided_on = $3
                 WHERE offer_id IS NULL AND status = 'pending' AND proposed_provider = $1
                     AND lower(proposed_name) = lower($2)
                 RETURNING id, reporter",
            )
            .bind(&report.proposed_provider)
            .bind(&report.proposed_name)
            .bind(today.0)
            .fetch_all(&mut **tx)
            .await?
        }
    };
    for (loser, reporter) in losers {
        settle_report(tx, loser, reporter, false).await?;
    }
    Ok(())
}
```

(`report.offer_id` is read from the row loaded before `create_offer` set it, so a proposal
takes the `None` branches.)

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p whichride-server && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS, including all of Task 8's tests.

- [ ] **Step 5: Commit**

```bash
git add crates/server
git commit -m "feat(server): crowd proposals of new offers

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 10: Routing, geocoding and autocomplete proxy

Spec §4, §8, §9: the app sends start and destination in a POST body; the server adds the
OpenRouteService key, forwards, and returns km and minutes — or `503 route_service_busy`, which
makes the app show the manual km/min fields. Nothing is cached or logged.

OpenRouteService reference (check paths, parameters and response fields against it before
implementing): <https://giscience.github.io/openrouteservice/api-reference/> — directions
`POST /v2/directions/driving-car` (body `{"coordinates": [[lon, lat], [lon, lat]]}`, response
`routes[0].summary.distance` in metres and `.duration` in seconds; ORS omits a zero value),
geocoding `GET /geocode/search` and `GET /geocode/autocomplete` (Pelias: `text`, `size`,
`boundary.country`, `focus.point.lat`, `focus.point.lon`; response GeoJSON `features[]` with
`properties.label` and `geometry.coordinates` = `[lon, lat]`). The key goes in the
`Authorization` header, never in the URL.

**Files:**
- Create: `crates/server/src/ors.rs`, `crates/server/src/limits.rs`, `crates/server/src/routing.rs`
- Modify: `crates/server/src/lib.rs`, `state.rs`, `commands.rs`, `app.rs`
- Modify: `crates/server/tests/common/mod.rs`
- Create: `crates/server/tests/routing.rs`, `crates/server/tests/privacy.rs`

**Interfaces:**
- Consumes: `Authed<T>`, `Install` (Task 7); `Coordinate`, `Trip` (core); `http::client` (Task 6).
- Produces:
  - `ors::Ors::new(base: Url, key: String) -> Ors`, `Ors::with_timeout(self, Duration) -> Ors`;
    `async fn route(&self, from: Coordinate, to: Coordinate) -> Result<Trip, OrsError>`;
    `async fn search(&self, kind: SearchKind, text: &str, near: Option<Coordinate>) -> Result<Vec<Place>, OrsError>`.
  - `ors::SearchKind::{Geocode, Autocomplete}`; `ors::Place { label: String, at: Coordinate }`
    (Serialize); `ors::OrsError::{Busy, NoRoute, Unavailable}`; `impl From<OrsError> for ApiError`
    (`Busy`/`Unavailable` → `RouteServiceBusy`, `NoRoute` → `NoRoute`).
  - `limits::Kind::{Route, Geocode, Autocomplete, Expand}` with `per_hour()` = 60, 60, 600, 60;
    `limits::RateLimits::new()`, `RateLimits::with_per_hour(u32)` (same limit for every kind,
    for tests), `fn allow(&self, install: Uuid, kind: Kind, now_secs: u64) -> bool`;
    `limits::check(state: &AppState, install: &Install, kind: Kind) -> Result<(), ApiError>`.
  - Handlers `routing::post_route`, `post_geocode`, `post_autocomplete`;
    `routing::MAX_SEARCH_CHARS = 200`.
  - `AppState` gains `ors: Arc<Ors>` and `limits: Arc<RateLimits>`.
  - Test support: `common::Options { integrity: Integrity, per_hour: Option<u32> }` (Default:
    `Off`, `None`), `TestApp::start_with(Options)`, `TestApp.upstream: MockServer` (the fake
    OpenRouteService; Task 11 reuses it for Google's short links), `common::capture_logs()`.

- [ ] **Step 1: Write the failing tests**

Unit tests at the bottom of `crates/server/src/limits.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: u64 = 3_600;

    #[test]
    fn allows_up_to_the_limit_per_hour() {
        let limits = RateLimits::with_per_hour(2);
        let install = Uuid::new_v4();
        assert!(limits.allow(install, Kind::Route, 10 * HOUR));
        assert!(limits.allow(install, Kind::Route, 10 * HOUR + 1));
        assert!(!limits.allow(install, Kind::Route, 10 * HOUR + 2));
        assert!(limits.allow(install, Kind::Route, 11 * HOUR));
    }

    #[test]
    fn installs_and_kinds_have_separate_counters() {
        let limits = RateLimits::with_per_hour(1);
        let (a, b) = (Uuid::new_v4(), Uuid::new_v4());
        assert!(limits.allow(a, Kind::Route, 0));
        assert!(limits.allow(b, Kind::Route, 0));
        assert!(limits.allow(a, Kind::Geocode, 0));
        assert!(!limits.allow(a, Kind::Route, 0));
    }

    #[test]
    fn the_default_limits() {
        assert_eq!(Kind::Route.per_hour(), 60);
        assert_eq!(Kind::Autocomplete.per_hour(), 600);
    }
}
```

Unit tests at the bottom of `crates/server/src/ors.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degrees_convert_both_ways() {
        assert_eq!(degrees(525_219_184), 52.521_918_4);
        assert_eq!(to_e7(52.521_918_4), Some(525_219_184));
        assert_eq!(to_e7(-0.000_000_05), Some(-1)); // rounds half away from zero
        assert_eq!(to_e7(f64::NAN), None);
        assert_eq!(to_e7(1e300), None);
    }

    #[test]
    fn whole_units_round_and_refuse_nonsense() {
        assert_eq!(whole(4_321.5), Some(4_322));
        assert_eq!(whole(0.0), Some(0));
        assert_eq!(whole(-1.0), None);
        assert_eq!(whole(f64::INFINITY), None);
        assert_eq!(whole(1e12), None);
    }
}
```

Add to `tests/common/mod.rs` (and change `start_with_integrity` to call `start_with`):

```rust
use std::io::Write;
use std::sync::Mutex;

use whichride_server::integrity::Integrity;
use whichride_server::limits::RateLimits;
use whichride_server::ors::Ors;
use wiremock::MockServer;

pub struct Options {
    pub integrity: Integrity,
    pub per_hour: Option<u32>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            integrity: Integrity::Off,
            per_hour: None,
        }
    }
}

// In TestApp: a new field
//     pub upstream: MockServer,
// and in start_with(options):
//     let upstream = MockServer::start().await;
//     let ors_base = url::Url::parse(&format!("{}/", upstream.uri())).unwrap();
//     … state fields:
//     integrity: Arc::new(options.integrity),
//     ors: Arc::new(Ors::new(ors_base, "test-key".into()).with_timeout(Duration::from_millis(500))),
//     limits: Arc::new(options.per_hour.map_or_else(RateLimits::new, RateLimits::with_per_hour)),

/// Everything logged on this thread while the guard lives, at `trace` for the server's own
/// targets and tower-http.
pub fn capture_logs() -> (Arc<Mutex<Vec<u8>>>, tracing::subscriber::DefaultGuard) {
    #[derive(Clone)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let sink = Sink(Arc::clone(&buffer));
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter("whichride_server=trace,tower_http=trace")
        .with_ansi(false)
        .with_writer(move || sink.clone())
        .finish();
    (buffer, tracing::subscriber::set_default(subscriber))
}
```

(Add `tracing` and `tracing-subscriber` to `[dev-dependencies]` with `.workspace = true` if the
test crate does not see them.)

`crates/server/tests/routing.rs`:

```rust
mod common;

use axum::http::StatusCode;
use common::{Options, TestApp};
use serde_json::json;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, ResponseTemplate};

const ALEXANDERPLATZ: (i32, i32) = (525_219_184, 134_132_147);
const HAUPTBAHNHOF: (i32, i32) = (525_250_839, 133_694_195);

fn route_body() -> serde_json::Value {
    json!({
        "from": {"lat_e7": ALEXANDERPLATZ.0, "lon_e7": ALEXANDERPLATZ.1},
        "to": {"lat_e7": HAUPTBAHNHOF.0, "lon_e7": HAUPTBAHNHOF.1}
    })
}

async fn mock_route(app: &TestApp, response: ResponseTemplate) {
    Mock::given(method("POST"))
        .and(path("/v2/directions/driving-car"))
        .and(header("authorization", "test-key"))
        .and(body_json(json!({"coordinates": [[13.413_214_7, 52.521_918_4], [13.369_419_5, 52.525_083_9]]})))
        .respond_with(response)
        .mount(&app.upstream)
        .await;
}

#[tokio::test]
async fn a_route_returns_metres_and_seconds() {
    let Some(app) = TestApp::start().await else { return };
    let token = app.register().await;
    mock_route(
        &app,
        ResponseTemplate::new(200).set_body_json(
            json!({"routes": [{"summary": {"distance": 4_321.6, "duration": 612.4}}]}),
        ),
    )
    .await;
    let (status, body) = app.post_as(&token, "/v1/route", &route_body()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, json!({"metres": 4_322, "seconds": 612}));
}

#[tokio::test]
async fn a_route_needs_a_registered_install() {
    let Some(app) = TestApp::start().await else { return };
    let (status, _) = app.post("/v1/route", &route_body()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn upstream_trouble_means_route_service_busy() {
    for upstream in [
        ResponseTemplate::new(429),
        ResponseTemplate::new(403).set_body_string("Daily quota reached"),
        ResponseTemplate::new(500).set_body_string("<html>stack trace</html>"),
        ResponseTemplate::new(200).set_body_string("not json"),
        ResponseTemplate::new(200).set_delay(std::time::Duration::from_secs(2)),
    ] {
        let Some(app) = TestApp::start().await else { return };
        let token = app.register().await;
        mock_route(&app, upstream).await;
        let (status, body) = app.post_as(&token, "/v1/route", &route_body()).await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body, json!({"error": "route_service_busy"}));
    }
}

#[tokio::test]
async fn no_route_is_its_own_error() {
    let Some(app) = TestApp::start().await else { return };
    let token = app.register().await;
    mock_route(&app, ResponseTemplate::new(404)).await;
    let (status, body) = app.post_as(&token, "/v1/route", &route_body()).await;
    assert_eq!((status, body), (StatusCode::UNPROCESSABLE_ENTITY, json!({"error": "no_route"})));
}

#[tokio::test]
async fn coordinates_off_the_earth_never_reach_the_routing_service() {
    let Some(app) = TestApp::start().await else { return };
    let token = app.register().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200)).expect(0).mount(&app.upstream).await;
    let body = json!({
        "from": {"lat_e7": 900_000_001, "lon_e7": 0},
        "to": {"lat_e7": 0, "lon_e7": 0}
    });
    assert_eq!(app.post_as(&token, "/v1/route", &body).await.0, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn each_install_has_an_hourly_limit() {
    let Some(app) = TestApp::start_with(Options { per_hour: Some(2), ..Options::default() }).await
    else {
        return;
    };
    let token = app.register().await;
    mock_route(
        &app,
        ResponseTemplate::new(200).set_body_json(json!({"routes": [{"summary": {}}]})),
    )
    .await;
    assert_eq!(app.post_as(&token, "/v1/route", &route_body()).await.1, json!({"metres": 0, "seconds": 0}));
    assert_eq!(app.post_as(&token, "/v1/route", &route_body()).await.0, StatusCode::OK);
    let (status, body) = app.post_as(&token, "/v1/route", &route_body()).await;
    assert_eq!((status, body), (StatusCode::TOO_MANY_REQUESTS, json!({"error": "too_many_requests"})));
    let other = app.register().await;
    assert_eq!(app.post_as(&other, "/v1/route", &route_body()).await.0, StatusCode::OK);
}

fn features() -> serde_json::Value {
    json!({"features": [
        {"properties": {"label": "Alexanderplatz, Berlin, Germany"},
         "geometry": {"coordinates": [13.413_214_7, 52.521_918_4]}},
        {"properties": {"label": "Broken"}, "geometry": {"coordinates": [999.0, 52.0]}},
        {"properties": {}, "geometry": {"coordinates": [13.0, 52.0]}}
    ]})
}

#[tokio::test]
async fn geocoding_returns_places_near_the_user() {
    let Some(app) = TestApp::start().await else { return };
    let token = app.register().await;
    Mock::given(method("GET"))
        .and(path("/geocode/search"))
        .and(header("authorization", "test-key"))
        .and(query_param("text", "Alexanderplatz"))
        .and(query_param("boundary.country", "DE"))
        .and(query_param("focus.point.lat", "52.5"))
        .and(query_param("focus.point.lon", "13.4"))
        .respond_with(ResponseTemplate::new(200).set_body_json(features()))
        .mount(&app.upstream)
        .await;
    let body = json!({"text": " Alexanderplatz ", "near": {"lat_e7": 525_000_000, "lon_e7": 134_000_000}});
    let (status, places) = app.post_as(&token, "/v1/geocode", &body).await;
    assert_eq!(status, StatusCode::OK, "{places}");
    // Places with impossible coordinates or no label are dropped.
    assert_eq!(
        places,
        json!({"places": [{"label": "Alexanderplatz, Berlin, Germany",
                           "at": {"lat_e7": ALEXANDERPLATZ.0, "lon_e7": ALEXANDERPLATZ.1}}]})
    );
}

#[tokio::test]
async fn autocomplete_uses_its_own_endpoint() {
    let Some(app) = TestApp::start().await else { return };
    let token = app.register().await;
    Mock::given(method("GET"))
        .and(path("/geocode/autocomplete"))
        .and(query_param("text", "Alexan"))
        .respond_with(ResponseTemplate::new(200).set_body_json(features()))
        .expect(1)
        .mount(&app.upstream)
        .await;
    let (status, _) = app.post_as(&token, "/v1/autocomplete", &json!({"text": "Alexan"})).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn search_text_must_be_short_and_not_blank() {
    let Some(app) = TestApp::start().await else { return };
    let token = app.register().await;
    for text in ["", "   ", &"x".repeat(201)] {
        let (status, _) = app.post_as(&token, "/v1/geocode", &json!({"text": text})).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{text:?}");
    }
}
```

`crates/server/tests/privacy.rs` (Review Focus 1):

```rust
mod common;

use axum::http::StatusCode;
use common::{TestApp, capture_logs};
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

/// Fragments of the coordinates and the search text below, as they could appear in any format.
const SECRETS: [&str; 7] = [
    "5219184", "52.52", "4132147", "13.41", "5250839", "Alexanderplatz", "lat_e7",
];

fn assert_clean(logs: &[u8]) {
    let logs = String::from_utf8_lossy(logs);
    assert!(logs.contains("/v1/"), "the capture works and logs routes:\n{logs}");
    for secret in SECRETS {
        assert!(!logs.contains(secret), "logs contain {secret:?}:\n{logs}");
    }
}

#[tokio::test]
async fn routing_logs_contain_no_location() {
    let Some(app) = TestApp::start().await else { return };
    let token = app.register().await;
    Mock::given(method("POST"))
        .and(path("/v2/directions/driving-car"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"routes": [{"summary": {"distance": 1.0, "duration": 1.0}}]}),
        ))
        .mount(&app.upstream)
        .await;
    let (logs, _guard) = capture_logs();
    let body = json!({
        "from": {"lat_e7": 525_219_184, "lon_e7": 134_132_147},
        "to": {"lat_e7": 525_250_839, "lon_e7": 133_694_195}
    });
    assert_eq!(app.post_as(&token, "/v1/route", &body).await.0, StatusCode::OK);
    assert_clean(&logs.lock().unwrap());
}

#[tokio::test]
async fn failures_log_no_location_either() {
    let Some(app) = TestApp::start().await else { return };
    let token = app.register().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500).set_body_string("at 52.5219184,13.4132147"))
        .mount(&app.upstream)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Alexanderplatz failed"))
        .mount(&app.upstream)
        .await;
    let (logs, _guard) = capture_logs();
    let route = json!({
        "from": {"lat_e7": 525_219_184, "lon_e7": 134_132_147},
        "to": {"lat_e7": 525_250_839, "lon_e7": 133_694_195}
    });
    app.post_as(&token, "/v1/route", &route).await;
    app.post_as(&token, "/v1/geocode", &json!({"text": "Alexanderplatz"})).await;
    // A path a client made up is logged as "unmatched", not as written.
    app.get("/v1/route/52.5219184,13.4132147").await;
    assert_clean(&logs.lock().unwrap());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p whichride-server`
Expected: compile errors (`ors`, `limits` modules not found).

- [ ] **Step 3: Implement**

`crates/server/src/limits.rs` (add `pub mod limits;` to `lib.rs`), above its tests:

```rust
//! Per-install hourly limits, in memory: one Cloud Run instance (spec §2), and a restart that
//! forgets the counters costs at most one extra hour's allowance. OpenRouteService's own daily
//! quota is the hard cap behind these.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::auth::Install;
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Route,
    Geocode,
    Autocomplete,
    Expand,
}

impl Kind {
    /// Generous for a person, small against the free daily quotas.
    pub fn per_hour(self) -> u32 {
        match self {
            Kind::Route | Kind::Geocode | Kind::Expand => 60,
            Kind::Autocomplete => 600,
        }
    }
}

/// Counters are dropped when the map grows past this, keeping only the current hour.
const MAX_ENTRIES: usize = 100_000;

pub struct RateLimits {
    override_per_hour: Option<u32>,
    windows: Mutex<HashMap<(Uuid, Kind), (u64, u32)>>,
}

impl RateLimits {
    pub fn new() -> RateLimits {
        RateLimits { override_per_hour: None, windows: Mutex::new(HashMap::new()) }
    }

    /// One limit for every kind; for tests.
    pub fn with_per_hour(per_hour: u32) -> RateLimits {
        RateLimits { override_per_hour: Some(per_hour), ..RateLimits::new() }
    }

    pub fn allow(&self, install: Uuid, kind: Kind, now_secs: u64) -> bool {
        let hour = now_secs / 3_600;
        let limit = self.override_per_hour.unwrap_or_else(|| kind.per_hour());
        let mut windows = self.windows.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if windows.len() >= MAX_ENTRIES {
            windows.retain(|_, (window, _)| *window == hour);
        }
        let entry = windows.entry((install, kind)).or_insert((hour, 0));
        if entry.0 != hour {
            *entry = (hour, 0);
        }
        if entry.1 >= limit {
            return false;
        }
        entry.1 += 1;
        true
    }
}

impl Default for RateLimits {
    fn default() -> RateLimits {
        RateLimits::new()
    }
}

pub fn check(state: &AppState, install: &Install, kind: Kind) -> Result<(), ApiError> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    if state.limits.allow(install.id, kind, now) {
        Ok(())
    } else {
        Err(ApiError::TooManyRequests)
    }
}
```

`crates/server/src/ors.rs` (add `pub mod ors;` to `lib.rs`), above its tests:

```rust
//! OpenRouteService behind our key (spec §8). Floats exist only here, at the boundary: they are
//! converted to whole metres, seconds and 1e-7 degrees right away. Request data never appears
//! in an error or a log line: errors are mapped to three kinds and only the status is logged.

use std::time::Duration;

use carsharing_core::{Coordinate, Trip};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::ApiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    Geocode,
    Autocomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Place {
    pub label: String,
    pub at: Coordinate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrsError {
    /// Quota used up or rate limited (403, 429).
    Busy,
    /// No route between the points (404, 400).
    NoRoute,
    /// Down, slow or answering nonsense.
    Unavailable,
}

impl From<OrsError> for ApiError {
    fn from(error: OrsError) -> ApiError {
        match error {
            OrsError::NoRoute => ApiError::NoRoute,
            OrsError::Busy | OrsError::Unavailable => ApiError::RouteServiceBusy,
        }
    }
}

/// Places asked for per search.
const RESULTS: &str = "5";

pub struct Ors {
    http: reqwest::Client,
    base: Url,
    key: String,
}

#[derive(Deserialize)]
struct Directions {
    routes: Vec<Route>,
}

#[derive(Deserialize)]
struct Route {
    summary: Summary,
}

#[derive(Deserialize)]
struct Summary {
    #[serde(default)]
    distance: f64,
    #[serde(default)]
    duration: f64,
}

#[derive(Deserialize)]
struct Features {
    features: Vec<Feature>,
}

#[derive(Deserialize)]
struct Feature {
    properties: Properties,
    geometry: Geometry,
}

#[derive(Deserialize)]
struct Properties {
    #[serde(default)]
    label: Option<String>,
}

#[derive(Deserialize)]
struct Geometry {
    coordinates: (f64, f64),
}

impl Ors {
    pub fn new(base: Url, key: String) -> Ors {
        Ors { http: crate::http::client(Duration::from_secs(8)), base, key }
    }

    pub fn with_timeout(self, timeout: Duration) -> Ors {
        Ors { http: crate::http::client(timeout), ..self }
    }

    pub async fn route(&self, from: Coordinate, to: Coordinate) -> Result<Trip, OrsError> {
        let url = self.base.join("v2/directions/driving-car").map_err(|_| OrsError::Unavailable)?;
        let body = serde_json::json!({
            "coordinates": [
                [degrees(from.lon_e7), degrees(from.lat_e7)],
                [degrees(to.lon_e7), degrees(to.lat_e7)],
            ]
        });
        let response = self.http.post(url).header("Authorization", &self.key).json(&body).send().await;
        let directions: Directions = read(response, true).await?;
        let summary = &directions.routes.first().ok_or(OrsError::NoRoute)?.summary;
        Ok(Trip {
            metres: whole(summary.distance).ok_or(OrsError::Unavailable)?,
            seconds: whole(summary.duration).ok_or(OrsError::Unavailable)?,
        })
    }

    pub async fn search(
        &self,
        kind: SearchKind,
        text: &str,
        near: Option<Coordinate>,
    ) -> Result<Vec<Place>, OrsError> {
        let path = match kind {
            SearchKind::Geocode => "geocode/search",
            SearchKind::Autocomplete => "geocode/autocomplete",
        };
        let url = self.base.join(path).map_err(|_| OrsError::Unavailable)?;
        let mut query = vec![
            ("text", text.to_owned()),
            ("size", RESULTS.to_owned()),
            ("boundary.country", "DE".to_owned()),
        ];
        if let Some(near) = near {
            query.push(("focus.point.lat", degrees(near.lat_e7).to_string()));
            query.push(("focus.point.lon", degrees(near.lon_e7).to_string()));
        }
        let response = self.http.get(url).header("Authorization", &self.key).query(&query).send().await;
        let features: Features = read(response, false).await?;
        Ok(features
            .features
            .into_iter()
            .filter_map(|feature| {
                let (lon, lat) = feature.geometry.coordinates;
                let at = Coordinate::new(to_e7(lat)?, to_e7(lon)?).ok()?;
                Some(Place { label: feature.properties.label?, at })
            })
            .collect())
    }
}

/// Maps the response to our three errors, logging only the status (never the URL or the body,
/// which can contain the coordinates or the search text).
async fn read<T: for<'de> Deserialize<'de>>(
    response: reqwest::Result<reqwest::Response>,
    routing: bool,
) -> Result<T, OrsError> {
    let Ok(response) = response else {
        tracing::warn!("routing service unreachable or timed out");
        return Err(OrsError::Unavailable);
    };
    let status = response.status().as_u16();
    let error = match status {
        200..=299 => return response.json().await.map_err(|_| {
            tracing::warn!("routing service answered with an unreadable body");
            OrsError::Unavailable
        }),
        403 | 429 => OrsError::Busy,
        400 | 404 if routing => OrsError::NoRoute,
        _ => OrsError::Unavailable,
    };
    tracing::warn!(status, ?error, "routing service refused");
    Err(error)
}

fn degrees(e7: i32) -> f64 {
    f64::from(e7) / 1e7
}

fn to_e7(degrees: f64) -> Option<i32> {
    let value = (degrees * 1e7).round();
    (value.is_finite() && value.abs() <= f64::from(i32::MAX)).then_some(value as i32)
}

fn whole(value: f64) -> Option<u32> {
    let value = value.round();
    (value.is_finite() && (0.0..=f64::from(u32::MAX)).contains(&value)).then_some(value as u32)
}
```

`crates/server/src/routing.rs` (add `pub mod routing;` to `lib.rs`):

```rust
//! `/v1/route`, `/v1/geocode`, `/v1/autocomplete`: the app's location data passes through to
//! OpenRouteService and back; nothing is stored, cached or logged (spec §9).

use axum::Json;
use axum::extract::State;
use carsharing_core::{Coordinate, Trip};
use serde::{Deserialize, Serialize};

use crate::auth::Authed;
use crate::error::ApiError;
use crate::limits::{self, Kind};
use crate::ors::{Place, SearchKind};
use crate::state::AppState;

pub const MAX_SEARCH_CHARS: usize = 200;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteRequest {
    pub from: Coordinate,
    pub to: Coordinate,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchRequest {
    pub text: String,
    #[serde(default)]
    pub near: Option<Coordinate>,
}

#[derive(Serialize)]
pub struct Places {
    pub places: Vec<Place>,
}

pub async fn post_route(
    State(state): State<AppState>,
    Authed { install, body }: Authed<RouteRequest>,
) -> Result<Json<Trip>, ApiError> {
    limits::check(&state, &install, Kind::Route)?;
    Ok(Json(state.ors.route(body.from, body.to).await?))
}

pub async fn post_geocode(
    state: State<AppState>,
    request: Authed<SearchRequest>,
) -> Result<Json<Places>, ApiError> {
    search(state, request, SearchKind::Geocode, Kind::Geocode).await
}

pub async fn post_autocomplete(
    state: State<AppState>,
    request: Authed<SearchRequest>,
) -> Result<Json<Places>, ApiError> {
    search(state, request, SearchKind::Autocomplete, Kind::Autocomplete).await
}

async fn search(
    State(state): State<AppState>,
    Authed { install, body }: Authed<SearchRequest>,
    search: SearchKind,
    limit: Kind,
) -> Result<Json<Places>, ApiError> {
    let text = body.text.trim();
    if text.is_empty() || text.chars().count() > MAX_SEARCH_CHARS {
        return Err(ApiError::BadRequest);
    }
    limits::check(&state, &install, limit)?;
    let places = state.ors.search(search, text, body.near).await?;
    Ok(Json(Places { places }))
}
```

`state.rs`: add `pub ors: Arc<Ors>,` and `pub limits: Arc<RateLimits>,`.
`commands.rs` `serve`: `ors: Arc::new(Ors::new(config.ors_base_url.clone(), config.ors_api_key.clone()))`,
`limits: Arc::new(RateLimits::new())`.

`app.rs` routes:

```rust
        .route("/v1/route", post(crate::routing::post_route))
        .route("/v1/geocode", post(crate::routing::post_geocode))
        .route("/v1/autocomplete", post(crate::routing::post_autocomplete))
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p whichride-server && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS. If `privacy.rs` fails because the logged route is `unmatched` for real routes,
see the `MatchedPath` note in Task 3 and move the trace layer; if it fails because a log line
contains a coordinate, find which line and remove the field — never weaken the test.

- [ ] **Step 5: Commit**

```bash
git add crates/server Cargo.lock
git commit -m "feat(server): routing, geocoding and autocomplete proxy that keeps no locations

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 11: Short-link expansion

Spec §4: "short links (`maps.app.goo.gl`) are expanded by following the redirect". Decision 31:
the core only detects them; the server expands them. The server follows each redirect by hand
and only ever fetches URLs that the core recognised as short links (forced to `https` by Task
1), so `/v1/expand` cannot be used to make the server fetch anything else.

**Files:**
- Create: `crates/server/src/expand.rs`
- Modify: `crates/server/src/lib.rs`, `state.rs`, `commands.rs`, `app.rs`
- Modify: `crates/server/tests/common/mod.rs`
- Create: `crates/server/tests/expand.rs`

**Interfaces:**
- Consumes: `parse_destination`, `Destination` (core, with Task 1's tidy short links);
  `http::client_without_redirects` (Task 6); `Authed`, `limits::check`, `Kind::Expand`.
- Produces:
  - `expand::Expander::new() -> Expander`; `Expander::with_origin(origin: Url) -> Expander` —
    every fetch goes to `origin` with the short link's path (tests);
    `async fn expand(&self, link: &str) -> Result<Destination, ExpandError>` — never returns
    `Destination::ShortLink`.
  - `expand::ExpandError::{NotAShortLink, NoDestination, Unavailable}` and
    `impl From<ExpandError> for ApiError` (`NotAShortLink` → 422 `not_a_short_link`,
    `NoDestination` → 422 `no_destination`, `Unavailable` → 503 `route_service_busy`).
  - `expand::MAX_HOPS = 5`; handler `expand::post_expand` for `POST /v1/expand` with body
    `{"url": "<shared text or link>"}`.
  - `AppState` gains `expander: Arc<Expander>`; `TestApp` builds it with
    `Expander::with_origin(<upstream mock URL>)`.

- [ ] **Step 1: Write the failing tests**

Unit tests at the bottom of `crates/server/src/expand.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_consent_page_leads_to_its_continue_target() {
        let location = "https://consent.google.com/ml?continue=https%3A%2F%2Fwww.google.com%2Fmaps%2Fplace%2FX%2F%4052.52%2C13.41%2C15z&gl=DE&hl=de&pc=m&src=1";
        assert_eq!(
            unwrap_consent(location).as_deref(),
            Some("https://www.google.com/maps/place/X/@52.52,13.41,15z")
        );
        assert_eq!(unwrap_consent("https://consent.google.de/ml?continue=https://www.google.de/maps").as_deref(),
                   Some("https://www.google.de/maps"));
    }

    #[test]
    fn other_pages_are_not_consent_pages() {
        assert_eq!(unwrap_consent("https://www.google.com/maps?continue=x"), None);
        assert_eq!(unwrap_consent("https://consent.google.com/ml"), None);
        assert_eq!(unwrap_consent("https://consent.evil.com/ml?continue=x"), None);
        assert_eq!(unwrap_consent("not a url"), None);
    }
}
```

`crates/server/tests/expand.rs`:

```rust
mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

const PLACE: &str = "https://www.google.com/maps/place/Alexanderplatz,+10178+Berlin/@52.52,13.41,15z/data=!3m1!4b1!4m6!3m5!1s0x47a84e1f1d4d3f0b:0x3b2d0f4c1c1e2a0!8m2!3d52.5219184!4d13.4132147!16zL20vMDFsNnQ1?entry=ttu";

fn alexanderplatz() -> serde_json::Value {
    json!({"type": "coordinates", "at": {"lat_e7": 525_219_184, "lon_e7": 134_132_147},
           "label": "Alexanderplatz, 10178 Berlin"})
}

async fn redirect(app: &TestApp, from: &str, to: &str) {
    Mock::given(method("GET"))
        .and(path(from))
        .respond_with(ResponseTemplate::new(302).insert_header("location", to))
        .mount(&app.upstream)
        .await;
}

#[tokio::test]
async fn a_short_link_expands_to_its_destination() {
    let Some(app) = TestApp::start().await else { return };
    let token = app.register().await;
    redirect(&app, "/AbC123", PLACE).await;
    let shared = "Alexanderplatz\nhttps://maps.app.goo.gl/AbC123?g_st=ic";
    let (status, body) = app.post_as(&token, "/v1/expand", &json!({"url": shared})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body, alexanderplatz());
}

#[tokio::test]
async fn the_eu_consent_page_is_skipped() {
    let Some(app) = TestApp::start().await else { return };
    let token = app.register().await;
    let consent = format!(
        "https://consent.google.com/ml?continue={}&gl=DE&hl=de&pc=m&src=1",
        url::form_urlencoded::byte_serialize(PLACE.as_bytes()).collect::<String>()
    );
    redirect(&app, "/AbC123", &consent).await;
    let (status, body) = app
        .post_as(&token, "/v1/expand", &json!({"url": "https://maps.app.goo.gl/AbC123"}))
        .await;
    assert_eq!((status, body), (StatusCode::OK, alexanderplatz()));
}

#[tokio::test]
async fn a_chain_of_short_links_is_followed_but_not_forever() {
    let Some(app) = TestApp::start().await else { return };
    let token = app.register().await;
    redirect(&app, "/maps/old", "https://maps.app.goo.gl/AbC123").await;
    redirect(&app, "/AbC123", PLACE).await;
    let (status, _) = app
        .post_as(&token, "/v1/expand", &json!({"url": "https://goo.gl/maps/old"}))
        .await;
    assert_eq!(status, StatusCode::OK);

    redirect(&app, "/Loop", "https://maps.app.goo.gl/Loop").await;
    let (status, body) = app
        .post_as(&token, "/v1/expand", &json!({"url": "https://maps.app.goo.gl/Loop"}))
        .await;
    assert_eq!((status, body), (StatusCode::SERVICE_UNAVAILABLE, json!({"error": "route_service_busy"})));
}

#[tokio::test]
async fn only_short_links_are_fetched() {
    let Some(app) = TestApp::start().await else { return };
    let token = app.register().await;
    Mock::given(method("GET")).respond_with(ResponseTemplate::new(302)).expect(0).mount(&app.upstream).await;
    for url in [
        "https://example.com/AbC123",
        "http://169.254.169.254/computeMetadata/v1/",
        "https://www.google.com/maps/place/X",
        "no link at all",
    ] {
        let (status, body) = app.post_as(&token, "/v1/expand", &json!({"url": url})).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{url}");
        assert_eq!(body, json!({"error": "not_a_short_link"}), "{url}");
    }
}

#[tokio::test]
async fn a_redirect_away_from_google_maps_has_no_destination() {
    let Some(app) = TestApp::start().await else { return };
    let token = app.register().await;
    redirect(&app, "/Elsewhere", "https://example.com/phishing").await;
    Mock::given(method("GET"))
        .and(path("/Gone"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&app.upstream)
        .await;
    for id in ["Elsewhere", "Gone"] {
        let url = format!("https://maps.app.goo.gl/{id}");
        let (status, body) = app.post_as(&token, "/v1/expand", &json!({"url": url})).await;
        assert_eq!((status, body), (StatusCode::UNPROCESSABLE_ENTITY, json!({"error": "no_destination"})), "{id}");
    }
}

#[tokio::test]
async fn expansion_needs_a_registered_install() {
    let Some(app) = TestApp::start().await else { return };
    let (status, _) = app.post("/v1/expand", &json!({"url": "https://maps.app.goo.gl/AbC123"})).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
```

(`url` is a normal dependency of the server crate; integration tests can use
`url::form_urlencoded`.)

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p whichride-server`
Expected: compile error (`expand` module not found).

- [ ] **Step 3: Implement**

`crates/server/src/expand.rs` (add `pub mod expand;` to `lib.rs`), above its tests:

```rust
//! Expanding Google Maps short links (spec §4, decision 31). Each redirect is followed by hand;
//! only links the core recognises as short links are ever fetched. Nothing is cached or logged.

use std::time::Duration;

use axum::Json;
use axum::extract::State;
use carsharing_core::{Destination, parse_destination};
use reqwest::header::LOCATION;
use serde::Deserialize;
use url::Url;

use crate::auth::Authed;
use crate::error::ApiError;
use crate::limits::{self, Kind};
use crate::state::AppState;

pub const MAX_HOPS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandError {
    NotAShortLink,
    NoDestination,
    Unavailable,
}

impl From<ExpandError> for ApiError {
    fn from(error: ExpandError) -> ApiError {
        match error {
            ExpandError::NotAShortLink => ApiError::NotAShortLink,
            ExpandError::NoDestination => ApiError::NoDestination,
            ExpandError::Unavailable => ApiError::RouteServiceBusy,
        }
    }
}

pub struct Expander {
    http: reqwest::Client,
    origin: Option<Url>,
}

impl Expander {
    pub fn new() -> Expander {
        Expander { http: crate::http::client_without_redirects(Duration::from_secs(5)), origin: None }
    }

    /// Sends every request to `origin` instead, keeping the path; for tests.
    pub fn with_origin(origin: Url) -> Expander {
        Expander { origin: Some(origin), ..Expander::new() }
    }

    pub async fn expand(&self, link: &str) -> Result<Destination, ExpandError> {
        let Ok(Destination::ShortLink { url }) = parse_destination(link) else {
            return Err(ExpandError::NotAShortLink);
        };
        let mut next = url;
        for _ in 0..MAX_HOPS {
            let location = self.follow(&next).await?;
            let target = unwrap_consent(&location).unwrap_or(location);
            match parse_destination(&target) {
                Ok(Destination::ShortLink { url }) => next = url,
                Ok(destination) => return Ok(destination),
                Err(_) => return Err(ExpandError::NoDestination),
            }
        }
        tracing::warn!("short link redirects too often");
        Err(ExpandError::Unavailable)
    }

    /// The redirect target of one short link, resolved against it.
    async fn follow(&self, short: &str) -> Result<String, ExpandError> {
        let short = Url::parse(short).map_err(|_| ExpandError::NotAShortLink)?;
        let fetch = match &self.origin {
            Some(origin) => origin.join(short.path()).map_err(|_| ExpandError::Unavailable)?,
            None => short.clone(),
        };
        let response = self
            .http
            .get(fetch)
            .header("User-Agent", "whichride-server")
            .send()
            .await
            .map_err(|_| {
                tracing::warn!("short link host unreachable or timed out");
                ExpandError::Unavailable
            })?;
        let status = response.status();
        if status.is_server_error() {
            tracing::warn!(status = status.as_u16(), "short link host failed");
            return Err(ExpandError::Unavailable);
        }
        if !status.is_redirection() {
            return Err(ExpandError::NoDestination);
        }
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(ExpandError::NoDestination)?;
        let resolved = short.join(location).map_err(|_| ExpandError::NoDestination)?;
        Ok(resolved.into())
    }
}

impl Default for Expander {
    fn default() -> Expander {
        Expander::new()
    }
}

/// From Europe, Google may redirect to its cookie consent page first; the real target is its
/// `continue` parameter (Review Focus 2).
fn unwrap_consent(location: &str) -> Option<String> {
    let url = Url::parse(location).ok()?;
    let host = url.host_str()?;
    if !host.starts_with("consent.google.") {
        return None;
    }
    url.query_pairs()
        .find(|(key, _)| key == "continue")
        .map(|(_, value)| value.into_owned())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpandRequest {
    pub url: String,
}

pub async fn post_expand(
    State(state): State<AppState>,
    Authed { install, body }: Authed<ExpandRequest>,
) -> Result<Json<Destination>, ApiError> {
    limits::check(&state, &install, Kind::Expand)?;
    Ok(Json(state.expander.expand(&body.url).await?))
}
```

`consent.google.` followed by anything is safe here: the `continue` value is then parsed by
the core, which accepts only Google Maps links, and is never fetched unless it is itself a
short link.

`state.rs`: `pub expander: Arc<Expander>,`. `commands.rs` `serve`:
`expander: Arc::new(Expander::new())`. `tests/common/mod.rs`:
`expander: Arc::new(Expander::with_origin(url::Url::parse(&upstream.uri()).unwrap()))`.
`app.rs`: `.route("/v1/expand", post(crate::expand::post_expand))`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p whichride-server && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: OWNER — check one real short link (optional, from the owner's machine)**

Ask the owner to share any place from Google Maps on their phone, paste the
`maps.app.goo.gl` link, and run the server locally (`PLAY_INTEGRITY=off`), register with
`curl -s -X POST localhost:8080/v1/installs -H 'content-type: application/json' -d '{}'`, then
`curl -s -X POST localhost:8080/v1/expand -H "authorization: Bearer <token>" -H 'content-type: application/json' -d '{"url":"<link>"}'`.
Expected: a `coordinates` or `place` destination. If Google answers in a way the tests did not
foresee, add that response as a new test case first, then fix.

- [ ] **Step 6: Commit**

```bash
git add crates/server
git commit -m "feat(server): expand Google Maps short links, skipping the EU consent page

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 12: Owner's emergency switch (CLI)

Spec §7.4: "revert/lock an offer, for abuse only … a server CLI command run by the owner
against the production database, not an endpoint." Plus a read-only command to see what is
going on before acting.

**Files:**
- Create: `crates/server/src/owner.rs`
- Modify: `crates/server/src/cli.rs`, `crates/server/src/main.rs`, `crates/server/src/lib.rs`
- Create: `crates/server/tests/owner.rs`

**Interfaces:**
- Consumes: the schema (Task 4), `ReportRow` semantics (Task 8).
- Produces:
  - CLI: `show-offer <OFFER>`, `lock-offer <OFFER>`, `unlock-offer <OFFER>`,
    `revert-offer <OFFER>` (each needs `DATABASE_URL`).
  - `owner::show_offer(pool, offer: &str) -> anyhow::Result<String>` — the text it prints.
  - `owner::set_locked(pool, offer: &str, locked: bool) -> anyhow::Result<()>`.
  - `owner::revert_offer(pool, offer: &str, today: Day) -> anyhow::Result<()>` — marks the
    latest non-reverted `price_history` row and its report `reverted`, puts back the value of
    the non-reverted row before it, and closes the offer's pending reports as `superseded`.
    Reverting twice goes back two values. Settles nobody: an abuse revert says nothing about
    honest voters. Fails if only the first value is left.

- [ ] **Step 1: Write the failing tests**

Add to `cli.rs` tests:

```rust
    #[test]
    fn owner_commands_parse() {
        assert_eq!(
            parse(&["lock-offer", "bolt.corsa"]).unwrap(),
            Command::LockOffer { offer: "bolt.corsa".into() }
        );
        assert_eq!(
            parse(&["revert-offer", "bolt.corsa"]).unwrap(),
            Command::RevertOffer { offer: "bolt.corsa".into() }
        );
        assert!(parse(&["lock-offer"]).is_err());
    }
```

`crates/server/tests/owner.rs`:

```rust
mod common;

use carsharing_core::Day;
use common::{TODAY, TestApp};
use serde_json::json;
use whichride_server::owner;

fn report(per_km: i64) -> serde_json::Value {
    json!({"offer": "miles.s", "cards": [{
        "label": "Standard", "kind": "standard", "base_fee": 100, "per_km": per_km,
        "per_minute": 0, "minimum": 0,
        "km_rounding": "started_unit", "minute_rounding": "started_unit"
    }]})
}

async fn accepted(app: &TestApp, per_km: i64) {
    let (a, b) = (app.register().await, app.register().await);
    app.post_as(&a, "/v1/reports", &report(per_km)).await;
    let (_, body) = app.post_as(&b, "/v1/reports", &report(per_km)).await;
    assert_eq!(body["status"], "accepted");
}

async fn per_km(app: &TestApp) -> i64 {
    let (_, body) = app.get("/v1/catalogue").await;
    body["catalogue"]["providers"][0]["offers"][1]["cards"][0]["per_km"].as_i64().unwrap()
}

#[tokio::test]
async fn locking_stops_reports_and_unlocking_restarts_them() {
    let Some(app) = TestApp::start().await else { return };
    owner::set_locked(&app.pool, "miles.s", true).await.unwrap();
    let a = app.register().await;
    assert_eq!(app.post_as(&a, "/v1/reports", &report(89)).await.0, 409);
    let (_, catalogue) = app.get("/v1/catalogue").await;
    assert_eq!(catalogue["offers"][1]["locked"], true);
    owner::set_locked(&app.pool, "miles.s", false).await.unwrap();
    assert_eq!(app.post_as(&a, "/v1/reports", &report(89)).await.0, 201);
}

#[tokio::test]
async fn reverting_restores_the_previous_price() {
    let Some(app) = TestApp::start().await else { return };
    accepted(&app, 89).await;
    accepted(&app, 300).await; // the abuse
    let c = app.register().await;
    app.post_as(&c, "/v1/reports", &report(301)).await; // pending, part of the attack
    owner::revert_offer(&app.pool, "miles.s", Day(TODAY + 1)).await.unwrap();
    assert_eq!(per_km(&app).await, 89);
    let statuses: Vec<String> =
        sqlx::query_scalar("SELECT status FROM reports ORDER BY created_on, id")
            .fetch_all(&app.pool)
            .await
            .unwrap();
    assert_eq!(statuses.iter().filter(|s| *s == "reverted").count(), 1);
    assert_eq!(statuses.iter().filter(|s| *s == "pending").count(), 0);
    owner::revert_offer(&app.pool, "miles.s", Day(TODAY + 1)).await.unwrap();
    assert_eq!(per_km(&app).await, 79); // back to the seed
}

#[tokio::test]
async fn the_first_value_cannot_be_reverted() {
    let Some(app) = TestApp::start().await else { return };
    assert!(owner::revert_offer(&app.pool, "miles.s", Day(TODAY)).await.is_err());
    assert!(owner::revert_offer(&app.pool, "nope.x", Day(TODAY)).await.is_err());
    assert!(owner::set_locked(&app.pool, "nope.x", true).await.is_err());
}

#[tokio::test]
async fn show_offer_summarises_prices_and_pending_reports() {
    let Some(app) = TestApp::start().await else { return };
    accepted(&app, 89).await;
    let c = app.register().await;
    app.post_as(&c, "/v1/reports", &report(99)).await;
    let text = owner::show_offer(&app.pool, "miles.s").await.unwrap();
    assert!(text.contains("miles.s"), "{text}");
    assert!(text.contains("per_km 89"), "{text}");
    assert!(text.contains("pending"), "{text}");
    assert!(text.contains("per_km 99"), "{text}");
    assert!(text.contains("2026-10-19"), "dates are shown as dates: {text}");
}
```

(`assert_eq!(…, 409)` compares a `StatusCode` with an integer; `StatusCode` implements
`PartialEq<u16>`.)

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p whichride-server`
Expected: compile errors (`Command::LockOffer`, `owner` not found).

- [ ] **Step 3: Implement**

Add to `Command` in `cli.rs`:

```rust
    /// Show an offer's price, history and pending reports.
    ShowOffer { offer: String },
    /// Stop all reports and votes on an offer (abuse only).
    LockOffer { offer: String },
    /// Allow reports and votes on an offer again.
    UnlockOffer { offer: String },
    /// Put back the offer's previous price and close its pending reports (abuse only).
    RevertOffer { offer: String },
```

`main.rs` dispatch:

```rust
        Command::ShowOffer { offer } => whichride_server::commands::show_offer(&offer).await,
        Command::LockOffer { offer } => whichride_server::commands::set_locked(&offer, true).await,
        Command::UnlockOffer { offer } => whichride_server::commands::set_locked(&offer, false).await,
        Command::RevertOffer { offer } => whichride_server::commands::revert_offer(&offer).await,
```

`commands.rs` gains thin wrappers (with `use crate::owner;`):

```rust
async fn owner_pool() -> anyhow::Result<sqlx::PgPool> {
    let url = config::database_url(|name| std::env::var(name).ok())?;
    db::connect(&url).await.context("cannot connect to the database")
}

pub async fn show_offer(offer: &str) -> anyhow::Result<()> {
    let pool = owner_pool().await?;
    print!("{}", owner::show_offer(&pool, offer).await?);
    pool.close().await;
    Ok(())
}

pub async fn set_locked(offer: &str, locked: bool) -> anyhow::Result<()> {
    let pool = owner_pool().await?;
    owner::set_locked(&pool, offer, locked).await?;
    println!("{offer}: {}", if locked { "locked" } else { "unlocked" });
    pool.close().await;
    Ok(())
}

pub async fn revert_offer(offer: &str) -> anyhow::Result<()> {
    let pool = owner_pool().await?;
    owner::revert_offer(&pool, offer, crate::system_today()).await?;
    println!("{offer}: reverted to its previous price");
    pool.close().await;
    Ok(())
}
```

The owner commands do not run migrations: they act on a database `serve` has already set up.

`crates/server/src/owner.rs` (add `pub mod owner;` to `lib.rs`):

```rust
//! The owner's emergency switch (spec §7.4). Run from a terminal with the production
//! `DATABASE_URL`; there is no HTTP route to any of this.

use std::fmt::Write as _;

use anyhow::{Context, bail};
use carsharing_core::{Day, RateCard};
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

pub async fn set_locked(pool: &PgPool, offer: &str, locked: bool) -> anyhow::Result<()> {
    let updated = sqlx::query("UPDATE offers SET locked = $2 WHERE id = $1")
        .bind(offer)
        .bind(locked)
        .execute(pool)
        .await?
        .rows_affected();
    if updated == 0 {
        bail!("no offer {offer:?}");
    }
    Ok(())
}

pub async fn revert_offer(pool: &PgPool, offer: &str, today: Day) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT 1 FROM offers WHERE id = $1 FOR UPDATE")
        .bind(offer)
        .fetch_optional(&mut *tx)
        .await?
        .with_context(|| format!("no offer {offer:?}"))?;
    let history: Vec<(i64, Option<Uuid>, Json<Vec<RateCard>>)> = sqlx::query_as(
        "SELECT seq, report_id, cards FROM price_history
         WHERE offer_id = $1 AND NOT reverted ORDER BY seq DESC LIMIT 2",
    )
    .bind(offer)
    .fetch_all(&mut *tx)
    .await?;
    let [(latest, latest_report, _), (_, _, previous)] = &history[..] else {
        bail!("{offer} has only its first price; nothing to revert");
    };
    sqlx::query("UPDATE price_history SET reverted = true WHERE seq = $1")
        .bind(latest)
        .execute(&mut *tx)
        .await?;
    if let Some(report) = latest_report {
        sqlx::query("UPDATE reports SET status = 'reverted' WHERE id = $1")
            .bind(report)
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query(
        "UPDATE reports SET status = 'superseded', decided_on = $2
         WHERE offer_id = $1 AND status = 'pending'",
    )
    .bind(offer)
    .bind(today.0)
    .execute(&mut *tx)
    .await?;
    sqlx::query("UPDATE offers SET cards = $2 WHERE id = $1")
        .bind(offer)
        .bind(previous)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn show_offer(pool: &PgPool, offer: &str) -> anyhow::Result<String> {
    let (name, cards, confirmed, locked): (String, Json<Vec<RateCard>>, Option<i32>, bool) =
        sqlx::query_as("SELECT name, cards, confirmed_on, locked FROM offers WHERE id = $1")
            .bind(offer)
            .fetch_optional(pool)
            .await?
            .with_context(|| format!("no offer {offer:?}"))?;
    let mut text = String::new();
    writeln!(text, "{offer} ({name}){}", if locked { " LOCKED" } else { "" })?;
    writeln!(text, "confirmed: {}", confirmed.map_or("never".into(), date))?;
    for card in &cards.0 {
        writeln!(text, "  {}", describe(card))?;
    }
    writeln!(text, "history (newest first):")?;
    let history: Vec<(Json<Vec<RateCard>>, i32, Option<Uuid>, bool)> = sqlx::query_as(
        "SELECT cards, from_day, report_id, reverted FROM price_history WHERE offer_id = $1
         ORDER BY seq DESC LIMIT 10",
    )
    .bind(offer)
    .fetch_all(pool)
    .await?;
    for (cards, day, report, reverted) in history {
        let source = report.map_or("seed".to_owned(), |id| format!("report {id}"));
        let reverted = if reverted { " REVERTED" } else { "" };
        writeln!(text, "  {} from {source}{reverted}", date(day))?;
        for card in &cards.0 {
            writeln!(text, "    {}", describe(card))?;
        }
    }
    writeln!(text, "reports:")?;
    let reports: Vec<(Uuid, String, i32, Json<Vec<RateCard>>, i64, i64)> = sqlx::query_as(
        "SELECT r.id, r.status, r.created_on, r.cards,
             count(v.*) FILTER (WHERE v.answer = 'yes'), count(v.*) FILTER (WHERE v.answer = 'no')
         FROM reports r LEFT JOIN votes v ON v.report_id = r.id
         WHERE r.offer_id = $1 GROUP BY r.id ORDER BY r.created_on DESC, r.id LIMIT 20",
    )
    .bind(offer)
    .fetch_all(pool)
    .await?;
    for (id, status, day, cards, yes, no) in reports {
        writeln!(text, "  {id} {status} {} yes {yes} no {no}", date(day))?;
        for card in &cards.0 {
            writeln!(text, "    {}", describe(card))?;
        }
    }
    Ok(text)
}

fn describe(card: &RateCard) -> String {
    format!(
        "{}: base_fee {} per_km {} per_minute {} minimum {} included_km {} included_minutes {}",
        card.label,
        card.base_fee.0,
        card.per_km.0,
        card.per_minute.0,
        card.minimum.0,
        card.included_km,
        card.included_minutes
    )
}

/// `Day` as `YYYY-MM-DD` (civil-from-days, Howard Hinnant's algorithm).
fn date(day: i32) -> String {
    let z = i64::from(day) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_print_as_dates() {
        assert_eq!(date(0), "1970-01-01");
        assert_eq!(date(20_745), "2026-10-19");
        assert_eq!(date(19_782), "2024-02-29");
        assert_eq!(date(-1), "1969-12-31");
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p whichride-server && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/server
git commit -m "feat(server): owner CLI to show, lock, unlock and revert offers

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---
### Task 13: Docker image and an upstream check command

The image is smoke-tracker's: a static musl binary on distroless, built with
`cargo auditable` so Trivy can scan the crates inside it. Because nothing in production can be
called until the app is on Play (milestone 6 links Play Integrity), the image gets a
`check-routing` command that proves, from inside the container, that TLS, the key and
OpenRouteService work — with a fixed route between two public landmarks, never a user's
location.

**Files:**
- Create: `Dockerfile`, `.dockerignore`
- Modify: root `Cargo.toml` (`[profile.release]`)
- Modify: `crates/server/src/cli.rs`, `main.rs`, `commands.rs`

**Interfaces:**
- Consumes: `Ors` (Task 10), `ServeConfig` (Task 3).
- Produces: CLI `check-routing` → prints `routing ok: <metres> m, <seconds> s` and exits 0, or
  exits 1 with the error kind (`Busy`, `NoRoute`, `Unavailable`). Image entrypoint
  `whichride-server`, default command `serve`, port 8080, user 65532.

- [ ] **Step 1: Write the failing test**

Add to `cli.rs` tests:

```rust
    #[test]
    fn check_routing_parses() {
        assert_eq!(parse(&["check-routing"]).unwrap(), Command::CheckRouting);
    }
```

Run: `cargo test -p whichride-server --lib cli` → FAIL (no variant `CheckRouting`).

- [ ] **Step 2: Implement the command**

`cli.rs`:

```rust
    /// Route between two Berlin landmarks through OpenRouteService, to check the key, TLS and
    /// the network from inside the deployed image. Needs ORS_API_KEY; no database.
    CheckRouting,
```

`main.rs`: `Command::CheckRouting => whichride_server::commands::check_routing().await,`

`commands.rs` (with `use carsharing_core::Coordinate;` and `use crate::ors::Ors;`):

```rust
/// Alexanderplatz → Berlin Hauptbahnhof: public landmarks, not anyone's location.
const CHECK_FROM: (i32, i32) = (525_219_184, 134_132_147);
const CHECK_TO: (i32, i32) = (525_250_839, 133_694_195);

pub async fn check_routing() -> anyhow::Result<()> {
    let get = |name: &str| std::env::var(name).ok();
    let key = get("ORS_API_KEY").filter(|k| !k.trim().is_empty()).context("ORS_API_KEY is not set")?;
    let base = get("ORS_BASE_URL").unwrap_or_else(|| ServeConfig::DEFAULT_ORS_BASE_URL.to_owned());
    let ors = Ors::new(url::Url::parse(&base)?, key.trim().to_owned());
    let from = Coordinate::new(CHECK_FROM.0, CHECK_FROM.1)?;
    let to = Coordinate::new(CHECK_TO.0, CHECK_TO.1)?;
    match ors.route(from, to).await {
        Ok(trip) => {
            println!("routing ok: {} m, {} s", trip.metres, trip.seconds);
            Ok(())
        }
        Err(error) => anyhow::bail!("routing failed: {error:?}"),
    }
}
```

- [ ] **Step 3: Release profile and Docker files**

Root `Cargo.toml`:

```toml
[profile.release]
codegen-units = 1
lto = "thin"
strip = true
```

`Dockerfile`:

```dockerfile
# syntax=docker/dockerfile:1
# A static musl binary on a distroless base: no shell, no package manager.

FROM rust:1-alpine AS build
RUN apk add --no-cache gcc musl-dev
# Embeds the dependency list in the binary, so Trivy can scan the Rust crates inside it.
RUN cargo install cargo-auditable --version 0.7.7 --locked
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo auditable build --release --locked --bin whichride-server \
    && cp target/release/whichride-server /whichride-server

FROM gcr.io/distroless/static-debian13:nonroot
COPY --from=build /whichride-server /usr/local/bin/whichride-server
ENV BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/whichride-server"]
CMD ["serve"]
```

`.dockerignore`:

```
target
spikes
docs
infra
.git
.github
.env
.env.*
**/*.md
```

(`README.md` is not needed in the image; `crates/server/catalogue.json` and the migrations are
compiled into the binary.)

- [ ] **Step 4: Build and run the image**

Check disk space first (`df -h /System/Volumes/Data`; the build needs about 3 GB). Then:

```bash
docker build -t whichride-server:dev .
docker run --rm whichride-server:dev --help
docker run --rm -e ORS_API_KEY whichride-server:dev check-routing   # needs ORS_API_KEY in your shell
docker images whichride-server:dev --format '{{.Size}}'               # expect about 15–25 MB
```

Expected: the help lists `serve`, `migrate`, `show-offer`, `lock-offer`, `unlock-offer`,
`revert-offer`, `check-routing`; `check-routing` prints `routing ok: … m, … s` (roughly
4–6 km). If the owner has no OpenRouteService key yet, ask them to create a free one at
<https://openrouteservice.org/dev/#/signup> (**OWNER**) and export it in their shell; never
paste it into a file.

Then run the server against the local test Postgres with integrity off:

```bash
docker run --rm -p 8080:8080 --add-host=host.docker.internal:host-gateway \
  -e ORS_API_KEY -e PLAY_INTEGRITY=off \
  -e DATABASE_URL=postgres://postgres@host.docker.internal:5432/postgres \
  whichride-server:dev
curl -s localhost:8080/health
curl -s localhost:8080/v1/catalogue | head -c 300
```

Expected: `ok` and the catalogue JSON. Stop the container with Ctrl-C (graceful shutdown logs
`shutting down`).

- [ ] **Step 5: Run all checks and commit**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check`

```bash
git add Dockerfile .dockerignore Cargo.toml crates/server
git commit -m "build: distroless server image and a routing check command

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 14: Infrastructure as code (copied from smoke-tracker)

Decision 40: whichride gets its own Google Cloud project and Neon project, built from a copy
of smoke-tracker's Terraform (bootstrap with state bucket and budget, `cloud-run-app` module,
`prod` root, plan-in-PR workflow). The module gains a generic `secret_env` so the
OpenRouteService key comes from Secret Manager like `DATABASE_URL`.

**Files:**
- Create: `infra/**` (copied), `.github/workflows/infra.yml` (copied)
- Modify after copying: `infra/prod/main.tf`, `infra/prod/project.tf`, `infra/prod/variables.tf`,
  `infra/prod/terraform.tfvars.example`, `infra/modules/cloud-run-app/{variables,main,service}.tf`,
  `infra/modules/cloud-run-app/tests/cloud_run_app.tftest.hcl`, `infra/README.md`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: the image of Task 13 (port 8080, `serve`).
- Produces: Terraform that creates the Cloud Run service `whichride` (max instances 1), the
  secrets `whichride-database-url` and `whichride-ors-api-key`, the deploy account trusted by
  `deploy.yml` on `main`, and the nightly backup of smoke-tracker's module. Outputs used by
  Task 15: the same names as smoke-tracker's (`terraform output` lists them).

- [ ] **Step 1: Copy the tracked files only**

```bash
git -C ../smoke-tracker archive HEAD infra .github/workflows/infra.yml | tar -x -C .
find infra -name '.terraform' -o -name '*.tfstate*' -o -name 'terraform.tfvars' -o -name 'backend.hcl'
```

The `find` must print nothing (only tracked files were copied). Append smoke-tracker's
Terraform lines to `.gitignore`:

```
# Terraform
.terraform/
*.tfstate
*.tfstate.*
infra/bootstrap/terraform.tfvars
infra/bootstrap/backend.hcl
infra/prod/terraform.tfvars
infra/prod/backend.hcl
infra/modules/**/.terraform.lock.hcl
```

- [ ] **Step 2: Rename**

```bash
grep -rln 'smoke-tracker\|smoke_tracker\|Smoke Tracker' infra .github/workflows/infra.yml
```

In every file listed, change the app name to `whichride` (`smoke-tracker` → `whichride`,
`locadani/smoke-tracker` → `locadani/whichride`). In `infra/prod/main.tf` set
`github_repository_id = "1425597390"` (whichride's numeric ID, from
`https://api.github.com/repos/locadani/whichride`). Read each change: the README's commands
must still make sense; smoke-tracker-specific steps (`create-resident`, SQLite, accessibility)
are deleted, not renamed.

- [ ] **Step 3: Extend the module with `secret_env`**

`infra/modules/cloud-run-app/variables.tf`, append:

```hcl
variable "secret_env" {
  description = "Extra environment variables read from Secret Manager, as ENV_NAME => secret ID suffix (the secret is <name>-<suffix>). Each secret is created empty; its value is added by hand."
  type        = map(string)
  default     = {}
}

variable "attach_secret_env" {
  description = "Give the service the secret_env variables. Enable only after every secret has a version: Cloud Run won't start an instance whose secret can't be read."
  type        = bool
  default     = false
}
```

`infra/modules/cloud-run-app/main.tf`, append:

```hcl
# Values are added by hand: a value managed by Terraform would be stored in its state.
resource "google_secret_manager_secret" "extra" {
  for_each = var.secret_env

  project             = var.project_id
  secret_id           = "${var.name}-${each.value}"
  deletion_protection = true

  replication {
    user_managed {
      replicas {
        location = var.region
      }
    }
  }
}

resource "google_secret_manager_secret_iam_member" "runtime_reads_extra" {
  for_each = var.secret_env

  project   = var.project_id
  secret_id = google_secret_manager_secret.extra[each.key].secret_id
  role      = "roles/secretmanager.secretAccessor"
  member    = google_service_account.runtime.member
}
```

`infra/modules/cloud-run-app/service.tf`, inside `containers { … }` after the `DATABASE_URL`
block:

```hcl
      dynamic "env" {
        for_each = var.attach_secret_env ? var.secret_env : {}
        content {
          name = env.key
          value_source {
            secret_key_ref {
              secret  = google_secret_manager_secret.extra[env.key].secret_id
              version = "latest"
            }
          }
        }
      }
```

and extend the service's `depends_on` with `google_secret_manager_secret_iam_member.runtime_reads_extra`.

Add two runs to `tests/cloud_run_app.tftest.hcl`, in the file's existing style (it uses
`command = plan` with mocked providers; copy the variables block of its first run):

```hcl
run "secret_env_creates_secrets_without_attaching_them" {
  command = plan

  variables {
    # … the first run's variables …
    secret_env = { ORS_API_KEY = "ors-api-key" }
  }

  assert {
    condition     = google_secret_manager_secret.extra["ORS_API_KEY"].secret_id == "${var.name}-ors-api-key"
    error_message = "the secret is named <name>-<suffix>"
  }

  assert {
    condition     = length([for e in google_cloud_run_v2_service.app.template[0].containers[0].env : e if e.name == "ORS_API_KEY"]) == 0
    error_message = "secrets are attached only with attach_secret_env"
  }
}

run "attached_secret_env_reaches_the_container" {
  command = plan

  variables {
    # … the first run's variables …
    secret_env        = { ORS_API_KEY = "ors-api-key" }
    attach_secret_env = true
  }

  assert {
    condition     = length([for e in google_cloud_run_v2_service.app.template[0].containers[0].env : e if e.name == "ORS_API_KEY"]) == 1
    error_message = "ORS_API_KEY comes from its secret"
  }
}
```

- [ ] **Step 4: Configure the prod root**

`infra/prod/project.tf`: add `"playintegrity.googleapis.com"` to `local.apis` (the runtime
account decodes integrity tokens; the API must be on in the project that Play Console links in
milestone 6).

`infra/prod/variables.tf`, append:

```hcl
variable "android_package" {
  description = "Android application ID whose Play Integrity verdicts the server accepts."
  type        = string
  default     = "io.github.locadani.whichride"
}
```

`infra/prod/main.tf`: the module call gets

```hcl
  env = {
    RUST_LOG        = "info"
    PLAY_INTEGRITY  = "google"
    ANDROID_PACKAGE = var.android_package
  }

  secret_env = {
    ORS_API_KEY = "ors-api-key"
  }

  # Set to true in a PR once each secret has its first version (infra/README.md).
  attach_database_url = false
  attach_secret_env   = false
```

and loses smoke-tracker's `APP_BASE_URL` / `CLIENT_IP_HEADER` settings and `app_base_url`
local (whichride does not use them).

- [ ] **Step 5: Check**

```bash
cd infra/modules/cloud-run-app && terraform init -backend=false && terraform test && cd -
for dir in infra/bootstrap infra/prod infra/modules/cloud-run-app; do
  terraform -chdir=$dir fmt -check && terraform -chdir=$dir init -backend=false -input=false >/dev/null && terraform -chdir=$dir validate
done
```

Expected: tests pass, `fmt` and `validate` clean. If Terraform is not installed locally, say so
and rely on the copied `infra.yml` workflow in the PR; do not install it without asking (disk).

- [ ] **Step 6: README — first-time setup for whichride**

Rewrite `infra/README.md`'s first-time setup for this project, keeping smoke-tracker's
structure, with these differences:
- A **new** Google Cloud project (e.g. `whichride-prod`), with the billing budget from the
  bootstrap.
- A **new Neon project** (`whichride`, region `aws-eu-central-1`, Postgres 17); its pooled
  connection string goes into the `whichride-database-url` secret.
- The OpenRouteService key goes into `whichride-ors-api-key`:
  `printf %s "$ORS_API_KEY" | gcloud secrets versions add whichride-ors-api-key --data-file=-`.
- After both secrets have versions: a PR setting `attach_database_url = true` and
  `attach_secret_env = true`.
- Check the deployed image (Cloud Shell):
  `curl -s https://<service URL>/health` and `/v1/catalogue`, and
  `docker run --rm -e ORS_API_KEY="$(gcloud secrets versions access latest --secret=whichride-ors-api-key)" <image> check-routing`.
- Owner commands (emergency switch):
  `docker run --rm -e DATABASE_URL="$(gcloud secrets versions access latest --secret=whichride-database-url)" <image> show-offer bolt.corsa`
  (and `lock-offer`, `unlock-offer`, `revert-offer`).
- Play Integrity: nothing to do until milestone 6, when Play Console's *App integrity* page is
  linked to this Cloud project. Until then every write from a real app is refused with
  `integrity_failed`, which is expected.

- [ ] **Step 7: Commit**

```bash
git add infra .github/workflows/infra.yml .gitignore
git commit -m "build(infra): Terraform for whichride's own project, with secret environment variables

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 8: OWNER — first-time setup**

Not done by the agent. Tell the owner the setup in `infra/README.md` is ready to follow after
the PR is merged; it creates billable-account resources (free tier, budget-capped) and needs
their Google and Neon accounts.

---

### Task 15: Deploy workflow and CI additions

**Files:**
- Create: `.github/workflows/deploy.yml` (from smoke-tracker's)
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: the image (Task 13) and the Terraform outputs and GitHub secrets of Task 14
  (`GCP_PROJECT_ID`, `GCP_WIF_PROVIDER`, `GCP_DEPLOY_SA`; variable `GCP_REGION`).
- Produces: on every push to `main` (outside docs), build → Trivy scan → push → `gcloud run
  services update`; skipped with a message while the secrets are not set. A CI job that builds
  the Docker image on pull requests.

- [ ] **Step 1: Copy and adapt the deploy workflow**

```bash
git -C ../smoke-tracker show HEAD:.github/workflows/deploy.yml > .github/workflows/deploy.yml
```

Edit:
- `SERVICE: whichride`; every `smoke-tracker` image name and tag → `whichride`.
- `paths-ignore`: `docs/**`, `**/*.md`, `infra/**`, `spikes/**`.
- The "Refuse to deploy before the database is connected" step also requires `ORS_API_KEY`:

```yaml
      - name: Refuse to deploy before the secrets are connected
        shell: bash
        run: |
          service_json=$(gcloud run services describe "$SERVICE" --region "$REGION" --format=json) \
            || { echo "Could not describe the service; refusing to deploy."; exit 1; }
          for name in DATABASE_URL ORS_API_KEY; do
            echo "$service_json" \
              | jq -e --arg name "$name" '.spec.template.spec.containers[0].env[]? | select(.name == $name)' > /dev/null \
              || { echo "$name is not attached yet (infra/README.md, first-time setup)."; exit 1; }
          done
```

Keep every action pinned to the same commit SHAs as smoke-tracker's file (they were reviewed
there); do not "update" them in this task.

- [ ] **Step 2: Build the image in CI**

Append a job to `.github/workflows/ci.yml`:

```yaml
  docker:
    name: docker image builds
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
        with:
          persist-credentials: false
      - uses: docker/setup-buildx-action@f87e5991a6d7451dcb8d9637bfbc97413f497069 # v4.4.1
      - uses: docker/build-push-action@c3c9e263c25d99ce0380d002d59b67737d91b0dc # v7.4.0
        env:
          DOCKER_BUILD_SUMMARY: false
          DOCKER_BUILD_RECORD_UPLOAD: false
        with:
          context: .
          load: true
          tags: whichride-server:ci
          cache-from: type=gha
          cache-to: type=gha,mode=max
      - run: docker run --rm whichride-server:ci --help
```

- [ ] **Step 3: Check the workflow files**

Run: `ruby -ryaml -e 'ARGV.each { |f| YAML.load_file(f) }' .github/workflows/*.yml && echo ok`
(or `python3 -c 'import sys,yaml;[yaml.safe_load(open(f)) for f in sys.argv[1:]]' .github/workflows/*.yml`
if PyYAML is installed). If `actionlint` is installed, run it too.
Expected: `ok`.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows
git commit -m "ci: deploy to Cloud Run from main; build the image on pull requests

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 16: Documentation

**Files:**
- Modify: `docs/DECISIONS.md`, `docs/plans/ROADMAP.md`, `README.md`,
  `docs/specs/2026-10-19-carsharing-compare-design.md`
- Create: `crates/server/README.md`

- [ ] **Step 1: Decision rows**

Append rows 32–41 to `docs/DECISIONS.md`, dated the day they are committed, from this plan's
"Decisions this plan makes" table, in the log's column order (`# | Date | Area | Decision |
Alternatives considered | Why`); areas: 32–33 Abuse, 34 Prices, 35 Server, 36–39 Crowd prices,
40 Infra, 41 Android. If the owner rejected or changed a decision while reviewing this plan, record what they chose
instead.

- [ ] **Step 2: Roadmap, README, spec**

- `docs/plans/ROADMAP.md`: milestone 3's plan cell → `03-server.md ✅`; under milestone 6 add
  "link Play Integrity to the Cloud project in Play Console".
- `README.md`: status line → milestones 1–3 done; under *Development* add how to run the
  server locally:

```sh
docker run -d --name whichride-pg --rm -p 5432:5432 -e POSTGRES_HOST_AUTH_METHOD=trust postgres:17-alpine
export TEST_POSTGRES_URL=postgres://postgres@localhost:5432/postgres
cargo test --workspace
cp .env.example .env   # fill in ORS_API_KEY; then:
set -a; . ./.env; set +a; cargo run -p whichride-server -- serve
```

- Spec §8: add one sentence after "the server also checks Play Integrity so only the real app
  can use the quota": "(Integrity tokens are checked when an install registers and on every
  write; routing calls need the install's token — decision 33.)"
- `crates/server/README.md`: the API table and error table from this plan (kept in sync with
  the code from now on), the environment variables of `.env.example`, and the owner commands.

- [ ] **Step 3: Final checks**

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
TEST_POSTGRES_URL=postgres://postgres@localhost:5432/postgres cargo test --workspace
cargo deny check   # if installed; CI runs it in security.yml
git status --short   # nothing unexpected; no .env, no tfstate
df -h /System/Volumes/Data
```

Report the disk line to the owner with the clean-up commands they can run when they want the
space back: `cargo clean`, `docker rm -f whichride-pg`, `docker rmi whichride-server:dev postgres:17-alpine`.

- [ ] **Step 4: Commit**

```bash
git add docs README.md crates/server/README.md
git commit -m "docs: decisions 32-41 and milestone 3 done

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

The owner pushes the branch and opens the PR (squash merge, as for milestones 1 and 2).

---

## Not in this milestone

- **Website requests** (Turnstile, per-IP limits, the web's own share of the routing quota,
  spec §8.1): milestone 7. `GET /v1/catalogue` is already public and read-only.
- **Shared request/response types in the core** for the app and website: decided in
  milestone 5, when it is clear whether the app calls the server from Kotlin or from Rust.
- **Registration abuse beyond Play Integrity** (one genuine phone registering many installs):
  new installs weigh 1000 against an established install's up to 10 000 (decision 28), and the
  owner can lock and revert. Play Integrity's device-recall signals can be added if abuse is
  seen.
- **Expiring old pending reports**: they stay pending until another report on the same offer is
  accepted (decision 39). Revisit with real data.
- **Zone fees**: milestone 9.
