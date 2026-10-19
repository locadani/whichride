# Milestone 2: Destination Input — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the pure `carsharing-core` crate so it can turn a shared Google Maps link,
`geo:` URI or shared text into a destination, charge airport/zone fees from the trip's start and
end coordinates, and decide crowd price reports with the voting rules of spec §7.2–7.5.

**Architecture:** Everything stays in `crates/core`: pure functions, no I/O, no clock, no
floating point. Coordinates are integers in 1e-7 degrees (OpenStreetMap's precision) parsed from
text without `f64`. Dates are whole days since 1970-01-01 passed in by the caller. Short links
(`maps.app.goo.gl`) are only *detected* here; expanding them needs the network and belongs to the
server (milestone 3).

**Tech Stack:** Rust (edition 2024), serde + serde_json, thiserror, proptest (tests), plus two new
dependencies: `url` 2 (WHATWG URL parsing, also used by the server later) and `percent-encoding` 2
(decoding path segments). Both are MIT/Apache-2.0 and build for Android and WebAssembly.

**Spec:** `docs/specs/2026-10-19-carsharing-compare-design.md` (§4 core flow, §5 zone fees, §6
adjustment order, §7 crowd prices, §9 privacy). Decisions: `docs/DECISIONS.md`. Previous milestone:
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
   confirmation question forever or produce a negative age (tests in Task 5 and Task 6).
5. **Vote stuffing:** one install voting many times, the reporter confirming their own report, an
   install whose weight has dropped to zero. None of these may accept a report on its own (tests
   in Task 5).

---

## File Structure

```
crates/core/
├─ Cargo.toml                 + url, percent-encoding
└─ src/
   ├─ lib.rs                  module list + re-exports (modified in every task)
   ├─ geo.rs                  NEW  Coordinate (1e-7°), parse_pair, polygon_contains
   ├─ link.rs                 NEW  Destination, LinkError, parse_destination
   ├─ zone.rs                 NEW  ZoneFee, ChargedWhen, Ends, ZoneCharge (+ fixtures)
   ├─ catalogue.rs            Provider.zone_fees + validation; fixture gets a BER zone
   ├─ compare.rs              compare(.., ends, ..) adds zone fees; Quote.zone_fees
   ├─ day.rs                  NEW  Day (days since 1970-01-01)
   ├─ voting.rs               NEW  InstallId, Weight, InstallRecord, Vote, Answer, weight, is_accepted
   └─ confirmation.rs         NEW  should_ask, confirmed_days_ago
Cargo.toml                    + url, percent-encoding in [workspace.dependencies]
docs/DECISIONS.md             rows 27–31
docs/specs/…-design.md        §6 order line updated
docs/plans/ROADMAP.md         milestone 2 ✅
README.md                     status line
```

---

### Task 1: Coordinates

Integer coordinates parsed from decimal text, and a point-in-polygon test, all without floating
point.

**Files:**
- Create: `crates/core/src/geo.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: nothing
- Produces: `Coordinate { lat_e7: i32, lon_e7: i32 }` (Copy, Eq, Hash, serde as
  `{"lat_e7":…,"lon_e7":…}`); `Coordinate::new(i32, i32) -> Result<Coordinate, CoordinateError>`;
  `Coordinate::parse_pair(&str) -> Result<Coordinate, CoordinateError>`;
  `CoordinateError::{NotAPair, OutOfRange}`; `polygon_contains(&[Coordinate], Coordinate) -> bool`.

- [ ] **Step 1: Register the module and write the failing tests**

Add to `crates/core/src/lib.rs` (keep `pub mod` lines and `pub use` lines each in alphabetical
order; `cargo fmt` orders the names inside braces):

```rust
pub mod geo;

pub use geo::{Coordinate, CoordinateError, polygon_contains};
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

    fn square() -> Vec<Coordinate> {
        vec![at(0, 0), at(0, 100), at(100, 100), at(100, 0)]
    }

    #[test]
    fn polygon_contains_points_inside_only() {
        assert!(polygon_contains(&square(), at(50, 50)));
        assert!(!polygon_contains(&square(), at(150, 50)));
        assert!(!polygon_contains(&square(), at(50, -1)));
    }

    #[test]
    fn polygon_contains_handles_concave_shapes() {
        // An L shape: the notch at the top right is outside.
        let l_shape = [
            at(0, 0),
            at(0, 40),
            at(20, 40),
            at(20, 20),
            at(40, 20),
            at(40, 0),
        ];
        assert!(polygon_contains(&l_shape, at(10, 30)));
        assert!(polygon_contains(&l_shape, at(30, 10)));
        assert!(!polygon_contains(&l_shape, at(30, 30)));
    }

    #[test]
    fn fewer_than_three_points_contain_nothing() {
        assert!(!polygon_contains(&[], at(0, 0)));
        assert!(!polygon_contains(&[at(0, 0), at(10, 10)], at(5, 5)));
    }

    #[test]
    fn extreme_polygons_do_not_overflow() {
        let world = [
            at(-900_000_000, -1_800_000_000),
            at(-900_000_000, 1_800_000_000),
            at(900_000_000, 1_800_000_000),
            at(900_000_000, -1_800_000_000),
        ];
        assert!(polygon_contains(&world, at(525_219_184, 134_132_147)));
        // A point on a vertex may count as either side, but must not panic.
        let _ = polygon_contains(&world, world[0]);
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p carsharing-core geo`
Expected: compile errors naming the items Step 3 defines (`Coordinate`, `CoordinateError`,
`polygon_contains`).

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

/// Whether `point` lies inside the polygon `area` (vertices in order, the last joined to the
/// first), by even-odd ray casting in exact integer arithmetic. Fewer than three vertices contain
/// nothing. A point exactly on an edge or vertex may count as either side.
pub fn polygon_contains(area: &[Coordinate], point: Coordinate) -> bool {
    if area.len() < 3 {
        return false;
    }
    let (py, px) = (i128::from(point.lat_e7), i128::from(point.lon_e7));
    let mut inside = false;
    let mut previous = area[area.len() - 1];
    for &current in area {
        let (ay, ax) = (i128::from(previous.lat_e7), i128::from(previous.lon_e7));
        let (by, bx) = (i128::from(current.lat_e7), i128::from(current.lon_e7));
        if (ay > py) != (by > py) {
            // Is the point west of where this edge crosses its latitude? Compared without division.
            let lhs = (px - ax) * (by - ay);
            let rhs = (bx - ax) * (py - ay);
            if (by > ay && lhs < rhs) || (by < ay && lhs > rhs) {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}
```

- [ ] **Step 4: Run the tests, lints and formatting**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p carsharing-core`
Expected: no warnings; all tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/geo.rs crates/core/src/lib.rs
git commit -m "feat(core): integer coordinates and point-in-polygon without floats" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
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

### Task 3: Zone fees in the catalogue

Providers can charge a fee when a trip starts and/or ends inside an area (MILES at BER,
Free2move airport fee). The area is a polygon in the provider data, so new zones need no code.

**Files:**
- Create: `crates/core/src/zone.rs`
- Modify: `crates/core/src/catalogue.rs`, `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `Coordinate`, `polygon_contains` (Task 1); `Cents` (milestone 1); `Provider`,
  `Catalogue::validate`, `CatalogueError`, `catalogue::fixtures::catalogue()` (milestone 1)
- Produces: `Ends { start: Coordinate, end: Coordinate }`; `ChargedWhen::{AtStart, AtEnd,
  AtStartOrEnd}` (JSON `"at_start"`, `"at_end"`, `"at_start_or_end"`); `ZoneFee { name, area:
  Vec<Coordinate>, fee: Cents, when: ChargedWhen }` with `applies(&Ends) -> bool`; `ZoneCharge {
  name, fee }`; `Provider.zone_fees: Vec<ZoneFee>` (JSON default `[]`);
  `CatalogueError::{ZoneTooSmall { provider, zone }, NegativeZoneFee { provider, zone }}`;
  test-only `zone::fixtures::{ALEXANDERPLATZ, BER_T1, ber_fee()}` (`ber_fee()` = "BER airport",
  €10, `AtStartOrEnd`); the catalogue fixture's `miles` provider now has `zone_fees: [ber_fee()]`.

- [ ] **Step 1: Register the module and write the failing tests**

Add to `crates/core/src/lib.rs`:

```rust
pub mod zone;

pub use zone::{ChargedWhen, Ends, ZoneCharge, ZoneFee};
```

Create `crates/core/src/zone.rs` containing only the fixtures and tests:

```rust
#[cfg(test)]
pub(crate) mod fixtures {
    use super::*;

    pub const ALEXANDERPLATZ: Coordinate = Coordinate {
        lat_e7: 525_219_184,
        lon_e7: 134_132_147,
    };

    /// BER airport, Terminal 1.
    pub const BER_T1: Coordinate = Coordinate {
        lat_e7: 523_667_000,
        lon_e7: 135_033_000,
    };

    /// A rectangle around BER airport. Test data, not the real fee area.
    pub fn ber_fee() -> ZoneFee {
        ZoneFee {
            name: "BER airport".into(),
            area: vec![
                Coordinate {
                    lat_e7: 523_500_000,
                    lon_e7: 134_800_000,
                },
                Coordinate {
                    lat_e7: 523_500_000,
                    lon_e7: 135_400_000,
                },
                Coordinate {
                    lat_e7: 523_900_000,
                    lon_e7: 135_400_000,
                },
                Coordinate {
                    lat_e7: 523_900_000,
                    lon_e7: 134_800_000,
                },
            ],
            fee: Cents(1_000),
            when: ChargedWhen::AtStartOrEnd,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    const TO_BER: Ends = Ends {
        start: ALEXANDERPLATZ,
        end: BER_T1,
    };
    const FROM_BER: Ends = Ends {
        start: BER_T1,
        end: ALEXANDERPLATZ,
    };
    const IN_TOWN: Ends = Ends {
        start: ALEXANDERPLATZ,
        end: ALEXANDERPLATZ,
    };

    fn ber(when: ChargedWhen) -> ZoneFee {
        ZoneFee { when, ..ber_fee() }
    }

    #[test]
    fn at_end_applies_only_when_ending_inside() {
        assert!(ber(ChargedWhen::AtEnd).applies(&TO_BER));
        assert!(!ber(ChargedWhen::AtEnd).applies(&FROM_BER));
    }

    #[test]
    fn at_start_applies_only_when_starting_inside() {
        assert!(ber(ChargedWhen::AtStart).applies(&FROM_BER));
        assert!(!ber(ChargedWhen::AtStart).applies(&TO_BER));
    }

    #[test]
    fn at_start_or_end_applies_to_either_direction_only() {
        let fee = ber(ChargedWhen::AtStartOrEnd);
        assert!(fee.applies(&TO_BER));
        assert!(fee.applies(&FROM_BER));
        assert!(!fee.applies(&IN_TOWN));
    }

    #[test]
    fn zone_with_too_few_points_never_applies() {
        let fee = ZoneFee {
            area: vec![BER_T1, ALEXANDERPLATZ],
            ..ber_fee()
        };
        assert!(!fee.applies(&TO_BER));
    }

    #[test]
    fn zone_fee_reads_from_json() {
        let json = r#"{"name":"BER","area":[{"lat_e7":1,"lon_e7":2}],"fee":1000,"when":"at_end"}"#;
        let fee: ZoneFee = serde_json::from_str(json).unwrap();
        assert_eq!(fee.when, ChargedWhen::AtEnd);
        assert_eq!(fee.fee, Cents(1_000));
    }
}
```

Append these tests inside the existing `mod tests` of `crates/core/src/catalogue.rs`:

```rust
    #[test]
    fn rejects_zone_with_fewer_than_three_points() {
        let mut c = catalogue();
        c.providers[0].zone_fees[0].area.truncate(2);
        assert!(
            matches!(c.validate(), Err(CatalogueError::ZoneTooSmall { provider, .. }) if provider == "miles")
        );
    }

    #[test]
    fn rejects_negative_zone_fee() {
        let mut c = catalogue();
        c.providers[0].zone_fees[0].fee = Cents(-1);
        assert!(
            matches!(c.validate(), Err(CatalogueError::NegativeZoneFee { zone, .. }) if zone == "BER airport")
        );
    }

    #[test]
    fn provider_without_zone_fees_reads_as_none() {
        let json = r#"{"id":"x","name":"X","android_package":"p","offers":[]}"#;
        let provider: Provider = serde_json::from_str(json).unwrap();
        assert!(provider.zone_fees.is_empty());
    }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p carsharing-core zone`
Expected: compile errors naming the items Step 3 defines (`ZoneFee`, `ChargedWhen`, `Ends`,
`zone_fees`, `ZoneTooSmall`, `NegativeZoneFee`).

- [ ] **Step 3: Write the implementation**

Insert at the top of `crates/core/src/zone.rs`:

```rust
//! Fees some providers charge when a trip starts or ends inside an area, e.g. an airport.

use serde::{Deserialize, Serialize};

use crate::geo::{Coordinate, polygon_contains};
use crate::money::Cents;

/// Where a trip starts and ends. Unknown for trips entered by hand as km and minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ends {
    pub start: Coordinate,
    pub end: Coordinate,
}

/// Which end of the trip must be inside the area for the fee to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChargedWhen {
    AtStart,
    AtEnd,
    AtStartOrEnd,
}

/// A fee for starting and/or ending a trip inside an area; charged at most once per trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneFee {
    /// Shown to the user, e.g. "BER airport".
    pub name: String,
    /// Polygon vertices in order; the last joins the first.
    pub area: Vec<Coordinate>,
    pub fee: Cents,
    pub when: ChargedWhen,
}

/// A zone fee included in a quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZoneCharge {
    pub name: String,
    pub fee: Cents,
}

impl ZoneFee {
    /// Whether this fee is charged for a trip between `ends`.
    pub fn applies(&self, ends: &Ends) -> bool {
        let starts_inside = || polygon_contains(&self.area, ends.start);
        let ends_inside = || polygon_contains(&self.area, ends.end);
        match self.when {
            ChargedWhen::AtStart => starts_inside(),
            ChargedWhen::AtEnd => ends_inside(),
            ChargedWhen::AtStartOrEnd => starts_inside() || ends_inside(),
        }
    }
}
```

In `crates/core/src/catalogue.rs`:

1. Add `use crate::zone::ZoneFee;` after `use crate::rate_card::RateCard;`.
2. In `struct Provider`, after the `refuel_credit` field, add:

```rust
    /// Fees for starting or ending in particular areas, e.g. an airport.
    #[serde(default)]
    pub zone_fees: Vec<ZoneFee>,
```

3. In `enum CatalogueError`, after `NegativePrice`, add:

```rust
    #[error("zone `{zone}` of provider `{provider}` needs at least three points")]
    ZoneTooSmall { provider: String, zone: String },
    #[error("zone `{zone}` of provider `{provider}` has a negative fee")]
    NegativeZoneFee { provider: String, zone: String },
```

4. In `Catalogue::validate`, right after the `NegativeRefuelCredit` check, add:

```rust
            for zone in &provider.zone_fees {
                let names = || (provider.id.0.clone(), zone.name.clone());
                if zone.area.len() < 3 {
                    let (provider, zone) = names();
                    return Err(CatalogueError::ZoneTooSmall { provider, zone });
                }
                if zone.fee < Cents::ZERO {
                    let (provider, zone) = names();
                    return Err(CatalogueError::NegativeZoneFee { provider, zone });
                }
            }
```

5. In `mod fixtures`, add `use crate::zone::fixtures::ber_fee;` and give the providers their
   zone fees: `zone_fees: vec![ber_fee()],` after `refuel_credit: Some(Cents(500)),` (miles) and
   `zone_fees: vec![],` after `refuel_credit: None,` (bolt).

- [ ] **Step 4: Run the tests, lints and formatting**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p carsharing-core`
Expected: no warnings; all tests pass (the existing catalogue and compare tests are unaffected:
nothing passes trip ends yet).

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/zone.rs crates/core/src/catalogue.rs crates/core/src/lib.rs
git commit -m "feat(core): providers can define airport and zone fees as polygons" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Zone fees in the comparison

`compare` takes the trip's ends when they are known and adds each provider's applicable zone
fees. Zone fees are added **after** personal adjustments and the minimum charge and are never
discounted (decision 27: they are surcharges for a place, not part of the driving price).

**Files:**
- Modify: `crates/core/src/compare.rs`

**Interfaces:**
- Consumes: `Ends`, `ZoneCharge`, `ZoneFee::applies`, `zone::fixtures` (Task 3); `Provider.zone_fees`
  (Task 3)
- Produces: `compare(&Catalogue, &Trip, Option<&Ends>, &Adjustments) -> Vec<Quote>` (new third
  parameter); `Quote.zone_fees: Vec<ZoneCharge>` (included in `total`).

- [ ] **Step 1: Update the existing calls and write the failing tests**

Give every existing `compare(…)` call in the tests a `None` third argument (7 calls):

```bash
perl -0pi -e 's/(compare\(\s*&\w+(?:\(\))?,\s*&(?:TRIP|long|trip|Trip \{[^}]*\}),)/$1 None,/g' crates/core/src/compare.rs
grep -c 'None,' crates/core/src/compare.rs
```

Expected: `7`.

Then add to the `use` lines of `mod tests`:

```rust
    use crate::zone::ZoneFee;
    use crate::zone::fixtures::{ALEXANDERPLATZ, BER_T1, ber_fee};
```

and append these tests inside `mod tests` (before the `proptest!` block):

```rust
    const TO_BER: Ends = Ends {
        start: ALEXANDERPLATZ,
        end: BER_T1,
    };

    #[test]
    fn zone_fee_is_added_when_the_trip_ends_in_the_zone() {
        let quotes = compare(&catalogue(), &TRIP, Some(&TO_BER), &Adjustments::default());
        let miles = quotes.iter().find(|q| q.offer.0 == "miles.s").unwrap();
        assert_eq!(miles.total, Cents(969 + 1_000));
        assert_eq!(
            miles.zone_fees,
            [ZoneCharge {
                name: "BER airport".into(),
                fee: Cents(1_000)
            }]
        );
        let bolt = quotes.iter().find(|q| q.offer.0 == "bolt.corsa").unwrap();
        assert_eq!((bolt.total, bolt.zone_fees.len()), (Cents(473), 0));
    }

    #[test]
    fn no_zone_fee_outside_the_zone_or_without_ends() {
        let in_town = Ends {
            start: ALEXANDERPLATZ,
            end: ALEXANDERPLATZ,
        };
        for ends in [Some(&in_town), None] {
            let quotes = compare(&catalogue(), &TRIP, ends, &Adjustments::default());
            let miles = quotes.iter().find(|q| q.offer.0 == "miles.s").unwrap();
            assert_eq!((miles.total, miles.zone_fees.len()), (Cents(969), 0));
        }
    }

    #[test]
    fn zone_fee_is_not_discounted() {
        let adjustments = Adjustments {
            discounts: vec![Discount::Percent {
                provider: ProviderId("miles".into()),
                off: Percent::from_basis_points(5_000).unwrap(),
            }],
            ..Adjustments::default()
        };
        let quotes = compare(&catalogue(), &TRIP, Some(&TO_BER), &adjustments);
        let miles = quotes.iter().find(|q| q.offer.0 == "miles.s").unwrap();
        // 969 × 0.5 = 484.5 → 485, plus the full €10 fee
        assert_eq!(miles.total, Cents(485 + 1_000));
    }

    #[test]
    fn zone_fee_is_charged_once_when_both_ends_are_inside() {
        let at_ber = Ends {
            start: BER_T1,
            end: BER_T1,
        };
        let zero = Trip {
            metres: 0,
            seconds: 0,
        };
        let quotes = compare(&catalogue(), &zero, Some(&at_ber), &Adjustments::default());
        let miles = quotes.iter().find(|q| q.offer.0 == "miles.s").unwrap();
        assert_eq!(miles.total, Cents(100 + 1_000));
    }

    #[test]
    fn several_zone_fees_add_up_in_catalogue_order() {
        let mut c = catalogue();
        c.providers[0].zone_fees.push(ZoneFee {
            name: "Test zone".into(),
            fee: Cents(200),
            ..ber_fee()
        });
        let quotes = compare(&c, &TRIP, Some(&TO_BER), &Adjustments::default());
        let miles = quotes.iter().find(|q| q.offer.0 == "miles.s").unwrap();
        assert_eq!(miles.total, Cents(969 + 1_000 + 200));
        let names: Vec<_> = miles.zone_fees.iter().map(|z| z.name.as_str()).collect();
        assert_eq!(names, ["BER airport", "Test zone"]);
    }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p carsharing-core compare`
Expected: compile errors: `compare` takes 3 arguments but 4 were supplied; `Ends`/`ZoneCharge`
not found; no field `zone_fees` on `Quote`.

- [ ] **Step 3: Write the implementation**

Replace everything in `crates/core/src/compare.rs` above `#[cfg(test)]` with:

```rust
//! The comparison the app shows: every offer's cheapest price for one trip.

use serde::{Deserialize, Serialize};

use crate::adjustments::Adjustments;
use crate::catalogue::{Catalogue, OfferId, Provider, ProviderId};
use crate::money::Cents;
use crate::usage::Trip;
use crate::zone::{Ends, ZoneCharge};

/// The best price of one offer for a trip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quote {
    pub provider: ProviderId,
    pub offer: OfferId,
    pub offer_name: String,
    /// The tariff that gave the lowest price, e.g. "Standard" or "3 h / 40 km".
    pub card_label: String,
    pub total: Cents,
    /// Zone fees included in `total`, e.g. an airport fee.
    pub zone_fees: Vec<ZoneCharge>,
    /// The price comes from the user's own override, not the shared price.
    pub overridden: bool,
}

/// Prices every offer of every provider and returns them cheapest first.
///
/// Each offer appears once, with its cheapest rate card; on a tie the first card wins. Zone fees
/// apply only when `ends` is known (not for trips entered by hand); they are added after personal
/// adjustments and the minimum charge and are never discounted. Equal totals are ordered by
/// provider and offer id, so the result is deterministic.
pub fn compare(
    catalogue: &Catalogue,
    trip: &Trip,
    ends: Option<&Ends>,
    adjustments: &Adjustments,
) -> Vec<Quote> {
    let mut quotes = Vec::new();
    for provider in &catalogue.providers {
        let zone_fees = zone_charges(provider, ends);
        let zone_total: Cents = zone_fees.iter().map(|charge| charge.fee).sum();
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
                    total: total + zone_total,
                    zone_fees: zone_fees.clone(),
                    overridden,
                });
            }
        }
    }
    quotes.sort_by(|a, b| (a.total, &a.provider, &a.offer).cmp(&(b.total, &b.provider, &b.offer)));
    quotes
}

/// The zone fees of `provider` that apply to a trip between `ends`.
fn zone_charges(provider: &Provider, ends: Option<&Ends>) -> Vec<ZoneCharge> {
    let Some(ends) = ends else {
        return Vec::new();
    };
    provider
        .zone_fees
        .iter()
        .filter(|zone| zone.applies(ends))
        .map(|zone| ZoneCharge {
            name: zone.name.clone(),
            fee: zone.fee,
        })
        .collect()
}
```

- [ ] **Step 4: Run the tests, lints and formatting**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets --locked -- -D warnings && cargo test -p carsharing-core`
Expected: no warnings; all tests pass, including the unchanged property test.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/compare.rs
git commit -m "feat(core): compare adds zone fees when the trip's ends are known" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Days and vote weighting

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

### Task 6: When to ask, and price freshness

Spec §7.2: ask at most once a week per install, and only when there is a pending report or a
price not confirmed recently. §7.5: show how many days ago a price was confirmed. Decision 30:
"recently" = 14 days.

**Files:**
- Create: `crates/core/src/confirmation.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `Day`, `Day::days_since` (Task 5)
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

    fn days_ago(days: i32) -> Option<Day> {
        Some(Day(TODAY.0 - days))
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
    fn asks_at_most_once_a_week() {
        assert!(!should_ask(TODAY, days_ago(3), true, None));
        assert!(!should_ask(TODAY, days_ago(6), true, None));
        assert!(should_ask(TODAY, days_ago(7), true, None));
    }

    #[test]
    fn does_not_ask_about_a_recently_confirmed_price() {
        assert!(!should_ask(TODAY, None, false, days_ago(5)));
        assert!(!should_ask(TODAY, None, false, days_ago(13)));
        assert!(should_ask(TODAY, None, false, days_ago(14)));
    }

    #[test]
    fn a_last_asked_date_in_the_future_does_not_block_asking() {
        assert!(should_ask(TODAY, days_ago(-400), true, None));
    }

    #[test]
    fn freshness_counts_days_since_confirmation() {
        assert_eq!(confirmed_days_ago(TODAY, days_ago(2)), Some(2));
        assert_eq!(confirmed_days_ago(TODAY, None), None);
        // Confirmed "in the future" (clock skew) shows as today.
        assert_eq!(confirmed_days_ago(TODAY, days_ago(-3)), Some(0));
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

### Task 7: Decisions, spec, README, roadmap, final verification

**Files:**
- Modify: `docs/DECISIONS.md`, `docs/specs/2026-10-19-carsharing-compare-design.md`,
  `docs/plans/ROADMAP.md`, `README.md`

**Interfaces:**
- Consumes: everything above
- Produces: nothing new

- [ ] **Step 1: Record the decisions**

Append to the table in `docs/DECISIONS.md` (use the date the task runs):

```markdown
| 27 | 2026-10-19 | Pricing | Zone/airport fees are polygons in the provider data, charged once per trip, added **after** personal adjustments and the minimum, never discounted; only when start and end are known | Zone fees as a rate card component; discounting them | A fee for a place is not part of the driving price; polygons let new zones ship as data |
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
discount → fixed discount and refuel credit → minimum charge → zone fees (never discounted;
decision 27).
```

- [ ] **Step 3: README and roadmap**

In `README.md`, replace `Milestone 1 (pricing core) is done` with
`Milestones 1 (pricing core) and 2 (destination input) are done`.

In `docs/plans/ROADMAP.md`, change the milestone 2 row's plan cell from `—` to
`` `02-destination-input.md` ✅ ``.

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

Expected: no formatting differences, no warnings, 117 tests pass, cargo-deny
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
