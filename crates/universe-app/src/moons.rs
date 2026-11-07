//! Thin headless `--list-moons` driver for unchanged generated objects (#1238).
//!
//! Catalog properties are not contact measurements or evidence of player travel.

use universe_core::moon_catalog::{CATALOG_MOONS, CATALOG_PLANETS, list_moons};
use universe_core::nav::Seed;

use crate::protocol::{EXIT_FAIL, EXIT_PASS, EXIT_USAGE};

/// Strict bounded catalog request; no profile, rendering or file I/O.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CatalogRequest {
    seed: Seed,
    prefix: Vec<u32>,
    planets: usize,
    moons: usize,
}

/// Dot-separated chain, or `root` for an empty prefix. Depth is checked here;
/// existence and marker kind are checked by the core against actual generation.
fn prefix(text: &str) -> Option<Vec<u32>> {
    if text == "root" {
        return Some(Vec::new());
    }
    let chain: Vec<_> = text
        .split('.')
        .map(str::parse::<u32>)
        .collect::<Result<_, _>>()
        .ok()?;
    (chain.len() <= 9).then_some(chain)
}

impl CatalogRequest {
    /// Empty arguments list the same near-home prefix as `--list-worlds`.
    pub(crate) fn parse(args: &[String]) -> Option<Self> {
        match args {
            [] => {
                let seed = Seed(universe_core::nav::DEMO_SEED);
                let journey = universe_core::nest::autopilot_path(seed.0);
                Some(Self {
                    seed,
                    prefix: journey.get(..3)?.to_vec(),
                    planets: CATALOG_PLANETS,
                    moons: CATALOG_MOONS,
                })
            }
            [seed, start, planets, moons] => {
                let planets = planets.parse().ok()?;
                let moons = moons.parse().ok()?;
                if !(1..=CATALOG_PLANETS).contains(&planets)
                    || !(1..=CATALOG_MOONS).contains(&moons)
                {
                    return None;
                }
                Some(Self {
                    seed: Seed(seed.parse().ok()?),
                    prefix: prefix(start)?,
                    planets,
                    moons,
                })
            }
            _ => None,
        }
    }
}

/// Prints actual moon records plus separately labeled boundary-only data.
pub(crate) fn run(args: &[String]) -> i32 {
    let Some(request) = CatalogRequest::parse(args) else {
        eprintln!("--list-moons takes no values or <seed> <prefix|root> <planets> <moons>");
        return EXIT_USAGE;
    };
    let moons = match list_moons(
        request.seed,
        &request.prefix,
        request.planets,
        request.moons,
    ) {
        Ok(moons) => moons,
        Err(reason) => {
            println!("MOONS-FAIL {reason}");
            return EXIT_FAIL;
        }
    };
    println!("{}", crate::protocol::moon_boundary());
    for moon in &moons {
        println!("{}", crate::protocol::moon_line(moon));
    }
    println!(
        "MOONS-OK count={} planet_limit={} moon_limit={} scope=bounded-not-global-minimum",
        moons.len(),
        request.planets,
        request.moons
    );
    EXIT_PASS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn catalog_requests_are_bounded_and_default_to_near_home() {
        let default = CatalogRequest::parse(&[]).unwrap();
        assert_eq!(default.seed, Seed(42));
        assert_eq!(default.prefix, universe_core::nest::autopilot_path(42)[..3]);
        let explicit = CatalogRequest::parse(&args(&["99", "root", "2", "3"])).unwrap();
        assert_eq!(explicit.prefix, []);
        assert_eq!(
            (explicit.seed, explicit.planets, explicit.moons),
            (Seed(99), 2, 3)
        );
        assert_eq!(prefix("1.2.3"), Some(vec![1, 2, 3]));
        for invalid in [
            &["42"][..],
            &["42", "root", "0", "2"],
            &["42", "root", "4001", "2"],
            &["42", "root", "1", "64001"],
            &["42", "1..2", "1", "2"],
            &["42", "1.2.3.4.5.6.7.8.9.0", "1", "2"],
            &["-1", "root", "1", "2"],
        ] {
            assert!(
                CatalogRequest::parse(&args(invalid)).is_none(),
                "{invalid:?}"
            );
        }
    }

    #[test]
    fn protocol_preserves_identity_units_and_separate_property_terrain_origins() {
        let host = universe_core::moon_catalog::PlanetIdentity::new(
            Seed(42),
            &universe_core::nest::autopilot_path(42),
        )
        .unwrap();
        let moon = universe_core::moon_catalog::moons_for_planet(host).unwrap()[0];
        let line = crate::protocol::moon_line(&moon);
        assert!(line.starts_with("MOON\tseed=42\tplanet="));
        assert!(line.contains("\tsystem="));
        assert!(line.contains("\tmoon="));
        assert!(line.contains("\tradius_km="));
        assert!(line.contains("\tinstance_origin=catalog-based\t"));
        assert!(line.contains("\tterrain_origin=synthetic\t"));
        assert!(line.contains("\tproperty_origin=supplied-reference-not-measured\t"));
        assert!(line.contains("\tcontact=unavailable:not-a-contact-observation"));
        assert!(!line.contains("boundary-only"));
        assert!(
            crate::protocol::moon_boundary().starts_with("MOON-BOUNDARY\torigin=boundary-only")
        );
    }
}
