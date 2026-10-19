# Milestone 1: Foundation & Pricing Core — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Git repository with a Cargo workspace, secret protection and CI, plus a fully tested,
I/O-free `carsharing-core` crate that prices one trip for every offer of every provider,
including packages and the user's personal adjustments.

**Architecture:** The workspace lives at the repo root, with Rust crates under `crates/`. This
milestone creates only `crates/core` (package `carsharing-core`). Server, Android and web arrive
in later milestones (`docs/plans/ROADMAP.md`). The core is pure: no I/O, no clock, no floating
point. Money is integer cents; percentages are basis points; every rounding is explicit.

**Tech Stack:** Rust (edition 2024, stable toolchain via `rust-toolchain.toml`), serde +
serde_json, thiserror, proptest (tests), GitHub Actions, gitleaks, cargo-deny, Dependabot.

**Spec:** `docs/specs/2026-10-19-carsharing-compare-design.md`. Decisions: `docs/DECISIONS.md`.

## Global Constraints

- Before every commit, `git config user.name` must print `locadani` and `git config user.email`
  must print `61902944+locadani@users.noreply.github.com`. Never put any email address in a
  tracked file.
- Never commit secrets. The gitleaks pre-commit hook (Task 1) must be active for every commit
  after Task 1.
- Every commit message ends with a blank line followed by
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Commit subjects use Conventional Commits, e.g. `feat(core): …`.
- Doc comments explain *what* and *why*; no tutorial-style comments.
- Workspace-wide: `unsafe_code = "forbid"`, clippy warnings are errors in CI
  (`RUSTFLAGS=-D warnings`), `cargo fmt` enforced.
- Money: `Cents` (i64 euro cents). Percentages: basis points 0–10000. Rounding: half up to a
  whole cent. No `f32`/`f64` anywhere in the core.
- Adjustment order (spec §6): override → subscription → percentage discounts → fixed discounts
  and refuel credit → minimum charge.
- `spikes/` is excluded from the workspace and stays as it is.
- Disk: Rust builds are large. At the end of the session, check `df -h /System/Volumes/Data` and
  give the owner the `cargo clean` command to run; never `rm -rf` build folders.

## Review Focus

1. **Start and destination are the same place** (0 m, 0 s). Every offer must still get a price —
   its fees and minimum — without a panic (test in Task 8).
2. **Very long rentals** (thousands of km, several days). No integer overflow in charges or
   rounding (tests in Task 3 and the property test in Task 8).
3. **Discounts bigger than the price.** The total must never go below zero, and the minimum
   charge still applies (tests in Task 7 and Task 8).
4. **A personal override saved without rate cards, or for an offer the catalogue no longer
   has.** It must be ignored, never hide the offer or crash (tests in Task 7 and Task 8).
5. **Two offers with the same total.** The order must be stable (by provider, then offer), so
   the list doesn't jump between screens (test in Task 8).

---

## File Structure

```
whichride/
├─ Cargo.toml                 workspace manifest, shared deps and lints
├─ Cargo.lock                 committed (this is an application)
├─ rust-toolchain.toml        stable + rustfmt + clippy
├─ rustfmt.toml
├─ .gitignore
├─ .gitleaks.toml             gitleaks config (default rules)
├─ .githooks/pre-commit       blocks commits containing secrets
├─ deny.toml                  cargo-deny policy
├─ LICENSE                    MIT
├─ README.md
├─ .github/
│  ├─ dependabot.yml
│  └─ workflows/
│     ├─ ci.yml               fmt, clippy, test
│     └─ security.yml         cargo-deny, gitleaks
├─ docs/                      spec, plans, decisions (already present)
├─ spikes/uniffi-android/     feasibility spike (already present, not in the workspace)
└─ crates/core/
   ├─ Cargo.toml
   └─ src/
      ├─ lib.rs               module list + re-exports
      ├─ money.rs             Cents, Percent
      ├─ usage.rs             Trip, Rounding, usage_charge
      ├─ rate_card.rs         RateCard, CardKind, Charges (+ test fixtures)
      ├─ catalogue.rs         Provider, Offer, ids, Catalogue (+ test fixtures)
      ├─ adjustments.rs       overrides, subscriptions, discounts, refuel credit
      └─ compare.rs           Quote, compare()
```

Tests live in `#[cfg(test)] mod tests` at the bottom of each source file. Shared test data lives
in `#[cfg(test)] pub(crate) mod fixtures` modules in `rate_card.rs` and `catalogue.rs`.

---

### Task 1: Repository and workspace scaffold

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `.gitignore`, `.gitleaks.toml`,
  `.githooks/pre-commit`, `deny.toml`, `LICENSE`
- Create: `crates/core/Cargo.toml`, `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: nothing
- Produces: workspace dependency names (`serde`, `serde_json`, `thiserror`, `proptest`) that
  member crates reference with `.workspace = true`; package `carsharing-core`.

- [ ] **Step 1: Initialise Git and verify the identity**

Run (in `/Users/daniele/Documents/Personal/whichride`):

```bash
git init -b main
git config user.name locadani
git config user.email 61902944+locadani@users.noreply.github.com
git config user.name && git config user.email
```

Expected:
```
locadani
61902944+locadani@users.noreply.github.com
```

- [ ] **Step 2: Write the workspace files**

`Cargo.toml`:

```toml
[workspace]
resolver = "3"
members = ["crates/*"]
exclude = ["spikes"]

[workspace.package]
edition = "2024"
rust-version = "1.94"
license = "MIT"
publish = false
repository = "https://github.com/locadani/whichride"

[workspace.dependencies]
carsharing-core = { path = "crates/core" }
proptest = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
```

`rust-toolchain.toml`:

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

`rustfmt.toml`:

```toml
edition = "2024"
```

`.gitignore`:

```gitignore
# Build output
/target

# Secrets and local config — never commit
.env
.env.*
!.env.example

# OS / editor
.DS_Store
.idea/
.vscode/
```

`.gitleaks.toml`:

```toml
title = "whichride"

[extend]
useDefault = true
```

`.githooks/pre-commit` (then `chmod +x .githooks/pre-commit`):

```sh
#!/bin/sh
# Blocks any commit whose staged changes contain a secret.
# Enable once per clone with: git config core.hooksPath .githooks
set -eu

if ! command -v gitleaks >/dev/null 2>&1; then
    echo "pre-commit: gitleaks is not installed (brew install gitleaks); commit blocked." >&2
    exit 1
fi

exec gitleaks git --pre-commit --staged --redact --no-banner --config .gitleaks.toml
```

`deny.toml`:

```toml
[graph]
all-features = true

[advisories]
yanked = "deny"

[licenses]
confidence-threshold = 0.8
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "Zlib",
]

[bans]
multiple-versions = "warn"
wildcards = "deny"
allow-wildcard-paths = true

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

`LICENSE`: the standard MIT licence text with the line `Copyright (c) 2026 locadani`.

- [ ] **Step 3: Write the core crate skeleton**

`crates/core/Cargo.toml`:

```toml
[package]
name = "carsharing-core"
version = "0.1.0"
description = "Pure pricing rules for comparing carsharing trips"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
repository.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true

[dev-dependencies]
proptest.workspace = true

[lints]
workspace = true
```

`crates/core/src/lib.rs`:

```rust
//! Pricing rules for comparing the cost of one carsharing trip across providers.
//!
//! The crate is pure: no I/O, no clock, no floating point. The Android app, the website and the
//! server all call the same functions, so their numbers can never disagree.
```

- [ ] **Step 4: Build, enable the hook, check for secrets**

Run:

```bash
cargo build --workspace && cargo test --workspace
git config core.hooksPath .githooks
gitleaks dir . --no-banner --redact --config .gitleaks.toml
```

Expected: build succeeds; `0 passed`; gitleaks reports `no leaks found`. `Cargo.lock` now exists.

- [ ] **Step 5: Commit**

The first commit also adds the existing `docs/` and `spikes/` (the spike has its own
`.gitignore` for its build output; check `git status` shows no `target/`, `build/`, `.so` or
`local.properties` files before committing).

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml .gitignore .gitleaks.toml \
  .githooks/pre-commit deny.toml LICENSE crates/core docs spikes
git status --short
git commit -m "chore: workspace scaffold with secret protection" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: CI and security workflows

**Files:**
- Create: `.github/workflows/ci.yml`, `.github/workflows/security.yml`, `.github/dependabot.yml`

**Interfaces:**
- Consumes: the workspace from Task 1
- Produces: checks named `fmt · clippy · test`, `cargo-deny`, `gitleaks` (later milestones add
  jobs to these files)

- [ ] **Step 1: Write the workflows**

`.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings

jobs:
  check:
    name: fmt · clippy · test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets --locked
      - run: cargo test --workspace --locked
```

`.github/workflows/security.yml`:

```yaml
name: Security

on:
  push:
    branches: [main]
  pull_request:
  schedule:
    - cron: "0 6 * * 1" # Mondays 06:00 UTC: catches advisories published after the last push

permissions:
  contents: read

jobs:
  cargo-deny:
    name: cargo-deny
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: EmbarkStudios/cargo-deny-action@v2

  gitleaks:
    name: gitleaks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
        with:
          fetch-depth: 0
      - uses: gitleaks/gitleaks-action@v3
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

`.github/dependabot.yml`:

```yaml
version: 2
updates:
  - package-ecosystem: cargo
    directory: /
    schedule:
      interval: weekly
    groups:
      rust-dependencies:
        patterns: ["*"]
  - package-ecosystem: github-actions
    directory: /
    schedule:
      interval: weekly
```

- [ ] **Step 2: Lint the workflows**

Run: `actionlint .github/workflows/*.yml`
Expected: no output (exit code 0).

- [ ] **Step 3: Commit**

```bash
git add .github
git commit -m "ci: fmt, clippy, test, cargo-deny and gitleaks workflows" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Money and percentages

Integer cents and basis-point percentages with explicit half-up rounding. Everything else builds on these.

**Files:**
- Create: `crates/core/src/money.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: nothing
- Produces: `Cents(pub i64)` with `ZERO`, `times(u32)`, `scaled(u64, u64)`, `at_least_zero()`, `+`, `-`, `Sum`; `Percent` with `ZERO`, `from_basis_points(u16) -> Result<Percent, PercentError>`, `basis_points()`, `off(Cents) -> Cents`; serde: `Cents` as a JSON integer, `Percent` as a JSON integer 0–10000.

- [ ] **Step 1: Register the module and write the failing tests**

Add to `crates/core/src/lib.rs` (keep `pub mod` lines and `pub use` lines each in alphabetical order):

```rust
pub mod money;

pub use money::{Cents, Percent, PercentError};
```

Create `crates/core/src/money.rs` containing only the tests (they go at the bottom of the file; the implementation
is added above them in Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_multiplies_by_units() {
        assert_eq!(Cents(79).times(11), Cents(869));
    }

    #[test]
    fn scaled_rounds_half_up() {
        assert_eq!(Cents(79).scaled(500, 1000), Cents(40)); // 39.5 → 40
        assert_eq!(Cents(79).scaled(499, 1000), Cents(39)); // 39.421 → 39
        assert_eq!(Cents(10).scaled(1, 3), Cents(3)); // 3.33 → 3
    }

    #[test]
    fn scaled_does_not_overflow_for_large_trips() {
        // 3 days in seconds at 50 cents a minute
        assert_eq!(Cents(50).scaled(259_200, 60), Cents(216_000));
    }

    #[test]
    fn at_least_zero_clamps_negative_amounts() {
        assert_eq!((Cents(300) - Cents(500)).at_least_zero(), Cents::ZERO);
        assert_eq!(Cents(300).at_least_zero(), Cents(300));
    }

    #[test]
    fn sum_adds_all_amounts() {
        let total: Cents = [Cents(100), Cents(250), Cents(5)].into_iter().sum();
        assert_eq!(total, Cents(355));
    }

    #[test]
    fn percent_rejects_more_than_one_hundred_percent() {
        assert_eq!(
            Percent::from_basis_points(10_001),
            Err(PercentError(10_001))
        );
        assert!(Percent::from_basis_points(10_000).is_ok());
    }

    #[test]
    fn percent_off_leaves_the_rest_rounded_half_up() {
        let twenty = Percent::from_basis_points(2_000).unwrap();
        assert_eq!(twenty.off(Cents(969)), Cents(775)); // 775.2 → 775
        assert_eq!(twenty.off(Cents(1_000)), Cents(800));
        let fifteen = Percent::from_basis_points(1_500).unwrap();
        assert_eq!(fifteen.off(Cents(3)), Cents(3)); // 2.55 → 3
    }

    #[test]
    fn percent_deserialises_only_valid_values() {
        assert_eq!(
            serde_json::from_str::<Percent>("2500")
                .unwrap()
                .basis_points(),
            2_500
        );
        assert!(serde_json::from_str::<Percent>("12000").is_err());
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p carsharing-core money`
Expected: compile errors naming the items Step 3 defines (`Cents`, `Percent`, `PercentError`).

- [ ] **Step 3: Write the implementation above the tests**

Insert at the top of `crates/core/src/money.rs`:

```rust
//! Money in integer euro cents and percentages in basis points: no floating point anywhere.

use std::iter::Sum;
use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// An amount of money in euro cents.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Cents(pub i64);

impl Cents {
    pub const ZERO: Cents = Cents(0);

    /// This price multiplied by a number of units.
    pub fn times(self, units: u32) -> Cents {
        Cents(self.0 * i64::from(units))
    }

    /// `self × numerator ÷ denominator`, rounded half up to a whole cent.
    pub fn scaled(self, numerator: u64, denominator: u64) -> Cents {
        assert!(denominator > 0, "denominator must be positive");
        let product = i128::from(self.0) * i128::from(numerator);
        let denominator = i128::from(denominator);
        let rounded = (2 * product + denominator).div_euclid(2 * denominator);
        Cents(i64::try_from(rounded).expect("amount fits in i64 cents"))
    }

    /// This amount, or zero if it is negative.
    pub fn at_least_zero(self) -> Cents {
        self.max(Cents::ZERO)
    }
}

impl Add for Cents {
    type Output = Cents;
    fn add(self, rhs: Cents) -> Cents {
        Cents(self.0 + rhs.0)
    }
}

impl Sub for Cents {
    type Output = Cents;
    fn sub(self, rhs: Cents) -> Cents {
        Cents(self.0 - rhs.0)
    }
}

impl Sum for Cents {
    fn sum<I: Iterator<Item = Cents>>(iter: I) -> Cents {
        iter.fold(Cents::ZERO, Add::add)
    }
}

/// A percentage in basis points: 1 % = 100, 100 % = 10 000.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(try_from = "u16", into = "u16")]
pub struct Percent(u16);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("a percentage must be between 0 and 10000 basis points, got {0}")]
pub struct PercentError(pub u16);

impl Percent {
    pub const ZERO: Percent = Percent(0);

    pub fn from_basis_points(basis_points: u16) -> Result<Percent, PercentError> {
        if basis_points > 10_000 {
            return Err(PercentError(basis_points));
        }
        Ok(Percent(basis_points))
    }

    pub fn basis_points(self) -> u16 {
        self.0
    }

    /// The amount left after taking this percentage off, rounded half up to a whole cent.
    pub fn off(self, amount: Cents) -> Cents {
        amount.scaled(u64::from(10_000 - self.0), 10_000)
    }
}

impl TryFrom<u16> for Percent {
    type Error = PercentError;
    fn try_from(basis_points: u16) -> Result<Percent, PercentError> {
        Percent::from_basis_points(basis_points)
    }
}

impl From<Percent> for u16 {
    fn from(percent: Percent) -> u16 {
        percent.0
    }
}
```

- [ ] **Step 4: Run the tests, lints and formatting**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p carsharing-core`
Expected: no warnings; all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/money.rs crates/core/src/lib.rs
git commit -m "feat(core): Cents and Percent newtypes with half-up rounding" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Trip and usage charges

A trip is metres and seconds. `usage_charge` turns an amount into money for both rounding styles providers use, with free included units.

**Files:**
- Create: `crates/core/src/usage.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `Cents::times`, `Cents::scaled` (Task 3)
- Produces: `Trip { metres: u32, seconds: u32 }`; `Rounding::{StartedUnit, Exact}` (JSON `"started_unit"`, `"exact"`); `usage_charge(rate: Cents, amount: u32, unit_size: u32, included_units: u32, rounding: Rounding) -> Cents`; `METRES_PER_KM`, `SECONDS_PER_MINUTE`.

- [ ] **Step 1: Register the module and write the failing tests**

Add to `crates/core/src/lib.rs` (keep `pub mod` lines and `pub use` lines each in alphabetical order):

```rust
pub mod usage;

pub use usage::{Rounding, Trip};
```

Create `crates/core/src/usage.rs` containing only the tests (they go at the bottom of the file; the implementation
is added above them in Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn started_unit_charges_every_started_km() {
        let charge = usage_charge(Cents(79), 10_200, METRES_PER_KM, 0, Rounding::StartedUnit);
        assert_eq!(charge, Cents(11 * 79));
    }

    #[test]
    fn started_unit_charges_exact_multiples_once() {
        let charge = usage_charge(Cents(79), 10_000, METRES_PER_KM, 0, Rounding::StartedUnit);
        assert_eq!(charge, Cents(10 * 79));
    }

    #[test]
    fn exact_charges_proportionally() {
        let charge = usage_charge(Cents(79), 10_200, METRES_PER_KM, 0, Rounding::Exact);
        assert_eq!(charge, Cents(806)); // 805.8 → 806
    }

    #[test]
    fn included_units_are_free() {
        let charge = usage_charge(
            Cents(29),
            250_000,
            METRES_PER_KM,
            200,
            Rounding::StartedUnit,
        );
        assert_eq!(charge, Cents(50 * 29));
        let charge = usage_charge(Cents(29), 250_000, METRES_PER_KM, 200, Rounding::Exact);
        assert_eq!(charge, Cents(50 * 29));
    }

    #[test]
    fn included_units_larger_than_the_trip_cost_nothing() {
        let charge = usage_charge(Cents(29), 5_000, METRES_PER_KM, 40, Rounding::StartedUnit);
        assert_eq!(charge, Cents::ZERO);
        let charge = usage_charge(Cents(29), 5_000, METRES_PER_KM, 40, Rounding::Exact);
        assert_eq!(charge, Cents::ZERO);
    }

    #[test]
    fn zero_amount_costs_nothing() {
        let charge = usage_charge(Cents(11), 0, SECONDS_PER_MINUTE, 0, Rounding::StartedUnit);
        assert_eq!(charge, Cents::ZERO);
    }

    #[test]
    fn trip_round_trips_through_json() {
        let trip: Trip = serde_json::from_str(r#"{"metres":10200,"seconds":1500}"#).unwrap();
        assert_eq!(
            trip,
            Trip {
                metres: 10_200,
                seconds: 1_500
            }
        );
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p carsharing-core usage`
Expected: compile errors naming the items Step 3 defines (`Trip`, `Rounding`, `usage_charge`).

- [ ] **Step 3: Write the implementation above the tests**

Insert at the top of `crates/core/src/usage.rs`:

```rust
//! A trip's distance and duration, and how providers turn them into a charge.

use serde::{Deserialize, Serialize};

use crate::money::Cents;

/// Distance and driving time of one trip, as returned by the routing service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trip {
    pub metres: u32,
    pub seconds: u32,
}

/// How a provider counts units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rounding {
    /// Every started unit is charged in full (10.2 km → 11 km).
    StartedUnit,
    /// Charged proportionally (10.2 km → 10.2 km), rounded to the cent at the end.
    Exact,
}

/// The charge for `amount` (in sub-units, e.g. metres) at `rate` per unit of `unit_size` sub-units
/// (e.g. 1000 metres), after the first `included_units` units, which are free.
pub fn usage_charge(
    rate: Cents,
    amount: u32,
    unit_size: u32,
    included_units: u32,
    rounding: Rounding,
) -> Cents {
    match rounding {
        Rounding::StartedUnit => {
            let units = amount.div_ceil(unit_size);
            rate.times(units.saturating_sub(included_units))
        }
        Rounding::Exact => {
            let included = u64::from(included_units) * u64::from(unit_size);
            let chargeable = u64::from(amount).saturating_sub(included);
            rate.scaled(chargeable, u64::from(unit_size))
        }
    }
}

pub const METRES_PER_KM: u32 = 1_000;
pub const SECONDS_PER_MINUTE: u32 = 60;
```

- [ ] **Step 4: Run the tests, lints and formatting**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p carsharing-core`
Expected: no warnings; all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/usage.rs crates/core/src/lib.rs
git commit -m "feat(core): usage_charge for started-unit and exact rounding" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Rate cards and packages

One tariff of an offer. A package is a rate card whose base fee is the package price and whose included km/minutes cover the allowance; overage uses the per-km/per-minute prices. The `fixtures` module holds example cards that later tasks' tests reuse.

**Files:**
- Create: `crates/core/src/rate_card.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `Cents` (Task 3), `Trip`, `Rounding`, `usage_charge`, `METRES_PER_KM`, `SECONDS_PER_MINUTE` (Task 4)
- Produces: `CardKind::{Standard, Package}`; `RateCard { label, kind, base_fee, per_km, per_minute, included_km, included_minutes, minimum, km_rounding, minute_rounding }`; `RateCard::charges(&Trip) -> Charges`; `Charges { base_fee, distance, time }` with `total()`; test-only `rate_card::fixtures::{per_km_card, per_minute_and_km_card, package_card}`.

- [ ] **Step 1: Register the module and write the failing tests**

Add to `crates/core/src/lib.rs` (keep `pub mod` lines and `pub use` lines each in alphabetical order):

```rust
pub mod rate_card;

pub use rate_card::{CardKind, Charges, RateCard};
```

Create `crates/core/src/rate_card.rs` containing only the tests (they go at the bottom of the file; the implementation
is added above them in Step 3):

```rust
#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;

    /// MILES-like: unlock fee + every started km.
    pub fn per_km_card() -> RateCard {
        RateCard {
            label: "Standard".into(),
            kind: CardKind::Standard,
            base_fee: Cents(100),
            per_km: Cents(79),
            per_minute: Cents::ZERO,
            included_km: 0,
            included_minutes: 0,
            minimum: Cents::ZERO,
            km_rounding: Rounding::StartedUnit,
            minute_rounding: Rounding::StartedUnit,
        }
    }

    /// Bolt-like: per started minute + per started km, with a minimum.
    pub fn per_minute_and_km_card() -> RateCard {
        RateCard {
            label: "Standard".into(),
            kind: CardKind::Standard,
            base_fee: Cents::ZERO,
            per_km: Cents(18),
            per_minute: Cents(11),
            included_km: 0,
            included_minutes: 0,
            minimum: Cents(250),
            km_rounding: Rounding::StartedUnit,
            minute_rounding: Rounding::StartedUnit,
        }
    }

    /// A 3-hour package with 40 km included.
    pub fn package_card() -> RateCard {
        RateCard {
            label: "3 h / 40 km".into(),
            kind: CardKind::Package,
            base_fee: Cents(2_900),
            per_km: Cents(39),
            per_minute: Cents(29),
            included_km: 40,
            included_minutes: 180,
            minimum: Cents::ZERO,
            km_rounding: Rounding::StartedUnit,
            minute_rounding: Rounding::StartedUnit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    const TRIP: Trip = Trip {
        metres: 10_200,
        seconds: 25 * 60,
    };

    #[test]
    fn per_km_card_charges_unlock_and_started_km() {
        let charges = per_km_card().charges(&TRIP);
        assert_eq!(
            charges,
            Charges {
                base_fee: Cents(100),
                distance: Cents(869),
                time: Cents::ZERO
            }
        );
        assert_eq!(charges.total(), Cents(969));
    }

    #[test]
    fn per_minute_and_km_card_charges_both() {
        let charges = per_minute_and_km_card().charges(&TRIP);
        assert_eq!(charges.total(), Cents(11 * 18 + 25 * 11));
    }

    #[test]
    fn minimum_is_not_applied_by_charges() {
        let short = Trip {
            metres: 500,
            seconds: 120,
        };
        assert_eq!(
            per_minute_and_km_card().charges(&short).total(),
            Cents(18 + 22)
        );
    }

    #[test]
    fn package_charges_only_the_price_within_its_allowance() {
        assert_eq!(package_card().charges(&TRIP).total(), Cents(2_900));
    }

    #[test]
    fn package_charges_overage_beyond_its_allowance() {
        let long = Trip {
            metres: 45_000,
            seconds: 200 * 60,
        };
        let charges = package_card().charges(&long);
        assert_eq!(charges.distance, Cents(5 * 39));
        assert_eq!(charges.time, Cents(20 * 29));
    }

    #[test]
    fn rate_card_reads_optional_fields_as_zero() {
        let json = r#"{
            "label": "Standard", "kind": "standard",
            "base_fee": 100, "per_km": 79, "per_minute": 0,
            "km_rounding": "started_unit", "minute_rounding": "exact"
        }"#;
        let card: RateCard = serde_json::from_str(json).unwrap();
        assert_eq!(card.included_km, 0);
        assert_eq!(card.minimum, Cents::ZERO);
        assert_eq!(card.minute_rounding, Rounding::Exact);
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p carsharing-core rate_card`
Expected: compile errors naming the items Step 3 defines (`RateCard`, `CardKind`, `Charges`).

- [ ] **Step 3: Write the implementation above the tests**

Insert at the top of `crates/core/src/rate_card.rs`:

```rust
//! One way an offer can be paid for: the standard tariff or a package.

use serde::{Deserialize, Serialize};

use crate::money::Cents;
use crate::usage::{METRES_PER_KM, Rounding, SECONDS_PER_MINUTE, Trip, usage_charge};

/// Whether a rate card is the pay-as-you-go tariff or a bundle bought before the trip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    Standard,
    Package,
}

/// The prices of one tariff. Every provider's pricing is a subset of these components.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RateCard {
    /// Shown to the user, e.g. "Standard" or "3 h / 40 km".
    pub label: String,
    pub kind: CardKind,
    /// Unlock fee for a standard tariff, the package price for a package.
    pub base_fee: Cents,
    pub per_km: Cents,
    pub per_minute: Cents,
    #[serde(default)]
    pub included_km: u32,
    #[serde(default)]
    pub included_minutes: u32,
    /// Applied last, after personal adjustments (spec §6).
    #[serde(default)]
    pub minimum: Cents,
    pub km_rounding: Rounding,
    pub minute_rounding: Rounding,
}

/// The parts of a trip's price before personal adjustments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Charges {
    pub base_fee: Cents,
    pub distance: Cents,
    pub time: Cents,
}

impl Charges {
    pub fn total(&self) -> Cents {
        self.base_fee + self.distance + self.time
    }
}

impl RateCard {
    /// What this tariff charges for the trip, before adjustments and before the minimum.
    pub fn charges(&self, trip: &Trip) -> Charges {
        Charges {
            base_fee: self.base_fee,
            distance: usage_charge(
                self.per_km,
                trip.metres,
                METRES_PER_KM,
                self.included_km,
                self.km_rounding,
            ),
            time: usage_charge(
                self.per_minute,
                trip.seconds,
                SECONDS_PER_MINUTE,
                self.included_minutes,
                self.minute_rounding,
            ),
        }
    }
}
```

- [ ] **Step 4: Run the tests, lints and formatting**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p carsharing-core`
Expected: no warnings; all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/rate_card.rs crates/core/src/lib.rs
git commit -m "feat(core): RateCard computes charges for standard tariffs and packages" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Provider catalogue

Providers → offers → rate cards, parsed from JSON and validated. This is the format of the versioned provider file (spec §5.1) and of what the server sends to the app.

**Files:**
- Create: `crates/core/src/catalogue.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `Cents` (Task 3), `RateCard`, `rate_card::fixtures` (Task 5)
- Produces: `ProviderId(pub String)`, `OfferId(pub String)` (JSON strings); `Offer { id, name, cards }`; `Provider { id, name, android_package, refuel_credit: Option<Cents>, offers }`; `Catalogue { providers }` with `from_json(&str) -> Result<Catalogue, CatalogueError>`, `validate()`, `provider(&ProviderId) -> Option<&Provider>`; `validate_cards(&OfferId, &[RateCard]) -> Result<(), CatalogueError>`; `CatalogueError`; test-only `catalogue::fixtures::catalogue()` (providers `miles` with offer `miles.s` = per-km card + package card and a €5 refuel credit, and `bolt` with offer `bolt.corsa` = per-minute-and-km card).

- [ ] **Step 1: Register the module and write the failing tests**

Add to `crates/core/src/lib.rs` (keep `pub mod` lines and `pub use` lines each in alphabetical order):

```rust
pub mod catalogue;

pub use catalogue::{Catalogue, CatalogueError, Offer, OfferId, Provider, ProviderId};
```

Create `crates/core/src/catalogue.rs` containing only the tests (they go at the bottom of the file; the implementation
is added above them in Step 3):

```rust
#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;
    use crate::rate_card::fixtures::*;

    pub fn catalogue() -> Catalogue {
        Catalogue {
            providers: vec![
                Provider {
                    id: ProviderId("miles".into()),
                    name: "MILES".into(),
                    android_package: "com.driveby.app".into(),
                    refuel_credit: Some(Cents(500)),
                    offers: vec![Offer {
                        id: OfferId("miles.s".into()),
                        name: "S".into(),
                        cards: vec![per_km_card(), package_card()],
                    }],
                },
                Provider {
                    id: ProviderId("bolt".into()),
                    name: "Bolt Drive".into(),
                    android_package: "ee.mtakso.client".into(),
                    refuel_credit: None,
                    offers: vec![Offer {
                        id: OfferId("bolt.corsa".into()),
                        name: "Opel Corsa".into(),
                        cards: vec![per_minute_and_km_card()],
                    }],
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::catalogue;
    use super::*;

    #[test]
    fn valid_catalogue_round_trips_through_json() {
        let json = serde_json::to_string(&catalogue()).unwrap();
        assert_eq!(Catalogue::from_json(&json).unwrap(), catalogue());
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(matches!(
            Catalogue::from_json("{"),
            Err(CatalogueError::Json(_))
        ));
    }

    #[test]
    fn rejects_duplicate_providers() {
        let mut c = catalogue();
        c.providers.push(c.providers[0].clone());
        assert!(
            matches!(c.validate(), Err(CatalogueError::DuplicateProvider(id)) if id == "miles")
        );
    }

    #[test]
    fn rejects_offer_ids_repeated_across_providers() {
        let mut c = catalogue();
        c.providers[1].offers[0].id = OfferId("miles.s".into());
        assert!(matches!(c.validate(), Err(CatalogueError::DuplicateOffer(id)) if id == "miles.s"));
    }

    #[test]
    fn rejects_provider_without_offers() {
        let mut c = catalogue();
        c.providers[1].offers.clear();
        assert!(matches!(c.validate(), Err(CatalogueError::NoOffers(id)) if id == "bolt"));
    }

    #[test]
    fn rejects_offer_without_cards() {
        let mut c = catalogue();
        c.providers[1].offers[0].cards.clear();
        assert!(matches!(c.validate(), Err(CatalogueError::NoCards(id)) if id == "bolt.corsa"));
    }

    #[test]
    fn rejects_duplicate_card_labels() {
        let mut c = catalogue();
        let card = c.providers[1].offers[0].cards[0].clone();
        c.providers[1].offers[0].cards.push(card);
        assert!(matches!(
            c.validate(),
            Err(CatalogueError::DuplicateCardLabel { .. })
        ));
    }

    #[test]
    fn rejects_negative_prices() {
        let mut c = catalogue();
        c.providers[1].offers[0].cards[0].per_km = Cents(-1);
        assert!(
            matches!(c.validate(), Err(CatalogueError::NegativePrice(id)) if id == "bolt.corsa")
        );
    }

    #[test]
    fn rejects_negative_refuel_credit() {
        let mut c = catalogue();
        c.providers[0].refuel_credit = Some(Cents(-500));
        assert!(
            matches!(c.validate(), Err(CatalogueError::NegativeRefuelCredit(id)) if id == "miles")
        );
    }

    #[test]
    fn finds_a_provider_by_id() {
        let c = catalogue();
        assert_eq!(
            c.provider(&ProviderId("bolt".into())).unwrap().name,
            "Bolt Drive"
        );
        assert!(c.provider(&ProviderId("sixt".into())).is_none());
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p carsharing-core catalogue`
Expected: compile errors naming the items Step 3 defines (`Catalogue`, `CatalogueError`, `Provider`, `Offer`, the id types).

- [ ] **Step 3: Write the implementation above the tests**

Insert at the top of `crates/core/src/catalogue.rs`:

```rust
//! Providers, their offers and the offers' rate cards, loaded from versioned JSON.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::money::Cents;
use crate::rate_card::RateCard;

/// Stable identifier of a provider, e.g. `miles`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderId(pub String);

/// Identifier of an offer, unique across all providers, e.g. `miles.s`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OfferId(pub String);

/// A car model or size class with its tariffs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub id: OfferId,
    pub name: String,
    pub cards: Vec<RateCard>,
}

/// A carsharing company.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provider {
    pub id: ProviderId,
    pub name: String,
    /// Android package used by the "Open <provider>" button.
    pub android_package: String,
    /// Credit given for refuelling or charging during a trip, if the provider offers one.
    #[serde(default)]
    pub refuel_credit: Option<Cents>,
    pub offers: Vec<Offer>,
}

/// Every provider the app compares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Catalogue {
    pub providers: Vec<Provider>,
}

#[derive(Debug, Error)]
pub enum CatalogueError {
    #[error("the catalogue is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("provider `{0}` appears more than once")]
    DuplicateProvider(String),
    #[error("offer `{0}` appears more than once")]
    DuplicateOffer(String),
    #[error("provider `{0}` has no offers")]
    NoOffers(String),
    #[error("offer `{0}` has no rate cards")]
    NoCards(String),
    #[error("offer `{offer}` has two rate cards labelled `{label}`")]
    DuplicateCardLabel { offer: String, label: String },
    #[error("provider `{0}` has a negative refuel credit")]
    NegativeRefuelCredit(String),
    #[error("offer `{0}` has a negative price")]
    NegativePrice(String),
}

impl Catalogue {
    /// Parses and validates a catalogue.
    pub fn from_json(json: &str) -> Result<Catalogue, CatalogueError> {
        let catalogue: Catalogue = serde_json::from_str(json)?;
        catalogue.validate()?;
        Ok(catalogue)
    }

    pub fn validate(&self) -> Result<(), CatalogueError> {
        let mut provider_ids = HashSet::new();
        let mut offer_ids = HashSet::new();
        for provider in &self.providers {
            if !provider_ids.insert(&provider.id) {
                return Err(CatalogueError::DuplicateProvider(provider.id.0.clone()));
            }
            if provider.offers.is_empty() {
                return Err(CatalogueError::NoOffers(provider.id.0.clone()));
            }
            if provider
                .refuel_credit
                .is_some_and(|credit| credit < Cents::ZERO)
            {
                return Err(CatalogueError::NegativeRefuelCredit(provider.id.0.clone()));
            }
            for offer in &provider.offers {
                if !offer_ids.insert(&offer.id) {
                    return Err(CatalogueError::DuplicateOffer(offer.id.0.clone()));
                }
                validate_cards(&offer.id, &offer.cards)?;
            }
        }
        Ok(())
    }

    pub fn provider(&self, id: &ProviderId) -> Option<&Provider> {
        self.providers.iter().find(|provider| &provider.id == id)
    }
}

/// Checks the rate cards of one offer; also used for personal overrides.
pub fn validate_cards(offer: &OfferId, cards: &[RateCard]) -> Result<(), CatalogueError> {
    if cards.is_empty() {
        return Err(CatalogueError::NoCards(offer.0.clone()));
    }
    let mut labels = HashSet::new();
    for card in cards {
        if !labels.insert(&card.label) {
            return Err(CatalogueError::DuplicateCardLabel {
                offer: offer.0.clone(),
                label: card.label.clone(),
            });
        }
        let prices = [card.base_fee, card.per_km, card.per_minute, card.minimum];
        if prices.iter().any(|price| *price < Cents::ZERO) {
            return Err(CatalogueError::NegativePrice(offer.0.clone()));
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run the tests, lints and formatting**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p carsharing-core`
Expected: no warnings; all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/catalogue.rs crates/core/src/lib.rs
git commit -m "feat(core): Catalogue parses and validates providers, offers and rate cards" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Personal adjustments

The user's override, subscription, discounts and refuel credit, applied in the spec's fixed order: override → subscription → percentage discounts → fixed discounts and refuel credit → minimum charge.

**Files:**
- Create: `crates/core/src/adjustments.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `Cents`, `Percent` (Task 3), `Trip` (Task 4), `RateCard`, `CardKind`, `Charges` (Task 5), `Offer`, `OfferId`, `Provider`, `ProviderId`, `catalogue::fixtures` (Task 6)
- Produces: `PriceOverride { offer, cards }`; `Subscription { provider, distance_off, time_off, waive_base_fee, package_off }`; `Discount::Percent { provider, off }` / `Discount::Amount { provider, amount }` (JSON tagged by `"type"`); `Adjustments { overrides, subscriptions, discounts, refuel }` with `Default`, `cards_for(&Offer) -> (&[RateCard], bool)`, `price(&Provider, &RateCard, &Trip) -> Cents`.

- [ ] **Step 1: Register the module and write the failing tests**

Add to `crates/core/src/lib.rs` (keep `pub mod` lines and `pub use` lines each in alphabetical order):

```rust
pub mod adjustments;

pub use adjustments::{Adjustments, Discount, PriceOverride, Subscription};
```

Create `crates/core/src/adjustments.rs` containing only the tests (they go at the bottom of the file; the implementation
is added above them in Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::fixtures::catalogue;
    use crate::rate_card::fixtures::*;

    const TRIP: Trip = Trip {
        metres: 10_200,
        seconds: 25 * 60,
    };

    fn miles() -> Provider {
        catalogue().providers[0].clone()
    }

    fn bolt() -> Provider {
        catalogue().providers[1].clone()
    }

    fn pct(basis_points: u16) -> Percent {
        Percent::from_basis_points(basis_points).unwrap()
    }

    #[test]
    fn no_adjustments_gives_the_rate_card_price() {
        assert_eq!(
            Adjustments::default().price(&miles(), &per_km_card(), &TRIP),
            Cents(969)
        );
    }

    #[test]
    fn override_replaces_the_offer_cards() {
        let offer = &miles().offers[0];
        let mut cheaper = per_km_card();
        cheaper.per_km = Cents(59);
        let adjustments = Adjustments {
            overrides: vec![PriceOverride {
                offer: offer.id.clone(),
                cards: vec![cheaper.clone()],
            }],
            ..Adjustments::default()
        };
        assert_eq!(adjustments.cards_for(offer), (&[cheaper][..], true));
    }

    #[test]
    fn override_for_another_offer_is_ignored() {
        let offer = &miles().offers[0];
        let adjustments = Adjustments {
            overrides: vec![PriceOverride {
                offer: OfferId("unknown".into()),
                cards: vec![],
            }],
            ..Adjustments::default()
        };
        assert_eq!(adjustments.cards_for(offer), (&offer.cards[..], false));
    }

    #[test]
    fn subscription_discounts_standard_components_and_waives_unlock() {
        let adjustments = Adjustments {
            subscriptions: vec![Subscription {
                provider: miles().id,
                distance_off: pct(1_000),
                time_off: Percent::ZERO,
                waive_base_fee: true,
                package_off: Percent::ZERO,
            }],
            ..Adjustments::default()
        };
        // 869 × 0.9 = 782.1 → 782, unlock waived
        assert_eq!(
            adjustments.price(&miles(), &per_km_card(), &TRIP),
            Cents(782)
        );
    }

    #[test]
    fn subscription_discounts_packages_as_a_whole() {
        let adjustments = Adjustments {
            subscriptions: vec![Subscription {
                provider: miles().id,
                distance_off: pct(1_000),
                time_off: Percent::ZERO,
                waive_base_fee: true,
                package_off: pct(500),
            }],
            ..Adjustments::default()
        };
        assert_eq!(
            adjustments.price(&miles(), &package_card(), &TRIP),
            Cents(2_755)
        );
    }

    #[test]
    fn subscription_of_another_provider_is_ignored() {
        let adjustments = Adjustments {
            subscriptions: vec![Subscription {
                provider: bolt().id,
                distance_off: pct(5_000),
                time_off: pct(5_000),
                waive_base_fee: true,
                package_off: pct(5_000),
            }],
            ..Adjustments::default()
        };
        assert_eq!(
            adjustments.price(&miles(), &per_km_card(), &TRIP),
            Cents(969)
        );
    }

    #[test]
    fn percentage_discounts_compound_in_order() {
        let adjustments = Adjustments {
            discounts: vec![
                Discount::Percent {
                    provider: miles().id,
                    off: pct(2_000),
                },
                Discount::Percent {
                    provider: miles().id,
                    off: pct(1_000),
                },
            ],
            ..Adjustments::default()
        };
        // 969 × 0.8 = 775.2 → 775; 775 × 0.9 = 697.5 → 698
        assert_eq!(
            adjustments.price(&miles(), &per_km_card(), &TRIP),
            Cents(698)
        );
    }

    #[test]
    fn percentage_applies_before_fixed_amount() {
        let adjustments = Adjustments {
            discounts: vec![
                Discount::Amount {
                    provider: miles().id,
                    amount: Cents(500),
                },
                Discount::Percent {
                    provider: miles().id,
                    off: pct(5_000),
                },
            ],
            ..Adjustments::default()
        };
        // (969 × 0.5 = 484.5 → 485) − 500 → below zero → 0
        assert_eq!(
            adjustments.price(&miles(), &per_km_card(), &TRIP),
            Cents::ZERO
        );
    }

    #[test]
    fn discounts_larger_than_the_price_stop_at_the_minimum() {
        let adjustments = Adjustments {
            discounts: vec![Discount::Amount {
                provider: bolt().id,
                amount: Cents(10_000),
            }],
            ..Adjustments::default()
        };
        assert_eq!(
            adjustments.price(&bolt(), &per_minute_and_km_card(), &TRIP),
            Cents(250)
        );
    }

    #[test]
    fn refuel_credit_applies_only_when_refuelling_and_offered() {
        let refuelling = Adjustments {
            refuel: true,
            ..Adjustments::default()
        };
        assert_eq!(
            refuelling.price(&miles(), &per_km_card(), &TRIP),
            Cents(469)
        );
        assert_eq!(
            Adjustments::default().price(&miles(), &per_km_card(), &TRIP),
            Cents(969)
        );
        // Bolt offers no credit in the fixture.
        assert_eq!(
            refuelling.price(&bolt(), &per_minute_and_km_card(), &TRIP),
            Cents(473)
        );
    }

    #[test]
    fn minimum_applies_to_short_trips() {
        let short = Trip {
            metres: 500,
            seconds: 120,
        };
        assert_eq!(
            Adjustments::default().price(&bolt(), &per_minute_and_km_card(), &short),
            Cents(250)
        );
    }

    #[test]
    fn adjustments_read_missing_lists_as_empty() {
        let adjustments: Adjustments = serde_json::from_str("{}").unwrap();
        assert_eq!(adjustments, Adjustments::default());
        let json = r#"{"discounts":[{"type":"amount","provider":"miles","amount":500}]}"#;
        let adjustments: Adjustments = serde_json::from_str(json).unwrap();
        assert_eq!(adjustments.discounts.len(), 1);
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p carsharing-core adjustments`
Expected: compile errors naming the items Step 3 defines (`Adjustments`, `Discount`, `PriceOverride`, `Subscription`).

- [ ] **Step 3: Write the implementation above the tests**

Insert at the top of `crates/core/src/adjustments.rs`:

```rust
//! A user's own price changes: overrides, subscriptions, discounts and refuel credits.
//!
//! They are applied in a fixed order (spec §6): override → subscription → percentage discounts →
//! fixed discounts and refuel credit → minimum charge.

use serde::{Deserialize, Serialize};

use crate::catalogue::{Offer, OfferId, Provider, ProviderId};
use crate::money::{Cents, Percent};
use crate::rate_card::{CardKind, Charges, RateCard};
use crate::usage::Trip;

/// The user's own rate cards for an offer, used instead of the shared ones.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceOverride {
    pub offer: OfferId,
    pub cards: Vec<RateCard>,
}

/// A paid membership that lowers specific parts of a provider's price.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub provider: ProviderId,
    /// Off the distance charge of standard tariffs.
    #[serde(default)]
    pub distance_off: Percent,
    /// Off the time charge of standard tariffs.
    #[serde(default)]
    pub time_off: Percent,
    /// The unlock fee of standard tariffs is not charged.
    #[serde(default)]
    pub waive_base_fee: bool,
    /// Off the whole charge of packages.
    #[serde(default)]
    pub package_off: Percent,
}

/// A promotion on one provider. The app passes only discounts that are valid right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Discount {
    Percent { provider: ProviderId, off: Percent },
    Amount { provider: ProviderId, amount: Cents },
}

/// Everything personal that changes a price. Stored on the device only.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Adjustments {
    #[serde(default)]
    pub overrides: Vec<PriceOverride>,
    /// At most one per provider is expected; the first one for a provider applies.
    #[serde(default)]
    pub subscriptions: Vec<Subscription>,
    #[serde(default)]
    pub discounts: Vec<Discount>,
    /// The user will refuel or charge during this trip.
    #[serde(default)]
    pub refuel: bool,
}

impl Adjustments {
    /// The rate cards to price an offer with, and whether they are the user's override.
    /// An override without cards is ignored rather than hiding the offer.
    pub fn cards_for<'a>(&'a self, offer: &'a Offer) -> (&'a [RateCard], bool) {
        let found = self
            .overrides
            .iter()
            .find(|o| o.offer == offer.id && !o.cards.is_empty());
        match found {
            Some(price_override) => (&price_override.cards, true),
            None => (&offer.cards, false),
        }
    }

    /// The final price of a trip with one rate card of `provider`.
    pub fn price(&self, provider: &Provider, card: &RateCard, trip: &Trip) -> Cents {
        let charges = card.charges(trip);
        let mut total = self.after_subscription(&provider.id, card.kind, charges);
        for discount in &self.discounts {
            if let Discount::Percent { provider: id, off } = discount
                && *id == provider.id
            {
                total = off.off(total);
            }
        }
        for discount in &self.discounts {
            if let Discount::Amount {
                provider: id,
                amount,
            } = discount
                && *id == provider.id
            {
                total = total - *amount;
            }
        }
        if self.refuel
            && let Some(credit) = provider.refuel_credit
        {
            total = total - credit;
        }
        total.at_least_zero().max(card.minimum)
    }

    fn after_subscription(&self, provider: &ProviderId, kind: CardKind, charges: Charges) -> Cents {
        let Some(subscription) = self.subscriptions.iter().find(|s| &s.provider == provider) else {
            return charges.total();
        };
        match kind {
            CardKind::Standard => {
                let base_fee = if subscription.waive_base_fee {
                    Cents::ZERO
                } else {
                    charges.base_fee
                };
                base_fee
                    + subscription.distance_off.off(charges.distance)
                    + subscription.time_off.off(charges.time)
            }
            CardKind::Package => subscription.package_off.off(charges.total()),
        }
    }
}
```

- [ ] **Step 4: Run the tests, lints and formatting**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p carsharing-core`
Expected: no warnings; all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/adjustments.rs crates/core/src/lib.rs
git commit -m "feat(core): Adjustments apply overrides, subscriptions and discounts in spec order" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Comparison

The function the app and website call: every offer once, with its cheapest rate card, cheapest first. A property test checks the invariants over random trips and discounts.

**Files:**
- Create: `crates/core/src/compare.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `Adjustments` (Task 7), `Catalogue`, `catalogue::fixtures` (Task 6), `Cents`, `Percent` (Task 3), `Trip` (Task 4)
- Produces: `Quote { provider, offer, offer_name, card_label, total, overridden }`; `compare(&Catalogue, &Trip, &Adjustments) -> Vec<Quote>`.

- [ ] **Step 1: Register the module and write the failing tests**

Add to `crates/core/src/lib.rs` (keep `pub mod` lines and `pub use` lines each in alphabetical order):

```rust
pub mod compare;

pub use compare::{Quote, compare};
```

Create `crates/core/src/compare.rs` containing only the tests (they go at the bottom of the file; the implementation
is added above them in Step 3):

```rust
#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::adjustments::{Discount, PriceOverride};
    use crate::catalogue::fixtures::catalogue;
    use crate::money::Percent;

    const TRIP: Trip = Trip {
        metres: 10_200,
        seconds: 25 * 60,
    };

    #[test]
    fn returns_every_offer_cheapest_first() {
        let quotes = compare(&catalogue(), &TRIP, &Adjustments::default());
        let summary: Vec<_> = quotes
            .iter()
            .map(|q| (q.offer.0.as_str(), q.total))
            .collect();
        assert_eq!(
            summary,
            [("bolt.corsa", Cents(473)), ("miles.s", Cents(969))]
        );
    }

    #[test]
    fn picks_the_cheapest_card_and_names_it() {
        let long = Trip {
            metres: 40_000,
            seconds: 170 * 60,
        };
        let quotes = compare(&catalogue(), &long, &Adjustments::default());
        let miles = quotes.iter().find(|q| q.offer.0 == "miles.s").unwrap();
        assert_eq!(miles.card_label, "3 h / 40 km");
        assert_eq!(miles.total, Cents(2_900));
    }

    #[test]
    fn marks_overridden_offers() {
        let c = catalogue();
        let mut card = c.providers[1].offers[0].cards[0].clone();
        card.per_minute = Cents(9);
        let adjustments = Adjustments {
            overrides: vec![PriceOverride {
                offer: OfferId("bolt.corsa".into()),
                cards: vec![card],
            }],
            ..Adjustments::default()
        };
        let quotes = compare(&c, &TRIP, &adjustments);
        let bolt = quotes.iter().find(|q| q.offer.0 == "bolt.corsa").unwrap();
        assert!(bolt.overridden);
        assert_eq!(bolt.total, Cents(11 * 18 + 25 * 9));
        assert!(
            !quotes
                .iter()
                .find(|q| q.offer.0 == "miles.s")
                .unwrap()
                .overridden
        );
    }

    #[test]
    fn empty_override_falls_back_to_the_shared_cards() {
        let adjustments = Adjustments {
            overrides: vec![PriceOverride {
                offer: OfferId("bolt.corsa".into()),
                cards: vec![],
            }],
            ..Adjustments::default()
        };
        let quotes = compare(&catalogue(), &TRIP, &adjustments);
        let bolt = quotes.iter().find(|q| q.offer.0 == "bolt.corsa").unwrap();
        assert_eq!((bolt.total, bolt.overridden), (Cents(473), false));
    }

    #[test]
    fn zero_length_trip_costs_fees_and_minimums_only() {
        let quotes = compare(
            &catalogue(),
            &Trip {
                metres: 0,
                seconds: 0,
            },
            &Adjustments::default(),
        );
        let summary: Vec<_> = quotes
            .iter()
            .map(|q| (q.offer.0.as_str(), q.total))
            .collect();
        assert_eq!(
            summary,
            [("miles.s", Cents(100)), ("bolt.corsa", Cents(250))]
        );
    }

    #[test]
    fn equal_totals_are_ordered_by_provider_then_offer() {
        let mut c = catalogue();
        let card = c.providers[1].offers[0].cards[0].clone();
        c.providers[0].offers[0].cards = vec![card];
        let quotes = compare(&c, &TRIP, &Adjustments::default());
        let order: Vec<_> = quotes.iter().map(|q| q.provider.0.as_str()).collect();
        assert_eq!(order, ["bolt", "miles"]);
    }

    proptest! {
        #[test]
        fn totals_are_never_below_zero_or_the_minimum_and_are_the_cheapest(
            metres in 0u32..2_000_000,
            seconds in 0u32..1_000_000,
            percent in 0u16..=10_000,
            amount in 0i64..100_000,
            refuel in any::<bool>(),
        ) {
            let c = catalogue();
            let trip = Trip { metres, seconds };
            let adjustments = Adjustments {
                discounts: vec![
                    Discount::Percent { provider: c.providers[0].id.clone(), off: Percent::from_basis_points(percent).unwrap() },
                    Discount::Amount { provider: c.providers[1].id.clone(), amount: Cents(amount) },
                ],
                refuel,
                ..Adjustments::default()
            };
            for quote in compare(&c, &trip, &adjustments) {
                prop_assert!(quote.total >= Cents::ZERO);
                let provider = c.provider(&quote.provider).unwrap();
                let offer = provider.offers.iter().find(|o| o.id == quote.offer).unwrap();
                for card in &offer.cards {
                    prop_assert!(quote.total <= adjustments.price(provider, card, &trip));
                }
                let card = offer.cards.iter().find(|card| card.label == quote.card_label).unwrap();
                prop_assert!(quote.total >= card.minimum);
            }
        }
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p carsharing-core compare`
Expected: compile errors naming the items Step 3 defines (`Quote`, `compare`).

- [ ] **Step 3: Write the implementation above the tests**

Insert at the top of `crates/core/src/compare.rs`:

```rust
//! The comparison the app shows: every offer's cheapest price for one trip.

use serde::{Deserialize, Serialize};

use crate::adjustments::Adjustments;
use crate::catalogue::{Catalogue, OfferId, ProviderId};
use crate::money::Cents;
use crate::usage::Trip;

/// The best price of one offer for a trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quote {
    pub provider: ProviderId,
    pub offer: OfferId,
    pub offer_name: String,
    /// The tariff that gave the lowest price, e.g. "Standard" or "3 h / 40 km".
    pub card_label: String,
    pub total: Cents,
    /// The price comes from the user's own override, not the shared price.
    pub overridden: bool,
}

/// Prices every offer of every provider and returns them cheapest first.
///
/// Each offer appears once, with its cheapest rate card; on a tie the first card wins. Equal
/// totals are ordered by provider and offer id, so the result is deterministic.
pub fn compare(catalogue: &Catalogue, trip: &Trip, adjustments: &Adjustments) -> Vec<Quote> {
    let mut quotes = Vec::new();
    for provider in &catalogue.providers {
        for offer in &provider.offers {
            let (cards, overridden) = adjustments.cards_for(offer);
            let cheapest = cards
                .iter()
                .map(|card| (card, adjustments.price(provider, card, trip)))
                .min_by_key(|(_, total)| *total);
            if let Some((card, total)) = cheapest {
                quotes.push(Quote {
                    provider: provider.id.clone(),
                    offer: offer.id.clone(),
                    offer_name: offer.name.clone(),
                    card_label: card.label.clone(),
                    total,
                    overridden,
                });
            }
        }
    }
    quotes.sort_by(|a, b| (a.total, &a.provider, &a.offer).cmp(&(b.total, &b.provider, &b.offer)));
    quotes
}
```

- [ ] **Step 4: Run the tests, lints and formatting**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p carsharing-core`
Expected: no warnings; all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/compare.rs crates/core/src/lib.rs
git commit -m "feat(core): compare returns each offer's cheapest price, cheapest first" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: README, final verification, roadmap

**Files:**
- Create: `README.md`
- Modify: `docs/plans/ROADMAP.md`

**Interfaces:**
- Consumes: everything above
- Produces: nothing new

- [ ] **Step 1: Write the README**

`README.md`:

~~~markdown
# Carsharing Compare

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
~~~

- [ ] **Step 2: Run the full verification**

Run:

```bash
cargo fmt --all --check
RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
gitleaks git --no-banner --redact --config .gitleaks.toml
grep -rn "f32\|f64" crates/core/src || echo "no floats"
```

Expected: no formatting differences, no warnings, 50 tests pass, `no leaks found`, `no floats`.

- [ ] **Step 3: Mark the milestone done**

In `docs/plans/ROADMAP.md`, change the milestone 1 row's plan cell from
`` `01-foundation-and-pricing-core.md` `` to `` `01-foundation-and-pricing-core.md` ✅ ``.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/plans/ROADMAP.md
git commit -m "docs: README and milestone 1 done" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 5: Hand over to the owner**

The owner created the public repo github.com/locadani/whichride (empty). Pushing is outward-facing: the owner
approves it explicitly. Then:

```bash
git remote add origin https://github.com/locadani/whichride.git && git push -u origin main
```

Then check that the three checks (`fmt · clippy · test`, `cargo-deny`, `gitleaks`) pass on
GitHub, enable push protection and a branch ruleset on `main` as in smoke-tracker, and report
the free disk space with the `cargo clean` command for this workspace.
