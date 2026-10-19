# Milestone 2: Destination Input — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the pure `carsharing-core` crate so it can turn a shared Google Maps link,
`geo:` URI or shared text into a destination, and decide crowd price reports with the voting
rules of spec §7.2–7.5. Airport/zone fees are **not** part of this milestone (owner decision,
2026-10-19); they stay on the roadmap for later.

**Architecture:** Everything stays in `crates/core`: pure functions, no I/O, no clock, no
floating point. Coordinates are integers in 1e-7 degrees (OpenStreetMap's precision) parsed from
text without `f64`. Dates are whole days since 1970-01-01 passed in by the caller. Short links
(`maps.app.goo.gl`) are only *detected* here; expanding them needs the network and belongs to the
server (milestone 3).

**Tech Stack:** Rust (edition 2024), serde + serde_json, thiserror, proptest (tests), plus two new
dependencies: `url` 2 (WHATWG URL parsing, also used by the server later) and `percent-encoding` 2
(decoding path segments). Both are MIT/Apache-2.0 and build for Android and WebAssembly.

**Spec:** `docs/specs/2026-10-19-carsharing-compare-design.md` (§4 core flow, §6 adjustment
order, §7 crowd prices, §9 privacy). Decisions: `docs/DECISIONS.md`. Previous milestone:
`docs/plans/01-foundation-and-pricing-core.md`.

## Global Constraints

- Work on branch `feat/m2-destination-input` (cut from `feat/m1-foundation-and-pricing-core`).
  Never commit to `main`. The owner pushes or approves pushes; the remote is
  `git@github-locadani:locadani/whichride.git` (SSH alias for the `locadani` key). Never use
  `gh` for this repo: it is logged in as a different account.
- Before every commit, `git config user.name` must print `locadani` and `git config user.email`
  must print `61902944+locadani@users.noreply.github.com`. Never put any email address in a
  tracked file.
- Never commit secrets. The gitleaks pre-commit hook (`git config core.hooksPath .githooks`) must
  be active.
- Every commit message ends with a blank line followed by
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Commit subjects use Conventional Commits, e.g. `feat(core): …`.
- Doc comments explain *what* and *why*; no tutorial-style comments.
- Workspace-wide: `unsafe_code = "forbid"`, clippy warnings are errors (`-D warnings`),
  `cargo fmt` enforced.
- Money: `Cents` (i64 euro cents, saturating arithmetic). Percentages: basis points. No
  `f32`/`f64` anywhere in the core — this now includes coordinates.
- Privacy (spec §9): the core never logs, stores or sends coordinates; it only computes with them.
- `spikes/` is excluded from the workspace and stays as it is.
- Tests live in `#[cfg(test)] mod tests` at the bottom of each source file; shared test data in
  `#[cfg(test)] pub(crate) mod fixtures`.
- Disk is nearly full on the owner's Mac: at the end, report `df -h /System/Volumes/Data` and give
  the `cargo clean` command; never `rm -rf` build folders.

## Review Focus

1. **Real shared text, not a bare link.** Android's share sheet sends e.g.
   `"Alexanderplatz\nhttps://maps.app.goo.gl/AbC123"` or `"Check this: https://…"`. The link must
   be found inside the text (tests in Task 2).
2. **Place names with non-ASCII characters** (`K%C3%B6nigstra%C3%9Fe`, `+` as space). They must
   decode to `Königstraße 1`, never to mojibake or a panic (tests in Task 2).
3. **Coordinates at the edges:** more than 7 decimals, negative values, exactly ±90/±180, just
   out of range, non-ASCII digits. They must round half away from zero, accept the limits, and
   reject the rest without a panic (tests in Task 1 and Task 2).
4. **Device clock skew:** a "last asked" or "first seen" date in the future. It must not block the
   confirmation question forever or produce a negative age (tests in Task 3 and Task 4).
5. **Vote stuffing:** one install voting many times, the reporter confirming their own report, an
   install whose weight has dropped to zero. None of these may accept a report on its own (tests
   in Task 3).

---

## File Structure

```
crates/core/
├─ Cargo.toml                 + url, percent-encoding
└─ src/
   ├─ lib.rs                  module list + re-exports (modified in every task)
   ├─ geo.rs                  NEW  Coordinate (1e-7°), parse_pair
   ├─ link.rs                 NEW  Destination, LinkError, parse_destination
   ├─ day.rs                  NEW  Day (days since 1970-01-01)
   ├─ voting.rs               NEW  InstallId, Weight, InstallRecord, Vote, Answer, weight, is_accepted
   └─ confirmation.rs         NEW  should_ask, confirmed_days_ago
Cargo.toml                    + url, percent-encoding in [workspace.dependencies]
docs/DECISIONS.md             rows 27–31
docs/specs/…-design.md        §6 order line updated
docs/plans/ROADMAP.md         milestone 2 ✅, zone fees moved to a later milestone
README.md                     status line
```

---

### Task 1: Coordinates

Integer coordinates parsed from decimal text, without floating point.

**Files:**
- Create: `crates/core/src/geo.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: nothing
- Produces: `Coordinate { lat_e7: i32, lon_e7: i32 }` (Copy, Eq, Hash, serde as
  `{"lat_e7":…,"lon_e7":…}`); `Coordinate::new(i32, i32) -> Result<Coordinate, CoordinateError>`;
  `Coordinate::parse_pair(&str) -> Result<Coordinate, CoordinateError>`;
  `CoordinateError::{NotAPair, OutOfRange}`.

- [ ] **Step 1: Register the module and write the failing tests**

Add to `crates/core/src/lib.rs` (keep `pub mod` lines and `pub use` lines each in alphabetical
order; `cargo fmt` orders the names inside braces):

```rust
pub mod geo;

pub use geo::{Coordinate, CoordinateError};
```

Create `crates/core/src/geo.rs` containing only the tests (the implementation is added above
them in Step 3):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn at(lat_e7: i32, lon_e7: i32) -> Coordinate {
        Coordinate { lat_e7, lon_e7 }
    }

    #[test]
    fn parses_a_decimal_pair() {
        assert_eq!(
            Coordinate::parse_pair("52.5219184,13.4132147"),
            Ok(at(525_219_184, 134_132_147))
        );
    }

    #[test]
    fn allows_spaces_around_the_numbers() {
        assert_eq!(
            Coordinate::parse_pair(" 52.52 , 13.41 "),
            Ok(at(525_200_000, 134_100_000))
        );
    }

    #[test]
    fn parses_whole_degrees_and_signs() {
        assert_eq!(
            Coordinate::parse_pair("52,13"),
            Ok(at(520_000_000, 130_000_000))
        );
        assert_eq!(
            Coordinate::parse_pair("-33.8688,+151.2093"),
            Ok(at(-338_688_000, 1_512_093_000))
        );
    }

    #[test]
    fn rounds_the_eighth_decimal_half_away_from_zero() {
        assert_eq!(
            Coordinate::parse_pair("52.52191849,13.41321475"),
            Ok(at(525_219_185, 134_132_148))
        );
        assert_eq!(
            Coordinate::parse_pair("52.52191844,13.41321474"),
            Ok(at(525_219_184, 134_132_147))
        );
        assert_eq!(Coordinate::parse_pair("-0.00000005,0"), Ok(at(-1, 0)));
    }

    #[test]
    fn accepts_the_limits() {
        assert_eq!(
            Coordinate::parse_pair("90,180"),
            Ok(at(900_000_000, 1_800_000_000))
        );
        assert_eq!(
            Coordinate::parse_pair("-90,-180"),
            Ok(at(-900_000_000, -1_800_000_000))
        );
    }

    #[test]
    fn rejects_values_out_of_range() {
        for text in ["90.0000001,0", "0,180.0000001", "91,13", "999,0"] {
            assert_eq!(
                Coordinate::parse_pair(text),
                Err(CoordinateError::OutOfRange),
                "{text}"
            );
        }
        assert_eq!(
            Coordinate::new(0, 1_800_000_001),
            Err(CoordinateError::OutOfRange)
        );
    }

    #[test]
    fn rejects_text_that_is_not_a_pair() {
        for text in [
            "",
            "Alexanderplatz",
            "52.52",
            "52.,13",
            ".5,13",
            "52.5,13.4,17z",
            "1e5,2",
            "-,1",
            "1000,0",
            "52.5;13.4",
            "٥٢,13",
        ] {
            assert_eq!(
                Coordinate::parse_pair(text),
                Err(CoordinateError::NotAPair),
                "{text}"
            );
        }
    }

    #[test]
    fn serialises_as_two_integers() {
        assert_eq!(
            serde_json::to_string(&at(1, -2)).unwrap(),
            r#"{"lat_e7":1,"lon_e7":-2}"#
        );
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p carsharing-core geo`
Expected: compile errors naming the items Step 3 defines (`Coordinate`, `CoordinateError`).

- [ ] **Step 3: Write the implementation above the tests**

Insert at the top of `crates/core/src/geo.rs`:

```rust
//! Points on Earth in integer 1e-7 degrees (about 1 cm), parsed from text without floating point.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Units of 1e-7 degrees in one degree.
const E7: i64 = 10_000_000;

/// A point in 1e-7 degrees, the precision OpenStreetMap uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Coordinate {
    pub lat_e7: i32,
    pub lon_e7: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoordinateError {
    #[error("not a latitude,longitude pair")]
    NotAPair,
    #[error("latitude must be within ±90° and longitude within ±180°")]
    OutOfRange,
}

impl Coordinate {
    /// A coordinate, checking that it lies on Earth.
    pub fn new(lat_e7: i32, lon_e7: i32) -> Result<Coordinate, CoordinateError> {
        if i64::from(lat_e7).abs() > 90 * E7 || i64::from(lon_e7).abs() > 180 * E7 {
            return Err(CoordinateError::OutOfRange);
        }
        Ok(Coordinate { lat_e7, lon_e7 })
    }

    /// Parses `"52.5219184,13.4132147"`; spaces around each number are allowed.
    pub fn parse_pair(text: &str) -> Result<Coordinate, CoordinateError> {
        let (lat, lon) = text.split_once(',').ok_or(CoordinateError::NotAPair)?;
        let lat = parse_degrees(lat.trim()).ok_or(CoordinateError::NotAPair)?;
        let lon = parse_degrees(lon.trim()).ok_or(CoordinateError::NotAPair)?;
        let lat = i32::try_from(lat).map_err(|_| CoordinateError::OutOfRange)?;
        let lon = i32::try_from(lon).map_err(|_| CoordinateError::OutOfRange)?;
        Coordinate::new(lat, lon)
    }
}

/// Parses a plain decimal number of degrees (optional sign, at most three whole digits) into
/// 1e-7 degrees, rounding half away from zero at the eighth decimal.
fn parse_degrees(text: &str) -> Option<i64> {
    let (negative, digits) = match text.as_bytes().first()? {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };
    let (whole, fraction) = match digits.split_once('.') {
        Some((_, "")) => return None,
        Some((whole, fraction)) => (whole, fraction),
        None => (digits, ""),
    };
    let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if whole.is_empty() || whole.len() > 3 || !all_digits(whole) || !all_digits(fraction) {
        return None;
    }
    let fraction = fraction.as_bytes();
    let mut value: i64 = whole.parse().ok()?;
    for index in 0..7 {
        let digit = fraction.get(index).map_or(0, |byte| i64::from(byte - b'0'));
        value = value * 10 + digit;
    }
    if fraction.get(7).is_some_and(|byte| *byte >= b'5') {
        value += 1;
    }
    Some(if negative { -value } else { value })
}
```

- [ ] **Step 4: Run the tests, lints and formatting**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p carsharing-core`
Expected: no warnings; all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/geo.rs crates/core/src/lib.rs
git commit -m "feat(core): integer coordinates parsed without floats" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Google Maps links and `geo:` URIs

Finds the link inside shared text and turns it into exact coordinates, a place name to geocode,
or a short link for the server to expand.

**Files:**
- Modify: `Cargo.toml` (workspace dependencies), `crates/core/Cargo.toml`
- Create: `crates/core/src/link.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `Coordinate`, `Coordinate::parse_pair`, `CoordinateError` (Task 1)
- Produces: `Destination::{Coordinates { at: Coordinate, label: Option<String> }, Place { query:
  String, near: Option<Coordinate> }, ShortLink { url: String }}` (JSON tagged by `"type"`:
  `"coordinates"`, `"place"`, `"short_link"`); `LinkError::{NoLink, NotGoogleMaps, NoDestination,
  InvalidCoordinates, TooLong}`; `parse_destination(&str) -> Result<Destination, LinkError>`;
  `MAX_INPUT_BYTES`.

Supported forms (each has a test):

| Input | Result |
|---|---|
| `…/maps/place/<name>/@<viewport>/data=…!3d<lat>!4d<lon>…` | `Coordinates` (pin from `data`), label = name |
| `…/maps/place/<name>/@<viewport>` | `Place` (name), near = viewport |
| `…/maps/search/<text>` and `?api=1&query=<text>` | `Place` or `Coordinates` |
| `…/maps/dir/<start>/<destination>/…` and `?api=1&destination=…` | the destination |
| `…/maps/@<lat>,<lon>,<zoom>z` | `Coordinates` |
| `maps.google.*/?q=…`, `?q=loc:…`, `?daddr=…`, `?ll=…` | `Coordinates`, or `Place` near `ll` |
| `geo:<lat>,<lon>[;u=…][?q=<text or lat,lon(label)>]` | `Coordinates` or `Place`; `geo:0,0` = no position |
| `https://maps.app.goo.gl/…`, `https://goo.gl/maps/…` | `ShortLink` |

- [ ] **Step 1: Add the dependencies**

In the root `Cargo.toml`, `[workspace.dependencies]` (keep alphabetical order):

```toml
percent-encoding = "2"
url = "2"
```

In `crates/core/Cargo.toml`, `[dependencies]` (keep alphabetical order):

```toml
percent-encoding.workspace = true
url.workspace = true
```

Run: `cargo build -p carsharing-core && cargo deny check licenses bans sources`
Expected: build succeeds; `licenses ok, bans ok, sources ok` (`url` brings in ICU crates under
`Unicode-3.0`, which `deny.toml` already allows).

- [ ] **Step 2: Register the module and write the failing tests**

Add to `crates/core/src/lib.rs`:

```rust
pub mod link;

pub use link::{Destination, LinkError, parse_destination};
```

Create `crates/core/src/link.rs` containing only the tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn at(lat_e7: i32, lon_e7: i32) -> Coordinate {
        Coordinate { lat_e7, lon_e7 }
    }

    fn coordinates(lat_e7: i32, lon_e7: i32, label: Option<&str>) -> Destination {
        Destination::Coordinates {
            at: at(lat_e7, lon_e7),
            label: label.map(str::to_owned),
        }
    }

    fn place(query: &str, near: Option<Coordinate>) -> Destination {
        Destination::Place {
            query: query.into(),
            near,
        }
    }

    fn parse(input: &str) -> Result<Destination, LinkError> {
        parse_destination(input)
    }

    #[test]
    fn place_link_uses_the_pin_from_data_not_the_viewport() {
        let link = "https://www.google.com/maps/place/Alexanderplatz,+10178+Berlin/@52.52,13.41,15z/data=!3m1!4b1!4m6!3m5!1s0x47a84e1f1d4d3f0b:0x3b2d0f4c1c1e2a0!8m2!3d52.5219184!4d13.4132147!16zL20vMDFsNnQ1?entry=ttu";
        assert_eq!(
            parse(link),
            Ok(coordinates(
                525_219_184,
                134_132_147,
                Some("Alexanderplatz, 10178 Berlin")
            ))
        );
    }

    #[test]
    fn place_link_without_pin_is_geocoded_near_the_viewport() {
        let link = "https://www.google.com/maps/place/Brandenburger+Tor/@52.5162746,13.3777041,17z";
        assert_eq!(
            parse(link),
            Ok(place(
                "Brandenburger Tor",
                Some(at(525_162_746, 133_777_041))
            ))
        );
    }

    #[test]
    fn place_names_are_percent_decoded() {
        let link = "https://www.google.de/maps/place/K%C3%B6nigstra%C3%9Fe+1/@52.4,13.1,15z";
        assert_eq!(
            parse(link),
            Ok(place("Königstraße 1", Some(at(524_000_000, 131_000_000))))
        );
    }

    #[test]
    fn query_with_coordinates() {
        assert_eq!(
            parse("https://maps.google.com/?q=52.5219,13.4132"),
            Ok(coordinates(525_219_000, 134_132_000, None))
        );
        assert_eq!(
            parse("https://maps.google.com/maps?q=loc:52.5,13.4"),
            Ok(coordinates(525_000_000, 134_000_000, None))
        );
    }

    #[test]
    fn query_with_a_place_name() {
        assert_eq!(
            parse("https://maps.google.com/maps?q=Alexanderplatz+Berlin"),
            Ok(place("Alexanderplatz Berlin", None))
        );
        assert_eq!(
            parse("https://www.google.com/maps/search/?api=1&query=Berlin%20Hbf"),
            Ok(place("Berlin Hbf", None))
        );
        assert_eq!(
            parse("https://www.google.com/maps/search/Berlin+Hbf/@52.52,13.37,15z"),
            Ok(place("Berlin Hbf", Some(at(525_200_000, 133_700_000))))
        );
    }

    #[test]
    fn query_place_is_searched_near_ll() {
        assert_eq!(
            parse("https://maps.google.com/?ll=52.5,13.4&q=Pizza"),
            Ok(place("Pizza", Some(at(525_000_000, 134_000_000))))
        );
    }

    #[test]
    fn directions_use_the_destination() {
        assert_eq!(
            parse(
                "https://www.google.com/maps/dir/?api=1&origin=Alexanderplatz&destination=52.5,13.4"
            ),
            Ok(coordinates(525_000_000, 134_000_000, None))
        );
        assert_eq!(
            parse(
                "https://www.google.com/maps/dir/Alexanderplatz/Berlin+Hauptbahnhof/@52.52,13.39,14z/data=!4m2!4m1!3e0"
            ),
            Ok(place(
                "Berlin Hauptbahnhof",
                Some(at(525_200_000, 133_900_000))
            ))
        );
        assert_eq!(
            parse("https://www.google.com/maps/dir/Alexanderplatz/Berlin+Hauptbahnhof/"),
            Ok(place("Berlin Hauptbahnhof", None))
        );
        assert_eq!(
            parse("https://maps.google.com/maps?saddr=Alexanderplatz&daddr=Berlin+Hbf"),
            Ok(place("Berlin Hbf", None))
        );
    }

    #[test]
    fn directions_without_a_destination_are_rejected() {
        assert_eq!(
            parse("https://www.google.com/maps/dir/Alexanderplatz//@52.52,13.39,14z"),
            Err(LinkError::NoDestination)
        );
    }

    #[test]
    fn map_view_without_a_place_uses_its_centre() {
        assert_eq!(
            parse("https://www.google.com/maps/@52.5219,13.4132,15z"),
            Ok(coordinates(525_219_000, 134_132_000, None))
        );
        assert_eq!(
            parse("https://www.google.co.uk/maps/@51.5,-0.12,15z"),
            Ok(coordinates(515_000_000, -1_200_000, None))
        );
        assert_eq!(
            parse("https://www.google.com/maps"),
            Err(LinkError::NoDestination)
        );
    }

    #[test]
    fn coordinates_out_of_range_are_rejected() {
        assert_eq!(
            parse("https://maps.google.com/?q=95.0,13.4"),
            Err(LinkError::InvalidCoordinates)
        );
        assert_eq!(
            parse("geo:52.5,200"),
            Err(LinkError::InvalidCoordinates)
        );
    }

    #[test]
    fn geo_uris() {
        assert_eq!(
            parse("geo:52.5219,13.4132"),
            Ok(coordinates(525_219_000, 134_132_000, None))
        );
        assert_eq!(
            parse("geo:52.5219,13.4132;u=35"),
            Ok(coordinates(525_219_000, 134_132_000, None))
        );
        assert_eq!(
            parse("GEO:52.5,13.4"),
            Ok(coordinates(525_000_000, 134_000_000, None))
        );
        assert_eq!(
            parse("geo:0,0?q=52.5219,13.4132(Alexanderplatz)"),
            Ok(coordinates(525_219_000, 134_132_000, Some("Alexanderplatz")))
        );
        assert_eq!(
            parse("geo:0,0?q=Alexanderplatz+Berlin"),
            Ok(place("Alexanderplatz Berlin", None))
        );
        assert_eq!(
            parse("geo:52.5,13.4?q=Pizza"),
            Ok(place("Pizza", Some(at(525_000_000, 134_000_000))))
        );
        assert_eq!(
            parse("geo:0,0?q=Caf%C3%A9+(Mitte)"),
            Ok(place("Café (Mitte)", None))
        );
    }

    #[test]
    fn geo_without_a_position_or_query_is_rejected() {
        assert_eq!(parse("geo:0,0"), Err(LinkError::NoDestination));
        assert_eq!(parse("geo:0,0?q="), Err(LinkError::NoDestination));
        assert_eq!(parse("geo:somewhere"), Err(LinkError::NoDestination));
    }

    #[test]
    fn short_links_are_left_for_the_server() {
        assert_eq!(
            parse("https://maps.app.goo.gl/AbC123"),
            Ok(Destination::ShortLink {
                url: "https://maps.app.goo.gl/AbC123".into()
            })
        );
        assert_eq!(
            parse("https://maps.app.goo.gl/AbC123?g_st=ic"),
            Ok(Destination::ShortLink {
                url: "https://maps.app.goo.gl/AbC123?g_st=ic".into()
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
    fn finds_the_link_inside_shared_text() {
        let expected = Ok(Destination::ShortLink {
            url: "https://maps.app.goo.gl/AbC123".into(),
        });
        assert_eq!(
            parse("Alexanderplatz\nhttps://maps.app.goo.gl/AbC123"),
            expected
        );
        assert_eq!(
            parse("Check this out: https://maps.app.goo.gl/AbC123 "),
            expected
        );
    }

    #[test]
    fn text_without_a_link_is_rejected() {
        assert_eq!(parse(""), Err(LinkError::NoLink));
        assert_eq!(parse("Alexanderplatz"), Err(LinkError::NoLink));
        assert_eq!(
            parse("ftp://maps.google.com/?q=1,2"),
            Err(LinkError::NoLink)
        );
    }

    #[test]
    fn links_that_are_not_google_maps_are_rejected() {
        for link in [
            "https://example.com/maps/place/X",
            "https://www.google.com/search?q=berlin",
            "https://google.evil.com/maps/@52.5,13.4,15z",
            "https://goo.gl/abc",
            "https://",
        ] {
            assert_eq!(parse(link), Err(LinkError::NotGoogleMaps), "{link}");
        }
    }

    #[test]
    fn overlong_input_is_rejected() {
        let input = format!("https://maps.google.com/?q={}", "a".repeat(MAX_INPUT_BYTES));
        assert_eq!(parse(&input), Err(LinkError::TooLong));
    }

    #[test]
    fn destination_json_is_tagged() {
        assert_eq!(
            serde_json::to_value(Destination::ShortLink { url: "u".into() }).unwrap(),
            serde_json::json!({"type": "short_link", "url": "u"})
        );
    }
}
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -p carsharing-core link`
Expected: compile errors naming the items Step 4 defines (`Destination`, `LinkError`,
`parse_destination`, `MAX_INPUT_BYTES`).

- [ ] **Step 4: Write the implementation above the tests**

Insert at the top of `crates/core/src/link.rs`:

```rust
//! Turns what the user shares or pastes (a Google Maps link, a `geo:` URI, or text containing
//! one) into a destination the app can route to.

use percent_encoding::percent_decode_str;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use crate::geo::{Coordinate, CoordinateError};

/// Longest input accepted. Real shares are a few hundred bytes; this bounds the work done on
/// pasted junk.
pub const MAX_INPUT_BYTES: usize = 4_096;

/// Where the user wants to go, as far as the link alone can tell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Destination {
    /// An exact position, with the place name when the link has one.
    Coordinates {
        at: Coordinate,
        label: Option<String>,
    },
    /// A name or address to geocode, biased towards the map position the link showed.
    Place {
        query: String,
        near: Option<Coordinate>,
    },
    /// A short link that the server must expand (follow its redirect) before parsing again.
    ShortLink { url: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LinkError {
    #[error("the text contains no link")]
    NoLink,
    #[error("the link is not a Google Maps link")]
    NotGoogleMaps,
    #[error("the link has no destination")]
    NoDestination,
    #[error("the link's coordinates are out of range")]
    InvalidCoordinates,
    #[error("the text is longer than {MAX_INPUT_BYTES} bytes")]
    TooLong,
}

/// Parses shared or pasted text containing a Google Maps link or a `geo:` URI.
pub fn parse_destination(input: &str) -> Result<Destination, LinkError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(LinkError::TooLong);
    }
    let link = find_link(input).ok_or(LinkError::NoLink)?;
    if starts_with_ignore_case(link, "geo:") {
        return parse_geo(&link["geo:".len()..]);
    }
    let url = Url::parse(link).map_err(|_| LinkError::NotGoogleMaps)?;
    if is_short_link(&url) {
        return Ok(Destination::ShortLink { url: url.into() });
    }
    if !is_google_maps(&url) {
        return Err(LinkError::NotGoogleMaps);
    }
    parse_google_maps(&url)
}

/// The first whitespace-separated word that starts like a web link or a `geo:` URI.
fn find_link(input: &str) -> Option<&str> {
    input.split_whitespace().find(|word| {
        ["https://", "http://", "geo:"]
            .iter()
            .any(|scheme| starts_with_ignore_case(word, scheme))
    })
}

fn starts_with_ignore_case(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

fn is_short_link(url: &Url) -> bool {
    match url.host_str() {
        Some("maps.app.goo.gl") => true,
        Some("goo.gl") => url.path().starts_with("/maps/"),
        _ => false,
    }
}

/// `google.<tld>/maps…`, `www.google.<tld>/maps…` or `maps.google.<tld>/…`, where `<tld>` is
/// `com`, a country code, or `co.<cc>` / `com.<cc>`.
fn is_google_maps(url: &Url) -> bool {
    let host = url.host_str().unwrap_or_default();
    let (maps_host, domain) = match host.strip_prefix("maps.") {
        Some(domain) => (true, domain),
        None => (false, host.strip_prefix("www.").unwrap_or(host)),
    };
    let Some(suffix) = domain.strip_prefix("google.") else {
        return false;
    };
    let country_code = |label: &str| label.len() == 2 && label.bytes().all(|b| b.is_ascii_lowercase());
    let google_suffix = match suffix.split_once('.') {
        None => suffix == "com" || country_code(suffix),
        Some((second_level, country)) => {
            matches!(second_level, "co" | "com") && country_code(country)
        }
    };
    let path = url.path();
    google_suffix && (maps_host || path == "/maps" || path.starts_with("/maps/"))
}

fn parse_google_maps(url: &Url) -> Result<Destination, LinkError> {
    let mut segments: Vec<String> = url
        .path_segments()
        .map(|segments| segments.map(decode_component).collect())
        .unwrap_or_default();
    // A trailing slash leaves one empty segment that is not a waypoint.
    if segments.last().is_some_and(String::is_empty) {
        segments.pop();
    }
    let viewport = segments
        .iter()
        .find_map(|segment| segment.strip_prefix('@'))
        .and_then(viewport_centre);
    let near = viewport.or_else(|| {
        query_value(url, "ll").and_then(|ll| Coordinate::parse_pair(&ll).ok())
    });

    for key in ["destination", "query", "q", "daddr"] {
        if let Some(text) = query_value(url, key) {
            return from_text(&text, near);
        }
    }

    let Some(kind) = segments
        .iter()
        .position(|segment| matches!(segment.as_str(), "place" | "search" | "dir"))
    else {
        return near
            .map(|at| Destination::Coordinates { at, label: None })
            .ok_or(LinkError::NoDestination);
    };
    let names: Vec<&str> = segments[kind + 1..]
        .iter()
        .map(String::as_str)
        .take_while(|segment| !segment.starts_with('@') && !segment.starts_with("data="))
        .collect();
    let name = if segments[kind] == "dir" {
        names.last()
    } else {
        names.first()
    };
    let name = name.map(|name| name.trim()).filter(|name| !name.is_empty());
    let pin = segments
        .iter()
        .find_map(|segment| segment.strip_prefix("data="))
        .and_then(pinned_coordinates);
    match (name, pin) {
        (name, Some(at)) if segments[kind] == "place" => Ok(Destination::Coordinates {
            at,
            label: name.map(str::to_owned),
        }),
        (Some(name), _) => from_text(name, near),
        (None, _) => Err(LinkError::NoDestination),
    }
}

/// `geo:` URIs (RFC 5870 plus Android's `?q=` extension). `geo:0,0` means "no position".
fn parse_geo(rest: &str) -> Result<Destination, LinkError> {
    let (position, query) = rest.split_once('?').unwrap_or((rest, ""));
    let position = position.split(';').next().unwrap_or_default();
    let at = match Coordinate::parse_pair(position) {
        Ok(at) => Some(at).filter(|at| at.lat_e7 != 0 || at.lon_e7 != 0),
        Err(CoordinateError::OutOfRange) => return Err(LinkError::InvalidCoordinates),
        Err(CoordinateError::NotAPair) => None,
    };
    let text = query
        .split('&')
        .find_map(|pair| pair.strip_prefix("q="))
        .map(decode_component)
        .filter(|text| !text.trim().is_empty());
    match text {
        Some(text) => from_text(&text, at),
        None => at
            .map(|at| Destination::Coordinates { at, label: None })
            .ok_or(LinkError::NoDestination),
    }
}

/// A search text: coordinates (optionally `lat,lon(label)` or `loc:lat,lon`) or a place name.
fn from_text(text: &str, near: Option<Coordinate>) -> Result<Destination, LinkError> {
    let text = text.trim();
    let text = text.strip_prefix("loc:").unwrap_or(text).trim();
    if text.is_empty() {
        return Err(LinkError::NoDestination);
    }
    let (position, label) = match text.strip_suffix(')').and_then(|t| t.split_once('(')) {
        Some((position, label)) => (
            position.trim(),
            Some(label.trim().to_owned()).filter(|label| !label.is_empty()),
        ),
        None => (text, None),
    };
    match Coordinate::parse_pair(position) {
        Ok(at) => Ok(Destination::Coordinates { at, label }),
        Err(CoordinateError::OutOfRange) => Err(LinkError::InvalidCoordinates),
        Err(CoordinateError::NotAPair) => Ok(Destination::Place {
            query: text.to_owned(),
            near,
        }),
    }
}

/// The centre of `@52.52,13.41,15z`-style map views.
fn viewport_centre(view: &str) -> Option<Coordinate> {
    let mut parts = view.splitn(3, ',');
    let (lat, lon) = (parts.next()?, parts.next()?);
    Coordinate::parse_pair(&format!("{lat},{lon}")).ok()
}

/// The pinned place in Google's `data=` segment: the last `!3d<lat>!4d<lon>`.
fn pinned_coordinates(data: &str) -> Option<Coordinate> {
    let start = data.rfind("!3d")? + "!3d".len();
    let (lat, rest) = data[start..].split_once("!4d")?;
    let lon = rest.split('!').next()?;
    Coordinate::parse_pair(&format!("{lat},{lon}")).ok()
}

fn query_value(url: &Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.into_owned())
}

/// Decodes a path segment or `geo:` query value: `+` is a space, then percent-decoding.
fn decode_component(text: &str) -> String {
    percent_decode_str(&text.replace('+', " "))
        .decode_utf8_lossy()
        .into_owned()
}
```

- [ ] **Step 5: Run the tests, lints and formatting**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p carsharing-core`
Expected: no warnings; all tests pass.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/core/Cargo.toml crates/core/src/link.rs crates/core/src/lib.rs
git commit -m "feat(core): parse Google Maps links, geo: URIs and shared text into destinations" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Days and vote weighting

The acceptance rule of spec §7.3 and the weights of §7.4 as pure functions. The server stores
the votes and calls these; the app never decides acceptance itself.

Weight formula (decision 28): `age points × accuracy`, where age points = 20 for a new install,
+1 per day, at most 100; accuracy = `(matching + 1) × 100 ÷ (decided + 2)` (a smoothed share of
past votes that matched the final outcome; 50 with no history). New install: 20 × 50 = 1 000.
Maximum 10 000.

Acceptance (decision 29): accepted when at least one install other than the reporter voted yes
with a weight above zero, and the reporter's weight plus the weighted yes votes exceed the
weighted no votes. Only each install's last vote counts; the reporter's own votes are ignored
(their report already counts as a yes).

**Files:**
- Create: `crates/core/src/day.rs`, `crates/core/src/voting.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: nothing from earlier tasks
- Produces: `Day(pub i32)` (days since 1970-01-01, JSON integer) with
  `days_since(self, earlier: Day) -> u32` (0 if `earlier` is later); `InstallId(pub String)`;
  `Weight(pub u32)`, `MAX_WEIGHT`; `InstallRecord { first_seen: Day, decided_votes: u32,
  matching_votes: u32 }`; `Answer::{Yes, No}` (JSON `"yes"`, `"no"`); `Vote { install, answer,
  weight }`; `weight(&InstallRecord, today: Day) -> Weight`;
  `is_accepted(reporter: &InstallId, reporter_weight: Weight, votes: &[Vote]) -> bool`.

- [ ] **Step 1: Register the modules and write the failing tests**

Add to `crates/core/src/lib.rs`:

```rust
pub mod day;
pub mod voting;

pub use day::Day;
pub use voting::{Answer, InstallId, InstallRecord, Vote, Weight, is_accepted, weight};
```

Create `crates/core/src/day.rs` containing only the tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_whole_days() {
        assert_eq!(Day(10).days_since(Day(3)), 7);
        assert_eq!(Day(3).days_since(Day(3)), 0);
    }

    #[test]
    fn a_later_date_counts_as_zero_days() {
        assert_eq!(Day(3).days_since(Day(10)), 0);
    }

    #[test]
    fn extreme_dates_do_not_overflow() {
        assert_eq!(Day(i32::MAX).days_since(Day(i32::MIN)), u32::MAX);
    }
}
```

Create `crates/core/src/voting.rs` containing only the tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn install(first_seen: i32, decided_votes: u32, matching_votes: u32) -> InstallRecord {
        InstallRecord {
            first_seen: Day(first_seen),
            decided_votes,
            matching_votes,
        }
    }

    fn vote(install: &str, answer: Answer, weight: u32) -> Vote {
        Vote {
            install: InstallId(install.into()),
            answer,
            weight: Weight(weight),
        }
    }

    fn reporter() -> InstallId {
        InstallId("reporter".into())
    }

    #[test]
    fn a_new_install_without_history_weighs_little() {
        assert_eq!(weight(&install(100, 0, 0), Day(100)), Weight(1_000));
    }

    #[test]
    fn weight_grows_with_age_up_to_a_cap() {
        assert_eq!(weight(&install(0, 0, 0), Day(40)), Weight(3_000));
        assert_eq!(weight(&install(0, 0, 0), Day(80)), Weight(5_000));
        assert_eq!(weight(&install(0, 0, 0), Day(5_000)), Weight(5_000));
    }

    #[test]
    fn weight_follows_how_often_past_votes_matched() {
        assert_eq!(weight(&install(0, 8, 8), Day(80)), Weight(9_000));
        assert_eq!(weight(&install(0, 8, 0), Day(80)), Weight(1_000));
    }

    #[test]
    fn odd_histories_stay_in_range() {
        // More matching than decided votes: treated as all decided votes matching.
        assert_eq!(weight(&install(0, 2, 10), Day(0)), Weight(1_500));
        assert_eq!(weight(&install(0, u32::MAX, 0), Day(80)), Weight(0));
        assert!(weight(&install(i32::MIN, u32::MAX, u32::MAX), Day(i32::MAX)) <= MAX_WEIGHT);
        // First seen "in the future" (clock skew) counts as new.
        assert_eq!(weight(&install(200, 0, 0), Day(100)), Weight(1_000));
    }

    #[test]
    fn a_report_alone_is_not_accepted() {
        assert!(!is_accepted(&reporter(), Weight(5_000), &[]));
    }

    #[test]
    fn one_confirmation_from_another_install_is_enough() {
        assert!(is_accepted(
            &reporter(),
            Weight(1_000),
            &[vote("b", Answer::Yes, 1_000)]
        ));
    }

    #[test]
    fn weighted_no_votes_can_outweigh_the_yes_votes() {
        let votes = [vote("b", Answer::Yes, 1_000), vote("c", Answer::No, 5_000)];
        assert!(!is_accepted(&reporter(), Weight(1_000), &votes));
    }

    #[test]
    fn a_tie_is_not_accepted() {
        let votes = [vote("b", Answer::Yes, 1_000), vote("c", Answer::No, 2_000)];
        assert!(!is_accepted(&reporter(), Weight(1_000), &votes));
    }

    #[test]
    fn the_reporters_own_votes_are_ignored() {
        let votes = [vote("reporter", Answer::Yes, 9_000)];
        assert!(!is_accepted(&reporter(), Weight(1_000), &votes));
    }

    #[test]
    fn only_the_last_vote_of_each_install_counts() {
        let changed_to_no = [vote("b", Answer::Yes, 1_000), vote("b", Answer::No, 1_000)];
        assert!(!is_accepted(&reporter(), Weight(1_000), &changed_to_no));
        let changed_to_yes = [vote("b", Answer::No, 1_000), vote("b", Answer::Yes, 1_000)];
        assert!(is_accepted(&reporter(), Weight(1_000), &changed_to_yes));
        // Repeating a yes does not add weight: 1000 + 1000 is not more than 2500.
        let stuffed = [
            vote("b", Answer::Yes, 1_000),
            vote("b", Answer::Yes, 1_000),
            vote("b", Answer::Yes, 1_000),
            vote("c", Answer::No, 2_500),
        ];
        assert!(!is_accepted(&reporter(), Weight(1_000), &stuffed));
    }

    #[test]
    fn a_confirmation_without_weight_does_not_count() {
        assert!(!is_accepted(
            &reporter(),
            Weight(1_000),
            &[vote("b", Answer::Yes, 0)]
        ));
    }

    #[test]
    fn answers_use_lowercase_json() {
        assert_eq!(serde_json::to_string(&Answer::No).unwrap(), r#""no""#);
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p carsharing-core` (`cargo test` takes a single name filter, and both new
modules must fail)
Expected: compile errors naming the items Step 3 defines (`Day`, `InstallRecord`, `Weight`,
`weight`, `is_accepted`, `Vote`, `Answer`, `InstallId`, `MAX_WEIGHT`).

- [ ] **Step 3: Write the implementations above the tests**

Insert at the top of `crates/core/src/day.rs`:

```rust
//! Calendar days as plain numbers, so the core needs no clock: callers pass today's date.

use serde::{Deserialize, Serialize};

/// Days since 1970-01-01 (UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Day(pub i32);

impl Day {
    /// Whole days from `earlier` to `self`; zero if `earlier` is later (a wrong device clock).
    pub fn days_since(self, earlier: Day) -> u32 {
        u32::try_from(i64::from(self.0) - i64::from(earlier.0)).unwrap_or(0)
    }
}
```

Insert at the top of `crates/core/src/voting.rs`:

```rust
//! When a crowd price report becomes the shared price (spec §7.3) and how much each install's
//! vote counts (§7.4). The server stores reports and votes and calls these functions.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::day::Day;

/// Anonymous random identity of one app installation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InstallId(pub String);

/// How much one install's vote counts, from 0 to [`MAX_WEIGHT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Weight(pub u32);

pub const MAX_WEIGHT: Weight = Weight(10_000);

/// Age points of a brand-new install; one more per day, up to [`MAX_AGE_POINTS`].
const NEW_INSTALL_POINTS: u32 = 20;
const MAX_AGE_POINTS: u32 = 100;

/// What the server knows about an install's past.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallRecord {
    pub first_seen: Day,
    /// Votes on reports that have since been decided.
    pub decided_votes: u32,
    /// Of those, how many agreed with the final outcome.
    pub matching_votes: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Answer {
    Yes,
    No,
}

/// One install's answer about one price report, with its weight when it voted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    pub install: InstallId,
    pub answer: Answer,
    pub weight: Weight,
}

/// An install's weight: age points (20 when new, +1 a day, at most 100) times the smoothed share
/// of its past votes that matched the outcome (50 % with no history).
pub fn weight(install: &InstallRecord, today: Day) -> Weight {
    let age_points = NEW_INSTALL_POINTS
        .saturating_add(today.days_since(install.first_seen))
        .min(MAX_AGE_POINTS);
    let decided = u64::from(install.decided_votes);
    let matching = u64::from(install.matching_votes.min(install.decided_votes));
    let accuracy = (matching + 1) * 100 / (decided + 2);
    Weight(u32::try_from(u64::from(age_points) * accuracy).unwrap_or(MAX_WEIGHT.0))
}

/// Whether a price report is accepted: at least one install other than the reporter confirmed it
/// with a weight above zero, and the reporter's weight plus the weighted yes votes exceed the
/// weighted no votes. Votes are in the order they were cast; only each install's last vote
/// counts, and the reporter's own votes are ignored.
pub fn is_accepted(reporter: &InstallId, reporter_weight: Weight, votes: &[Vote]) -> bool {
    let mut last_votes: HashMap<&InstallId, &Vote> = HashMap::new();
    for vote in votes.iter().filter(|vote| &vote.install != reporter) {
        last_votes.insert(&vote.install, vote);
    }
    let mut yes = u64::from(reporter_weight.0);
    let mut no = 0;
    let mut confirmed = false;
    for vote in last_votes.values() {
        let weight = u64::from(vote.weight.0);
        match vote.answer {
            Answer::Yes => {
                yes += weight;
                confirmed |= weight > 0;
            }
            Answer::No => no += weight,
        }
    }
    confirmed && yes > no
}
```

- [ ] **Step 4: Run the tests, lints and formatting**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p carsharing-core`
Expected: no warnings; all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/day.rs crates/core/src/voting.rs crates/core/src/lib.rs
git commit -m "feat(core): vote weights and the report acceptance rule" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: When to ask, and price freshness

Spec §7.2: ask at most once a week per install, and only when there is a pending report or a
price not confirmed recently. §7.5: show how many days ago a price was confirmed. Decision 30:
"recently" = 14 days. Both numbers are constants and the tests are written against the constants,
so tuning either is a one-line change (it reaches users with the next app release).

**Files:**
- Create: `crates/core/src/confirmation.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `Day`, `Day::days_since` (Task 3)
- Produces: `ASK_INTERVAL_DAYS = 7`, `STALE_AFTER_DAYS = 14`;
  `should_ask(today: Day, last_asked: Option<Day>, pending_report: bool, last_confirmed:
  Option<Day>) -> bool`; `confirmed_days_ago(today: Day, last_confirmed: Option<Day>) ->
  Option<u32>`.

- [ ] **Step 1: Register the module and write the failing tests**

Add to `crates/core/src/lib.rs`:

```rust
pub mod confirmation;

pub use confirmation::{confirmed_days_ago, should_ask};
```

Create `crates/core/src/confirmation.rs` containing only the tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const TODAY: Day = Day(20_000);

    fn days_ago(days: u32) -> Option<Day> {
        Some(Day(TODAY.0 - i32::try_from(days).unwrap()))
    }

    fn days_ahead(days: i32) -> Option<Day> {
        Some(Day(TODAY.0 + days))
    }

    #[test]
    fn asks_about_a_pending_report_when_never_asked() {
        assert!(should_ask(TODAY, None, true, days_ago(1)));
    }

    #[test]
    fn asks_about_a_price_never_confirmed() {
        assert!(should_ask(TODAY, None, false, None));
    }

    #[test]
    fn asks_at_most_once_per_interval() {
        assert!(!should_ask(TODAY, days_ago(0), true, None));
        assert!(!should_ask(TODAY, days_ago(ASK_INTERVAL_DAYS - 1), true, None));
        assert!(should_ask(TODAY, days_ago(ASK_INTERVAL_DAYS), true, None));
    }

    #[test]
    fn does_not_ask_about_a_recently_confirmed_price() {
        assert!(!should_ask(TODAY, None, false, days_ago(0)));
        assert!(!should_ask(TODAY, None, false, days_ago(STALE_AFTER_DAYS - 1)));
        assert!(should_ask(TODAY, None, false, days_ago(STALE_AFTER_DAYS)));
    }

    #[test]
    fn a_last_asked_date_in_the_future_does_not_block_asking() {
        assert!(should_ask(TODAY, days_ahead(400), true, None));
    }

    #[test]
    fn freshness_counts_days_since_confirmation() {
        assert_eq!(confirmed_days_ago(TODAY, days_ago(2)), Some(2));
        assert_eq!(confirmed_days_ago(TODAY, None), None);
        // Confirmed "in the future" (clock skew) shows as today.
        assert_eq!(confirmed_days_ago(TODAY, days_ahead(3)), Some(0));
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p carsharing-core confirmation`
Expected: compile errors naming the items Step 3 defines (`should_ask`, `confirmed_days_ago`).

- [ ] **Step 3: Write the implementation above the tests**

Insert at the top of `crates/core/src/confirmation.rs`:

```rust
//! When the app asks the user to confirm a price (spec §7.2), and how fresh a price is (§7.5).

use crate::day::Day;

/// The app asks at most once in this many days per install.
pub const ASK_INTERVAL_DAYS: u32 = 7;

/// A price not confirmed for this many days is worth asking about.
pub const STALE_AFTER_DAYS: u32 = 14;

/// Whether to show the one-question confirmation when the user comes back from a provider's app.
///
/// A `last_asked` date later than `today` (a wrong device clock) does not block asking.
pub fn should_ask(
    today: Day,
    last_asked: Option<Day>,
    pending_report: bool,
    last_confirmed: Option<Day>,
) -> bool {
    let asked_recently = last_asked
        .is_some_and(|day| day <= today && today.days_since(day) < ASK_INTERVAL_DAYS);
    let stale = last_confirmed.is_none_or(|day| today.days_since(day) >= STALE_AFTER_DAYS);
    !asked_recently && (pending_report || stale)
}

/// Whole days since the price was last confirmed, for the "confirmed 2 days ago" label;
/// `None` if it never was.
pub fn confirmed_days_ago(today: Day, last_confirmed: Option<Day>) -> Option<u32> {
    last_confirmed.map(|day| today.days_since(day))
}
```

- [ ] **Step 4: Run the tests, lints and formatting**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p carsharing-core`
Expected: no warnings; all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/confirmation.rs crates/core/src/lib.rs
git commit -m "feat(core): confirmation timing and price freshness" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Decisions, spec, README, roadmap, final verification

**Files:**
- Modify: `docs/DECISIONS.md`, `docs/specs/2026-10-19-carsharing-compare-design.md`,
  `docs/plans/ROADMAP.md`, `README.md`

**Interfaces:**
- Consumes: everything above
- Produces: nothing new

- [ ] **Step 1: Record the decisions**

Append to the table in `docs/DECISIONS.md` (use the date the task runs):

```markdown
| 27 | 2026-10-19 | Scope | Airport/zone fees are **not** in milestone 2; moved to a later milestone | Building them now with the link parsing | Owner decision: not needed yet (YAGNI) |
| 28 | 2026-10-19 | Crowd prices | Vote weight = age points (20 new, +1/day, max 100) × smoothed accuracy `(matching+1)×100/(decided+2)` | Equal weights; age only | New installs weigh little; installs that voted against outcomes lose influence; integer-only |
| 29 | 2026-10-19 | Crowd prices | Acceptance: ≥1 other install confirms with weight > 0, and reporter + weighted yes > weighted no; last vote per install counts; reporter's own votes ignored | Reporter excluded from the sum; every vote counts | Works from the second user; repeat votes and self-confirmation add nothing |
| 30 | 2026-10-19 | Crowd prices | Ask at most every 7 days; a price is "not recently confirmed" after 14 days | 30 days | Prices change often (dynamic pricing); weekly asking stays unintrusive |
| 31 | 2026-10-19 | Core | Coordinates as integer 1e-7 degrees parsed from text; short links only detected in the core, expanded by the server | `f64` coordinates; expanding in the app | No floats in the core (same results everywhere); expansion needs the network |
```

- [ ] **Step 2: Update the spec's adjustment order**

In `docs/specs/2026-10-19-carsharing-compare-design.md` §6, replace

```markdown
Order of application (to be fixed in the core and tested): override → subscription → percentage
discount → fixed discount → minimum charge.
```

with

```markdown
Order of application (fixed in the core and tested): override → subscription → percentage
discount → fixed discount and refuel credit → minimum charge.
```

- [ ] **Step 3: README and roadmap**

In `README.md`, replace `Milestone 1 (pricing core) is done` with
`Milestones 1 (pricing core) and 2 (destination input) are done`.

In `docs/plans/ROADMAP.md`:
- in the milestone 2 row, remove `airport/zone fees (e.g. MILES at BER) from start/destination
  coordinates; ` from the Delivers cell and change the plan cell from `—` to
  `` `02-destination-input.md` ✅ ``;
- add a row after milestone 8: `| 9 | **Zone fees** | Airport/zone fees (e.g. MILES at BER) from start/destination coordinates | §5 | — |`.

- [ ] **Step 4: Run the full verification**

Run:

```bash
cargo fmt --all --check
RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
cargo deny check
gitleaks git --no-banner --redact --config .gitleaks.toml
grep -rn "f32\|f64" crates/core/src || echo "no floats"
```

Expected: no formatting differences, no warnings, 100 tests pass, cargo-deny
`advisories ok, bans ok, licenses ok, sources ok`, `no leaks found`, `no floats`.

- [ ] **Step 5: Commit**

```bash
git add docs/DECISIONS.md docs/specs/2026-10-19-carsharing-compare-design.md docs/plans/ROADMAP.md README.md
git commit -m "docs: decisions 27-31 and milestone 2 done" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 6: Hand over to the owner**

Pushing is outward-facing: the owner approves it explicitly. Then:

```bash
git push -u origin feat/m2-destination-input
```

Report the free disk space (`df -h /System/Volumes/Data`) with the `cargo clean` command for this
workspace.
