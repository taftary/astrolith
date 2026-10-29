//! Shot files for `--screens <dir> --shots <file>` (#863).
//!
//! One shot per line: a name, a level, then `key=value` words. Blank lines
//! and lines starting with `#` are ignored.
//!
//! ```text
//! # name        level  where                          look
//! cliff-sunset  L10    lat=12.5 lon=-40 h=2           heading=270 tilt=-5
//! probe-xyz     L10    x=1200000 y=6200000 z=-300000  dir=0,-1,0
//! walk-ridge    L10    lat=30 lon=10 figure           heading=90
//! galaxy-view   L4     x=0.1 y=0.2 z=3                dir=0,0,-1
//! ```
//!
//! On the planet (L10) a place is `lat`/`lon` in degrees plus `h` metres
//! above the drawn ground (default 2), or `x y z` planet metres; above it,
//! `x y z` in that level's own units. A look is `heading` (compass
//! degrees, 0 north, 90 east) with `tilt` (degrees up), or `dir=a,b,c`.
//! `figure` (planet only) stands the walking figure there; `overlay` turns
//! the F3 debug overlay on for the shot (#918); `time=H` sets the sky's
//! game time to `H` hours after the start (Day 1, 08:00; default 0, #981
//! T6), with the clock paused. A wrong line is
//! refused with its line number and the problem (Q1 C, Q2 A, Q3 B, Q4 B).

/// Where a shot stands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShotPlace {
    /// On the planet: degrees, and metres above the drawn ground.
    Planet {
        /// Latitude in degrees, -90 to 90.
        lat: f64,
        /// Longitude in degrees, growing east.
        lon: f64,
        /// Height above the drawn ground in metres.
        h: f64,
    },
    /// Planet metres at L10, the level's own units above it.
    Xyz([f64; 3]),
}

/// Where a shot looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShotLook {
    /// Compass heading and tilt up, in degrees.
    HeadingTilt {
        /// 0 north, 90 east.
        heading: f64,
        /// Up from the horizon; negative looks down.
        tilt: f64,
    },
    /// A direction vector, normalised when placed.
    Dir([f64; 3]),
}

/// One shot from a shot file.
#[derive(Debug, Clone, PartialEq)]
pub struct ShotSpec {
    /// Picture name, saved as `<name>.png`.
    pub name: String,
    /// Level along the demo journey, 1 to 10.
    pub level: u8,
    /// Where the camera (or the figure) stands.
    pub place: ShotPlace,
    /// Where it looks.
    pub look: ShotLook,
    /// Stand the walking figure at the spot, camera behind it.
    pub figure: bool,
    /// Show the F3 debug overlay (#918).
    pub overlay: bool,
    /// The sky's game time, hours after the start (#981 T6).
    pub time_h: f64,
    /// Explicit sibling planet index in the demo L9 cell (developer fixtures).
    pub planet: Option<u32>,
    /// First-person body view; implies `figure`.
    pub first_person: bool,
    /// Marker selected in the open cell (developer fixtures).
    pub selected: Option<u32>,
}

/// The shots of a `--shots` run, in file order (#863). Present only in
/// such a run; it replaces the built-in list.
#[derive(bevy::prelude::Resource, Debug, Clone, Default, PartialEq)]
pub struct ShotFile(pub Vec<ShotSpec>);

/// Default height above the ground for a planet-terms shot, in metres.
pub(crate) const DEFAULT_HEIGHT_M: f64 = 2.0;

fn number(key: &str, value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{key}={value} is not a number"))
}

fn vector(value: &str) -> Result<[f64; 3], String> {
    let parts: Vec<&str> = value.split(',').collect();
    let [a, b, c] = parts.as_slice() else {
        return Err(format!("dir={value} needs three numbers a,b,c"));
    };
    let v = [number("dir", a)?, number("dir", b)?, number("dir", c)?];
    if v.iter().all(|x| *x == 0.0) {
        return Err("dir=0,0,0 has no direction".to_string());
    }
    Ok(v)
}

fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Reads one shot line (after the comment and blank filter).
pub(crate) fn parse_line(line: &str) -> Result<ShotSpec, String> {
    let mut words = line.split_whitespace();
    let name = words.next().unwrap_or_default();
    if !safe_name(name) {
        return Err(format!(
            "name '{name}' must be 1-64 letters, digits, '-' or '_'"
        ));
    }
    let level_word = words.next().ok_or("a level (L1-L10) is missing")?;
    let level = level_word
        .strip_prefix('L')
        .and_then(|n| n.parse::<u8>().ok())
        .filter(|n| (1..=10).contains(n))
        .ok_or_else(|| format!("level '{level_word}' must be L1 to L10"))?;
    let (mut lat, mut lon, mut h) = (None, None, None);
    let (mut x, mut y, mut z) = (None, None, None);
    let (mut heading, mut tilt, mut dir) = (None, None, None);
    let mut time_h = None;
    let mut figure = false;
    let mut overlay = false;
    let (mut planet, mut selected) = (None, None);
    let mut first_person = false;
    for word in words {
        if word == "figure" {
            figure = true;
            continue;
        }
        if word == "overlay" {
            overlay = true;
            continue;
        }
        if word == "first-person" {
            figure = true;
            first_person = true;
            continue;
        }
        let Some((key, value)) = word.split_once('=') else {
            return Err(format!("unknown word '{word}'"));
        };
        let slot = match key {
            "planet" | "select" => {
                let index = value
                    .parse::<u32>()
                    .map_err(|_| format!("{key} must be an unsigned marker index"))?;
                if key == "planet" {
                    planet = Some(index);
                } else {
                    selected = Some(index);
                }
                continue;
            }
            "lat" => &mut lat,
            "lon" => &mut lon,
            "h" => &mut h,
            "x" => &mut x,
            "y" => &mut y,
            "z" => &mut z,
            "heading" => &mut heading,
            "tilt" => &mut tilt,
            "time" => &mut time_h,
            "dir" => {
                dir = Some(vector(value)?);
                continue;
            }
            _ => return Err(format!("unknown key '{key}'")),
        };
        *slot = Some(number(key, value)?);
    }
    let place = match (lat, lon, x, y, z) {
        (Some(lat), Some(lon), None, None, None) => {
            if level != 10 {
                return Err("lat and lon work on the planet (L10) only".to_string());
            }
            if !(-90.0..=90.0).contains(&lat) {
                return Err(format!("lat={lat} must be between -90 and 90"));
            }
            let h = h.unwrap_or(DEFAULT_HEIGHT_M);
            if h < 0.0 {
                return Err(format!("h={h} must not be below the ground"));
            }
            ShotPlace::Planet { lat, lon, h }
        }
        (None, None, Some(x), Some(y), Some(z)) => {
            if h.is_some() {
                return Err("h works with lat and lon, not with x y z".to_string());
            }
            ShotPlace::Xyz([x, y, z])
        }
        _ => {
            return Err("give either lat and lon (L10 only) or all of x, y and z".to_string());
        }
    };
    let look = match (heading, tilt, dir) {
        (None, None, Some(d)) => ShotLook::Dir(d),
        (Some(heading), tilt, None) => {
            if !matches!(place, ShotPlace::Planet { .. }) && !figure {
                return Err("heading and tilt need lat and lon (use dir= with x y z)".to_string());
            }
            ShotLook::HeadingTilt {
                heading,
                tilt: tilt.unwrap_or(0.0),
            }
        }
        (None, None, None) => return Err("a look (heading or dir) is missing".to_string()),
        _ => return Err("give either heading (and tilt) or dir, not both".to_string()),
    };
    if figure {
        if level != 10 {
            return Err("figure works on the planet (L10) only".to_string());
        }
        if !matches!(place, ShotPlace::Planet { .. }) {
            return Err("figure needs lat and lon".to_string());
        }
        if !matches!(look, ShotLook::HeadingTilt { .. }) {
            return Err("figure needs a heading".to_string());
        }
    }
    let time_h = time_h.unwrap_or(0.0);
    if time_h < 0.0 {
        return Err(format!("time={time_h} must not be before the start"));
    }
    if planet.is_some() && level != 10 {
        return Err("planet= works at L10 only".to_string());
    }
    let spec = ShotSpec {
        name: name.to_string(),
        level,
        place,
        look,
        figure,
        overlay,
        time_h,
        planet,
        first_person,
        selected,
    };
    if planet.is_some() || selected.is_some() {
        let mut universe = crate::Universe(universe_core::nav::Universe::new(
            universe_core::nav::DEMO_SEED,
        ));
        if !crate::shot_place::open_fixture(&mut universe, &spec) {
            return Err("fixture planet or selected marker does not exist".to_string());
        }
    }
    Ok(spec)
}

/// Reads a whole shot file.
///
/// # Errors
///
/// The first wrong line, as `line N: <problem>`, or a file with no shot.
pub fn parse_shot_file(text: &str) -> Result<Vec<ShotSpec>, String> {
    let mut shots: Vec<ShotSpec> = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let number = index + 1;
        let shot = parse_line(line).map_err(|problem| format!("line {number}: {problem}"))?;
        if shots.iter().any(|s| s.name == shot.name) {
            return Err(format!("line {number}: name '{}' is used twice", shot.name));
        }
        shots.push(shot);
    }
    if shots.is_empty() {
        return Err("the shot file has no shot".to_string());
    }
    Ok(shots)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The spec's example file (#863 Spec v1).
    pub(crate) const EXAMPLE: &str = "\
# name        level  where                          look
cliff-sunset  L10    lat=12.5 lon=-40 h=2           heading=270 tilt=-5
probe-xyz     L10    x=1200000 y=6200000 z=-300000  dir=0,-1,0

walk-ridge    L10    lat=30 lon=10 figure           heading=90
galaxy-view   L4     x=0.1 y=0.2 z=3                dir=0,0,-1
";

    #[test]
    fn shot_file_takes_its_shots() {
        // C1 (#863 AC1): the example file reads as its four shots, in order,
        // with every value as written.
        let shots = parse_shot_file(EXAMPLE).expect("the example reads");
        let names: Vec<&str> = shots.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            ["cliff-sunset", "probe-xyz", "walk-ridge", "galaxy-view"]
        );
        assert_eq!(
            shots[0].place,
            ShotPlace::Planet {
                lat: 12.5,
                lon: -40.0,
                h: 2.0
            }
        );
        assert_eq!(
            shots[0].look,
            ShotLook::HeadingTilt {
                heading: 270.0,
                tilt: -5.0
            }
        );
        assert_eq!(
            shots[1].place,
            ShotPlace::Xyz([1_200_000.0, 6_200_000.0, -300_000.0])
        );
        assert_eq!(shots[1].look, ShotLook::Dir([0.0, -1.0, 0.0]));
        assert!(shots[2].figure);
        assert!(!shots[2].overlay);
        assert_eq!(shots[3].level, 4);
        // #918: `overlay` turns the F3 overlay on for the shot.
        let lines = parse_shot_file("o L10 lat=1 lon=2 overlay heading=0").expect("reads");
        assert!(lines[0].overlay);
        // #981 T6: `time=H` sets the sky's hours after the start; 0 by
        // default, never before the start.
        let timed = parse_shot_file("t L10 lat=1 lon=2 heading=0 time=9.5").expect("reads");
        assert_eq!(timed[0].time_h, 9.5);
        assert_eq!(lines[0].time_h, 0.0);
        assert_eq!(
            parse_shot_file("t L10 lat=1 lon=2 heading=0 time=-1"),
            Err("line 1: time=-1 must not be before the start".to_string())
        );
        // Height defaults to 2 m.
        let one = parse_shot_file("a L10 lat=1 lon=2 heading=0").expect("reads");
        assert_eq!(
            one[0].place,
            ShotPlace::Planet {
                lat: 1.0,
                lon: 2.0,
                h: DEFAULT_HEIGHT_M
            }
        );
    }

    #[test]
    fn wrong_shot_lines_are_refused() {
        // C5 (#863 AC5): every wrong line is refused with its number and
        // the problem, before anything runs.
        let cases = [
            (
                "a L10 lat=abc lon=1 heading=0",
                "line 1: lat=abc is not a number",
            ),
            (
                "a L11 x=1 y=2 z=3 dir=0,0,1",
                "line 1: level 'L11' must be L1 to L10",
            ),
            (
                "a L4 lat=1 lon=2 heading=0",
                "line 1: lat and lon work on the planet (L10) only",
            ),
            (
                "a L4 x=1 y=2 z=3 dir=0,0,1 figure",
                "line 1: figure works on the planet (L10) only",
            ),
            (
                "a L10 x=1 y=2 z=3 dir=0,0,1 figure",
                "line 1: figure needs lat and lon",
            ),
            (
                "a L10 lat=1 lon=2 heading=0 colour=red",
                "line 1: unknown key 'colour'",
            ),
            (
                "a L10 lat=1 lon=2 heading=0 wide",
                "line 1: unknown word 'wide'",
            ),
            (
                "a L10 lat=1 lon=2",
                "line 1: a look (heading or dir) is missing",
            ),
            (
                "a L10 lat=1 heading=0",
                "line 1: give either lat and lon (L10 only) or all of x, y and z",
            ),
            (
                "a L10 lat=95 lon=0 heading=0",
                "line 1: lat=95 must be between -90 and 90",
            ),
            (
                "a L10 x=1 y=2 z=3 dir=0,0,0",
                "line 1: dir=0,0,0 has no direction",
            ),
            (
                "a L10 lat=1 lon=2 heading=0 dir=0,0,1",
                "line 1: give either heading (and tilt) or dir, not both",
            ),
            (
                "../evil L10 lat=1 lon=2 heading=0",
                "line 1: name '../evil' must be 1-64 letters, digits, '-' or '_'",
            ),
            ("# only a comment", "the shot file has no shot"),
        ];
        for (text, want) in cases {
            assert_eq!(parse_shot_file(text), Err(want.to_string()), "{text}");
        }
        assert_eq!(
            parse_shot_file("a L10 lat=1 lon=2 heading=0\n\na L10 lat=3 lon=4 heading=0"),
            Err("line 3: name 'a' is used twice".to_string())
        );
    }
}
