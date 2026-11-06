//! `--list-worlds`: the worlds near home and their surface types (#1161 T1).
//!
//! Prints one line per L10 planet, depth first under the home journey's L3
//! cell (the same order as [`universe_core::world_type::list_worlds`]), so shot
//! lists and routes can name a world of each type with `chain=`. Headless and
//! deterministic: the same build prints the same lines.

use universe_core::world_type::{ListedWorld, list_worlds};

/// How many worlds the listing walks (enough to hold every type).
pub(crate) const LISTED_WORLDS: usize = 4_000;

/// One listing line for `world`.
pub(crate) fn world_line(world: &ListedWorld) -> String {
    let chain: Vec<String> = world.chain.iter().map(u32::to_string).collect();
    format!(
        "chain={} type={} t_eq_k={:.0} radius_earth={:.3} orbit_au={:.3} star_mass_solar={:.3} air={}",
        chain.join("."),
        world.kind.map_or("giant", |kind| kind.name()),
        world.t_eq_k,
        world.inputs.radius_earth,
        world.inputs.orbit_au,
        world.inputs.star_mass_solar,
        world.inputs.has_air,
    )
}

/// Prints the listing and a `WORLDS-OK` count line; exit code 0.
pub(crate) fn run_list_worlds() -> i32 {
    let root = universe_core::nav::DEMO_SEED;
    let journey = universe_core::nest::autopilot_path(root);
    let start = journey.get(..3).unwrap_or(&[]);
    let worlds = list_worlds(root, start, LISTED_WORLDS);
    for world in &worlds {
        println!("{}", world_line(world));
    }
    println!("WORLDS-OK count={}", worlds.len());
    0
}

/// Sample directions `--list-water` scans on one world.
const WATER_SAMPLES: usize = 200_000;
/// Places of each kind `--list-water` prints.
const WATER_PER_KIND: usize = 6;

/// The listing line for one water place.
pub(crate) fn water_line(place: &universe_core::water_places::WaterPlace) -> String {
    let mut line = format!(
        "place={} lat={:.5} lon={:.5} h_m={:.1}",
        place.kind.name(),
        place.lat_deg,
        place.lon_deg,
        place.height_m
    );
    if let Some(bearing) = place.sea_bearing_deg {
        line.push_str(&format!(" sea_heading={bearing:.0}"));
    }
    line
}

/// Prints the water places of the world at `chain` (dot-separated L10
/// indices) and a `WATER-OK` count line; exit code 0, or 2 for a bad chain.
pub(crate) fn run_list_water(chain: &str) -> i32 {
    let parsed: Option<Vec<u32>> = chain.split('.').map(|p| p.parse().ok()).collect();
    let Some(chain) = parsed.filter(|c| c.len() == 9) else {
        eprintln!("--list-water needs an L10 chain like 7.2.7.1.2.0.0.0.5");
        return crate::protocol::EXIT_USAGE;
    };
    let root = universe_core::nav::DEMO_SEED;
    let relief = universe_core::relief::Relief::for_planet(root, &chain);
    let places = universe_core::water_places::water_places(&relief, WATER_SAMPLES, WATER_PER_KIND);
    for place in &places {
        println!("{}", water_line(place));
    }
    println!("WATER-OK count={}", places.len());
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_listing_line_names_the_chain_and_type() {
        let root = universe_core::nav::DEMO_SEED;
        let journey = universe_core::nest::autopilot_path(root);
        let system = journey.get(..8).expect("home system");
        let worlds = list_worlds(root, system, 32);
        let home = worlds.iter().find(|w| w.inputs.home).expect("home listed");
        let line = world_line(home);
        assert!(line.starts_with("chain="), "{line}");
        assert!(line.contains(" type=earth-like "), "{line}");
        assert!(line.contains(" air=true"), "{line}");
    }
}
