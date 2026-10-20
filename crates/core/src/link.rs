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
    let mut first_error = None;
    for link in find_links(input) {
        match parse_link(link) {
            Ok(destination) => return Ok(destination),
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }
    Err(first_error.unwrap_or(LinkError::NoLink))
}

fn parse_link(link: &str) -> Result<Destination, LinkError> {
    if starts_with_ignore_case(link, "geo:") {
        return parse_geo(&link["geo:".len()..]);
    }
    let url = Url::parse(link).map_err(|_| LinkError::NotGoogleMaps)?;
    if is_short_link(&url) {
        return short_link(&url);
    }
    if !is_google_maps(&url) {
        return Err(LinkError::NotGoogleMaps);
    }
    parse_google_maps(&url)
}

/// Every link in the text, in order: each word from its first `https://`, `http://` or `geo:`,
/// without punctuation stuck to its end (as in "(https://…)." pasted from a chat).
fn find_links(input: &str) -> impl Iterator<Item = &str> {
    input.split_whitespace().filter_map(|word| {
        let lowercase = word.to_ascii_lowercase();
        let start = ["https://", "http://", "geo:"]
            .iter()
            .filter_map(|scheme| lowercase.find(scheme))
            .min()?;
        Some(trim_trailing_punctuation(&word[start..]))
    })
}

/// Removes `.,;:!?]>"'` and unmatched `)` from the end; a `)` that closes a `(` in the link,
/// like a `geo:` label, is kept.
fn trim_trailing_punctuation(mut link: &str) -> &str {
    loop {
        let trimmed = match link.chars().last() {
            Some('.' | ',' | ';' | ':' | '!' | '?' | ']' | '>' | '"' | '\'') => {
                &link[..link.len() - 1]
            }
            Some(')') if link.matches('(').count() < link.matches(')').count() => {
                &link[..link.len() - 1]
            }
            _ => return link,
        };
        link = trimmed;
    }
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
    let country_code =
        |label: &str| label.len() == 2 && label.bytes().all(|b| b.is_ascii_lowercase());
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
    let near =
        viewport.or_else(|| query_value(url, "ll").and_then(|ll| Coordinate::parse_pair(&ll).ok()));

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
    // `lat,lon[,altitude][;parameters]`: only latitude and longitude matter.
    let position = position.split(';').next().unwrap_or_default();
    let position = match position.splitn(3, ',').collect::<Vec<_>>()[..] {
        [lat, lon, ..] => format!("{lat},{lon}"),
        _ => position.to_owned(),
    };
    let at = match Coordinate::parse_pair(&position) {
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
        assert_eq!(parse("geo:52.5,200"), Err(LinkError::InvalidCoordinates));
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
            Ok(coordinates(
                525_219_000,
                134_132_000,
                Some("Alexanderplatz")
            ))
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

    #[test]
    fn punctuation_around_the_link_is_ignored() {
        let expected = Ok(Destination::ShortLink {
            url: "https://maps.app.goo.gl/AbC123".into(),
        });
        for text in [
            "https://maps.app.goo.gl/AbC123.",
            "(https://maps.app.goo.gl/AbC123)",
            "<https://maps.app.goo.gl/AbC123>",
            "Link:https://maps.app.goo.gl/AbC123",
            "\"https://maps.app.goo.gl/AbC123\",",
        ] {
            assert_eq!(parse(text), expected, "{text}");
        }
        // A bracketed label that belongs to the link is kept.
        assert_eq!(
            parse("(geo:0,0?q=52.5219,13.4132(Alexanderplatz))"),
            Ok(coordinates(
                525_219_000,
                134_132_000,
                Some("Alexanderplatz")
            ))
        );
    }

    #[test]
    fn a_later_maps_link_wins_over_an_earlier_other_link() {
        assert_eq!(
            parse("via https://example.com https://maps.app.goo.gl/AbC123"),
            Ok(Destination::ShortLink {
                url: "https://maps.app.goo.gl/AbC123".into()
            })
        );
        assert_eq!(
            parse("via https://example.com"),
            Err(LinkError::NotGoogleMaps)
        );
    }

    #[test]
    fn geo_uris_with_altitude() {
        assert_eq!(
            parse("geo:52.5,13.4,100"),
            Ok(coordinates(525_000_000, 134_000_000, None))
        );
        assert_eq!(
            parse("geo:52.5,13.4,100;u=5"),
            Ok(coordinates(525_000_000, 134_000_000, None))
        );
    }
}
