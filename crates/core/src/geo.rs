//! Points on Earth in integer 1e-7 degrees (about 1 cm), parsed from text without floating point.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Units of 1e-7 degrees in one degree.
const E7: i64 = 10_000_000;

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
}
