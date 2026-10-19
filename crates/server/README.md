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
| `GET /v1/catalogue` | none | → `200 CatalogueResponse` |
| `POST /v1/installs` | integrity | `{}` → `201 {"install_id","token"}` |
| `POST /v1/reports` | bearer + integrity | `{"offer","cards"}` or a proposal `{"provider","name","cards"}` → `201 {"report","status"}` for a new report, `200` when it joined an identical pending report or equals the current price (status `"current"`) |
| `POST /v1/reports/{id}/votes` | bearer + integrity | `{"answer":"yes"\|"no"}` → `200 {"status"}` |
| `POST /v1/offers/{id}/confirmations` | bearer + integrity | `{"answer"}` → `204` |
| `POST /v1/route` | bearer | `{"from":Coordinate,"to":Coordinate}` → `200 Trip` |
| `POST /v1/geocode` | bearer | `{"text","near"?}` → `200 {"places":[{"label","at"}]}` |
| `POST /v1/autocomplete` | bearer | `{"text","near"?}` → same as geocode |
| `POST /v1/expand` | bearer | `{"url"}` → `200 Destination` (never `short_link`) |

Headers: `Authorization: Bearer <token>`; `X-Integrity-Token: <Play Integrity standard token>`.

Per-install hourly limits (in memory): 60 for route, geocode and expand; 600 for autocomplete.

| Error code | Status | When |
|---|---|---|
| `bad_request` | 400 | Body is not valid JSON for the endpoint, or a value is out of range |
| `unauthorized` | 401 | Missing or unknown bearer token |
| `integrity_failed` | 403 | Missing, invalid, stale or non-matching integrity token; Google answered 400 or 404 |
| `not_found` | 404 | Unknown offer or report |
| `offer_locked` / `report_closed` | 409 | Owner locked the offer / report no longer pending |
| `not_a_short_link` / `no_destination` | 422 | Expand input is not a short link / its target has no destination |
| `no_route` | 422 | OpenRouteService found no route |
| `too_many_requests` | 429 | Per-install limit reached |
| `integrity_unavailable` | 503 | Google's verdict service answered 401, 403, 429 or another error, or its answer could not be read |
| `route_service_busy` | 503 | OpenRouteService quota used up, rate limited, down or slow; short-link expansion failed because Google is down or too many redirects |
| `internal` | 500 | Anything else (logged without request data) |

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
| `RUST_LOG` | no | Log filter, e.g. `info` |

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
whichride-server revert-offer <offer>   # restore the previous price, close pending reports
whichride-server check-routing          # route two Berlin landmarks; checks key, TLS, network
```

## Seed catalogue

`catalogue.json` is currently a placeholder; the owner's real prices are pending. The ignored
test `the_real_seed_catalogue_has_owner_prices` fails until they are entered. Seeding only adds
offers the database does not have yet; crowd prices are never overwritten.
