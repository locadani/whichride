# whichride

Which carsharing service is cheapest for this trip? Share a Google Maps link (or type an
address) and the app compares what the trip costs with MILES, Bolt Drive and SIXT share in
Berlin, including your own discounts and subscriptions, then opens the provider's app.

> **Status:** in development. Milestones 1 (pricing core) and 2 (destination input) are done; milestone 3's server is
> done. See the
> [roadmap](docs/plans/ROADMAP.md).

## Design

- [Design spec](docs/specs/2026-10-19-carsharing-compare-design.md)
- [Decision log](docs/DECISIONS.md)
- [Feasibility spike: Rust on Android via UniFFI](spikes/uniffi-android/README.md)

One Rust core (`crates/core`) holds every pricing rule. The Android app (UniFFI), the website
(WebAssembly) and the server (Axum) all call it, so their numbers can never disagree.

## Development

```sh
git config core.hooksPath .githooks   # once per clone: blocks commits containing secrets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Run the server locally ([details](crates/server/README.md)):

```sh
docker run -d --name whichride-pg --rm -p 5432:5432 -e POSTGRES_HOST_AUTH_METHOD=trust -e POSTGRES_DB=whichride postgres:17-alpine
export TEST_POSTGRES_URL=postgres://postgres@localhost:5432/postgres
cargo test --workspace
cp .env.example .env   # fill in ORS_API_KEY; then:
set -a; . ./.env; set +a; cargo run -p whichride-server -- serve
```

Until the owner's real prices replace the placeholder `crates/server/catalogue.json`, `serve` and
`migrate` refuse to start (nothing is written to the database); the tests use their own catalogue.

## Licence

MIT
