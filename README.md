# whichride

Which carsharing service is cheapest for this trip? Share a Google Maps link (or type an
address) and the app compares what the trip costs with MILES, Bolt Drive and Free2move in
Berlin, including your own discounts and subscriptions, then opens the provider's app.

> **Status:** in development. Milestone 1 (pricing core) is done; see the
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

## Licence

MIT
