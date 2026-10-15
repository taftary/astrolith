//! Type gallery tables (#685, T13): where to show each planet and moon
//! kind (`engine-lab --screens` pictures, one per kind).
//!
//! The lab shows planets by star path and planet marker, so a gallery
//! entry names all three under one fixed root (the demo seed the screens
//! mode starts from). The scan walks neighbouring star systems off the
//! journey prefix, opening each exactly the way the lab does, and resolves
//! every planet through the same framing the game uses
//! ([`crate::surface::SurfaceFrame::for_path`]); moon kinds ride the
//! lab's rank mapping (display rank `i` wears system moon `i`, as in
//! render). Rogues have no star to orbit and never come up. Everything is
//! deterministic (`E-DET-TIERS`): rescans always find the same tables.

use super::classify::WorldKind;
use super::moons::MoonKind;
use crate::r#gen::MarkerKind;
use crate::nav::{MarkerIndex, Universe};

/// Gallery root (the lab screens mode starts here).
pub const GALLERY_ROOT: u64 = crate::nav::DEMO_SEED;

// Breadth cap per tree level and star-cell cap (bound the ignored
// pin-search walk below).
#[cfg(test)]
const WALK_BREADTH: usize = 30;

/// Star cells collected at most (bounds the pin-search walk).
#[cfg(test)]
const STAR_CAP: usize = 15000;

/// One planet gallery entry: where to show the kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GalleryPlanet {
    /// The kind on show.
    pub kind: WorldKind,
    /// Star path (L1-L9 markers) under the gallery root.
    pub star: [u32; 8],
    /// Planet index in the star's list (the lab selects it).
    pub planet: usize,
}

/// One moon gallery entry: where to show the kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GalleryMoon {
    /// The kind on show.
    pub kind: MoonKind,
    /// Star path (L1-L9 markers) under the gallery root.
    pub star: [u32; 8],
    /// Planet index in the star's list.
    pub planet: usize,
    /// Terrain moon index wearing it (the lab shows it on its own).
    pub moon_index: usize,
}

/// Star paths scanned by the ignored `pin_scan` search: a portal tree
/// walk from the root with `generate_cell` only (no opens), collecting
/// L9 cells that hold planets.
#[cfg(test)]
fn star_paths() -> Vec<[u32; 8]> {
    use crate::r#gen::MarkerKind;
    let mut stars = Vec::new();
    let mut frontier: Vec<Vec<u32>> = vec![Vec::new()];
    for _ in 0..8 {
        if stars.len() >= STAR_CAP {
            break;
        }
        let mut next: Vec<Vec<u32>> = Vec::new();
        for chain in &frontier {
            if stars.len() >= STAR_CAP || next.len() >= STAR_CAP {
                break;
            }
            let cell = crate::nest::generate_cell(GALLERY_ROOT, chain);
            let mut portals = 0usize;
            for (index, point) in cell.points.iter().enumerate() {
                if portals >= WALK_BREADTH {
                    break;
                }
                if point.kind != MarkerKind::Portal {
                    continue;
                }
                let Ok(marker) = u32::try_from(index) else {
                    continue;
                };
                portals += 1;
                let mut child = chain.clone();
                child.push(marker);
                if child.len() == 8 {
                    stars.push(child);
                } else {
                    next.push(child);
                }
            }
        }
        frontier = next;
    }
    let mut paths: Vec<[u32; 8]> = Vec::new();
    for star in stars {
        let mut path = [0u32; 8];
        path.copy_from_slice(&star);
        paths.push(path);
    }
    paths.sort();
    paths.dedup();
    paths
}

/// Opens a star cell the way the lab does: the planet markers in list
/// order (`None` when the path reaches no star with planets).
fn open_star(star: &[u32; 8]) -> Option<Vec<u32>> {
    let mut universe = Universe::new(GALLERY_ROOT);
    for &marker in star.iter() {
        if !universe.open(MarkerIndex(marker)) {
            return None;
        }
    }
    let planets: Vec<u32> = universe
        .open
        .points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.kind == MarkerKind::Portal && point.planet.is_some())
        .filter_map(|(index, _)| u32::try_from(index).ok())
        .collect();
    if planets.is_empty() {
        return None;
    }
    Some(planets)
}

/// Recipe of one planet, framed exactly the way the game frames it.
fn planet_recipe(
    star: &[u32; 8],
    planet: u32,
) -> Option<(u64, super::WorldRecipe, crate::surface::SurfaceFrame)> {
    let mut chain = star.to_vec();
    chain.push(planet);
    let frame = crate::surface::SurfaceFrame::for_path(GALLERY_ROOT, &chain)?;
    let recipe = super::biome::recipe_for_frame(&frame)?;
    Some((frame.planet_seed(), recipe, frame))
}

/// Fixed planet gallery: one entry per star-bound planet kind (pinned
/// by the `pin_scan` search below; re-pin by running it ignored).
/// Rogue planets have no star to orbit.
pub const PLANET_GALLERY: [GalleryPlanet; 13] = [
    GalleryPlanet {
        kind: WorldKind::Iron,
        star: [0, 0, 0, 0, 2, 0, 0, 0],
        planet: 3,
    },
    GalleryPlanet {
        kind: WorldKind::EarthLike,
        star: [0, 0, 0, 1, 1, 0, 0, 0],
        planet: 1,
    },
    GalleryPlanet {
        kind: WorldKind::Desert,
        star: [0, 0, 0, 0, 2, 0, 0, 0],
        planet: 0,
    },
    GalleryPlanet {
        kind: WorldKind::Lava,
        star: [0, 0, 5, 6, 11, 0, 0, 0],
        planet: 0,
    },
    GalleryPlanet {
        kind: WorldKind::Ice,
        star: [0, 0, 0, 0, 1, 0, 0, 0],
        planet: 0,
    },
    GalleryPlanet {
        kind: WorldKind::Ocean,
        star: [0, 0, 5, 6, 5, 0, 0, 0],
        planet: 0,
    },
    GalleryPlanet {
        kind: WorldKind::Carbon,
        star: [0, 0, 0, 1, 3, 0, 0, 0],
        planet: 0,
    },
    GalleryPlanet {
        kind: WorldKind::SuperEarth,
        star: [0, 3, 11, 2, 2, 0, 0, 0],
        planet: 0,
    },
    GalleryPlanet {
        kind: WorldKind::Dwarf,
        star: [0, 0, 0, 0, 2, 0, 0, 0],
        planet: 4,
    },
    GalleryPlanet {
        kind: WorldKind::SubNeptune,
        star: [0, 0, 0, 0, 2, 0, 0, 0],
        planet: 1,
    },
    GalleryPlanet {
        kind: WorldKind::IceGiant,
        star: [0, 0, 0, 0, 0, 0, 0, 0],
        planet: 2,
    },
    GalleryPlanet {
        kind: WorldKind::GasGiant,
        star: [0, 0, 0, 0, 0, 0, 0, 0],
        planet: 0,
    },
    GalleryPlanet {
        kind: WorldKind::HotJupiter,
        star: [0, 0, 5, 6, 11, 0, 0, 0],
        planet: 1,
    },
];

/// Fixed moon gallery: one entry per moon kind (pinned by the `pin_scan`
/// search below; re-pin by running it ignored).
pub const MOON_GALLERY: [GalleryMoon; 7] = [
    GalleryMoon {
        kind: MoonKind::Dead,
        star: [0, 0, 0, 0, 2, 0, 0, 0],
        planet: 4,
        moon_index: 0,
    },
    GalleryMoon {
        kind: MoonKind::Volcanic,
        star: [0, 0, 0, 0, 0, 0, 0, 0],
        planet: 1,
        moon_index: 1,
    },
    GalleryMoon {
        kind: MoonKind::Ocean,
        star: [0, 0, 1, 7, 7, 0, 0, 0],
        planet: 2,
        moon_index: 1,
    },
    GalleryMoon {
        kind: MoonKind::Hazy,
        star: [0, 0, 0, 1, 1, 0, 0, 0],
        planet: 5,
        moon_index: 1,
    },
    GalleryMoon {
        kind: MoonKind::Captured,
        star: [0, 0, 0, 0, 0, 0, 0, 0],
        planet: 0,
        moon_index: 0,
    },
    GalleryMoon {
        kind: MoonKind::Shepherd,
        star: [0, 0, 0, 0, 0, 0, 0, 0],
        planet: 1,
        moon_index: 0,
    },
    GalleryMoon {
        kind: MoonKind::Ringed,
        star: [0, 0, 0, 0, 0, 0, 0, 0],
        planet: 0,
        moon_index: 3,
    },
];

/// Resolves a planet gallery entry through the game framing (the lab
/// shot shows this planet).
#[must_use]
pub fn resolve_planet(entry: &GalleryPlanet) -> Option<super::WorldRecipe> {
    let planets = open_star(&entry.star)?;
    let marker = planets.get(entry.planet).copied()?;
    planet_recipe(&entry.star, marker).map(|(_, recipe, _)| recipe)
}

/// Resolves a moon gallery entry: the planet recipe plus the kind worn
/// at the terrain moon index (`None` when either moved).
#[must_use]
pub fn resolve_moon(entry: &GalleryMoon) -> Option<(super::WorldRecipe, MoonKind)> {
    let planets = open_star(&entry.star)?;
    let marker = planets.get(entry.planet).copied()?;
    let (planet_seed, recipe, frame) = planet_recipe(&entry.star, marker)?;
    let mut chain = entry.star.to_vec();
    chain.push(marker);
    let cell = crate::nest::generate_cell(GALLERY_ROOT, &chain);
    let terrain: Vec<usize> = cell
        .points
        .iter()
        .enumerate()
        .filter(|(_, point)| point.moon.is_some())
        .map(|(rank, _)| rank)
        .collect();
    if entry.moon_index >= terrain.len() {
        return None;
    }
    let body = crate::terrain::body_recipe(planet_seed, frame.home());
    let spec = super::moons::PlanetSpec::from_recipe(&recipe, body.spin_hours);
    let system = super::moons::system_for(planet_seed, &spec);
    let moon = system.moons.get(entry.moon_index)?;
    Some((recipe, moon.kind))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "gallery pin search: run on demand, prints PIN lines for the tables above"]
    fn pin_scan() {
        // One-time gallery pin search (background): star-grid planets for
        // each planet kind, and terrain-mapped moons for each moon kind.
        // Prints PIN lines; the tables below pin the results.
        use std::collections::BTreeSet;
        let mut planet_missing: BTreeSet<&str> = WorldKind::ALL
            .iter()
            .filter(|kind| **kind != WorldKind::Rogue)
            .map(|kind| kind.label())
            .collect();
        let mut moon_missing: BTreeSet<&str> =
            MoonKind::ALL.iter().map(|kind| kind.label()).collect();
        let mut stars = 0u32;
        for star in star_paths() {
            if planet_missing.is_empty() && moon_missing.is_empty() {
                break;
            }
            let Some(planets) = open_star(&star) else {
                continue;
            };
            stars += 1;
            if stars.is_multiple_of(50) {
                eprintln!("scan {stars} planets={planet_missing:?} moons={moon_missing:?}");
            }
            for (index, marker) in planets.iter().enumerate() {
                let Some((planet_seed, recipe, frame)) = planet_recipe(&star, *marker) else {
                    continue;
                };
                if planet_missing.remove(recipe.kind.label()) {
                    eprintln!(
                        "PIN planet {:?} star {} planet {index}",
                        recipe.kind,
                        path_text(&star)
                    );
                }
                if moon_missing.is_empty() {
                    continue;
                }
                let mut chain = star.to_vec();
                chain.push(*marker);
                let cell = crate::nest::generate_cell(GALLERY_ROOT, &chain);
                let terrain: Vec<usize> = cell
                    .points
                    .iter()
                    .enumerate()
                    .filter(|(_, point)| point.moon.is_some())
                    .map(|(rank, _)| rank)
                    .collect();
                if terrain.is_empty() {
                    continue;
                }
                let body = crate::terrain::body_recipe(planet_seed, frame.home());
                let spec = super::super::moons::PlanetSpec::from_recipe(&recipe, body.spin_hours);
                let system = super::super::moons::system_for(planet_seed, &spec);
                for (rank, _) in terrain.iter().enumerate() {
                    let Some(moon) = system.moons.get(rank) else {
                        break;
                    };
                    if moon_missing.remove(moon.kind.label()) {
                        eprintln!(
                            "PIN moon {:?} star {} planet {index} moon {rank}",
                            moon.kind,
                            path_text(&star)
                        );
                    }
                }
            }
        }
        eprintln!("DONE stars={stars} planets={planet_missing:?} moons={moon_missing:?}");
    }

    fn path_text(star: &[u32; 8]) -> String {
        star.iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join("/")
    }

    #[test]
    fn planet_gallery_covers_every_star_bound_kind() {
        // The pinned table holds one entry per star-bound kind, each
        // re-resolving through the game framing (re-pin by running
        // `pin_scan` ignored when this fails).
        assert_eq!(PLANET_GALLERY.len(), WorldKind::ALL.len() - 1);
        for (index, entry) in PLANET_GALLERY.iter().enumerate() {
            assert_eq!(entry.kind, WorldKind::ALL[index], "order moved");
        }
        for entry in PLANET_GALLERY {
            let recipe = resolve_planet(&entry).expect("gallery entry broke");
            assert_eq!(recipe.kind, entry.kind, "entry moved");
        }
    }

    #[test]
    fn moon_gallery_covers_every_moon_kind() {
        // The pinned table holds one entry per moon kind, each
        // re-resolving through the game framing (re-pin by running
        // `pin_scan` ignored when this fails).
        assert_eq!(MOON_GALLERY.len(), MoonKind::ALL.len());
        for (index, entry) in MOON_GALLERY.iter().enumerate() {
            assert_eq!(entry.kind, MoonKind::ALL[index], "order moved");
        }
        for entry in MOON_GALLERY {
            let Some((_, kind)) = resolve_moon(&entry) else {
                panic!("gallery entry broke: {entry:?}");
            };
            assert_eq!(kind, entry.kind, "entry moved");
        }
    }
}
