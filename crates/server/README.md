# whichride-server

The Axum server behind the app: provider catalogue, crowd-sourced prices, install registration,
and a proxy for OpenRouteService (routing, geocoding, autocomplete) and short-link expansion.
Postgres stores installs, prices, reports and votes. Provider data comes from `catalogue.json`,
compiled into the binary (decision 34).

Keep this file in sync with the code.

## API

All bodies are JSON. Errors are `{"error": "<code>"}` with the status below. Every `POST` body is
at most 16 KiB.

| Method and path | Auth | Body → response |
|---|---|---|
| `GET /health` | none | → `200 ok` (plain text) |
| `GET /v1/catalogue` | none | → `200 CatalogueResponse` (offers the owner hid are left out) |
| `POST /v1/installs` | integrity | `{"nonce"}` (16–128 random characters of `[A-Za-z0-9_-]`, new for each attempt) → `201 {"install_id","token"}` |
| `POST /v1/reports` | bearer + integrity | `{"offer","cards"}` or a proposal `{"provider","name","cards"}` → `201 {"report","status"}` for a new report, `200` when it joined an identical pending report or equals the current price (status `"current"`) |
| `POST /v1/reports/{id}/votes` | bearer + integrity | `{"answer":"yes"\|"no"}` → `200 {"status"}` |
| `POST /v1/offers/{id}/confirmations` | bearer + integrity | `{"answer"}` → `204` |
| `POST /v1/route` | bearer | `{"from":Coordinate,"to":Coordinate}` → `200 Trip` |
| `POST /v1/geocode` | bearer | `{"text","near"?}` → `200 {"places":[{"label","at"}]}` |
| `POST /v1/autocomplete` | bearer | `{"text","near"?}` → same as geocode |
| `POST /v1/expand` | bearer | `{"url"}` → `200 Destination` (never `short_link`) |

Headers: `Authorization: Bearer <token>`; `X-Integrity-Token: <Play Integrity standard token>`.

The integrity token's `requestHash` is the lowercase hex SHA-256 of (decision 44):

- registration: `"POST /v1/installs\n"` followed by the exact body bytes;
- reports, votes and confirmations: `"{METHOD} {PATH}\n{install_id}\n"` followed by the exact
  body bytes, where `install_id` is the install's UUID in lowercase hyphenated form.

Known values (from `shasum -a 256`): `POST /v1/installs` with body
`{"nonce":"abcdefghijklmnop"}` →
`c59b7c5ccbba73f1c1f2c526838ae9486eaaf2ac726b320a170aad41b31565c3`;
`POST /v1/offers/miles.s/confirmations` by install `123e4567-e89b-42d3-a456-426614174000`
with body `{"answer":"yes"}` →
`78a4418304e69be8eebd247ac4e93fc97b3fca818a814017cfef1d82be803413`. Each token is accepted
once; a token that is not five base64url parts separated by dots is refused without asking
Google.

Hourly limits (in memory): per install 60 for route, geocode and expand, 600 for autocomplete
and 120 for reports, votes and confirmations together; 600 registrations for all clients
together (no IP addresses are used, decision 45). The write and registration limits are checked
before Google is asked.

An install has at most one pending report per offer and one pending proposal per provider: a
new one replaces the earlier one (decision 46).

| Error code | Status | When |
|---|---|---|
| `bad_request` | 400 | Body is not valid JSON for the endpoint, is over 16 KiB, or a value is out of range (including a malformed nonce, and labels or names with control or bidirectional-formatting characters) |
| `unauthorized` | 401 | Missing or unknown bearer token |
| `integrity_failed` | 403 | Missing, malformed, invalid, stale, already used or non-matching integrity token; Google answered 400 or 404 |
| `not_found` | 404 | Unknown offer or report, or unknown provider in a proposal |
| `method_not_allowed` | 405 | Known path, wrong method |
| `offer_locked` / `report_closed` | 409 | Owner locked or hid the offer / report no longer pending |
| `not_a_short_link` / `no_destination` | 422 | Expand input is not a short link / its target has no destination |
| `no_route` | 422 | OpenRouteService found no route |
| `too_many_requests` | 429 | Per-install limit or the registration limit reached |
| `integrity_unavailable` | 503 | Google's verdict service answered 401, 403, 429 or another error, or its answer could not be read; or the server is remembering too many used tokens |
| `route_service_busy` | 503 | OpenRouteService quota used up, rate limited, down or slow; short-link expansion failed because Google is down, rate limits us (429) or redirects too often |
| `internal` | 500 | Anything else, including a panic (logged without request data) |

Every response has a fresh `x-request-id`; one sent by the client is ignored.

## Environment

Copy `.env.example` to `.env` (git-ignored). The server reads only the environment.

| Variable | Needed | Meaning |
|---|---|---|
| `DATABASE_URL` | every command except `check-routing` | Postgres connection string |
| `BIND_ADDR` | no | Listen address, default `127.0.0.1:8080` |
| `ORS_API_KEY` | `serve`, `check-routing` | OpenRouteService key (free from openrouteservice.org) |
| `ORS_BASE_URL` | no | Override of `https://api.openrouteservice.org` |
| `PLAY_INTEGRITY` | no | `google` (default) or `off`; `off` is refused on Cloud Run |
| `ANDROID_PACKAGE` | `serve`, unless `PLAY_INTEGRITY=off` | Android application ID checked in integrity verdicts |
| `RUST_LOG` | no | Log filter, e.g. `info`; `reqwest`, `hyper` and `hyper_util` are always off |

## Run locally

```sh
docker run -d --name whichride-pg --rm -p 5432:5432 -e POSTGRES_HOST_AUTH_METHOD=trust -e POSTGRES_DB=whichride postgres:17-alpine
export TEST_POSTGRES_URL=postgres://postgres@localhost:5432/postgres   # tests create their own databases
cp .env.example .env   # DATABASE_URL points at the `whichride` database; fill in ORS_API_KEY
set -a; . ./.env; set +a; cargo run -p whichride-server -- serve
```

## Owner commands

Run against the production database from your machine or Cloud Shell, never over HTTP:

```sh
whichride-server serve                  # migrate, seed new offers, serve on BIND_ADDR
whichride-server migrate                # migrate and seed new offers, then exit
whichride-server show-offer <offer>     # price, history and pending reports
whichride-server lock-offer <offer>     # stop reports and votes (abuse only)
whichride-server unlock-offer <offer>   # allow them again
whichride-server revert-offer <offer>   # restore the previous price and its date, close pending reports
whichride-server hide-offer <offer>     # leave the catalogue, refuse crowd input (abuse only)
whichride-server unhide-offer <offer>   # show it again
whichride-server close-report <report>  # close a pending report or proposal, settling nobody
whichride-server check-routing          # route two Berlin landmarks; checks key, TLS, network
```

## Seed catalogue

`catalogue.json` is currently a placeholder; the owner's real prices are pending. Until they are
entered, `serve` and `migrate` refuse to run: they stop, before migrating or writing anything,
when an offer ID contains `pending-owner-prices` or a card has `base_fee`, `per_km` and
`per_minute` all 0. The ignored test `the_real_seed_catalogue_has_owner_prices` fails until then
too (un-ignore it when the prices land). Seeding only adds offers the database does not have yet;
crowd prices are never overwritten.
