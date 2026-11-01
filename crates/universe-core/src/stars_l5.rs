//! L5 holds stars at their real spacing (#996, Spec v5 Goal 4).
//!
//! An L5 cell is 100 pc across (ladder R5), so its markers are stars, each
//! with the neighbourhood of an L6 cell around it. Real stars near the Sun
//! are 0.14 per cubic parsec (study source R9), about 1.06 pc apart, so a
//! whole cell would hold about 140,000: more than any Marker detail level
//! draws. The cell holds the [`crate::stars_l5::CORE_STARS`] nearest its centre, where every
//! dive enters, at that real density: a ball about 40 pc across (20 pc radius), nearest
//! first. The bright stars, classes A, B and O (about one in 140 of the
//! observed mix), can be entered: their neighbourhoods are farther apart
//! than a neighbourhood's opening distance, which an ordinary star's
//! (about 1 pc) is not. On the home journey the
//! Sun sits at the centre and Alpha Centauri 1.34 pc away is the pick.
//! Pure (`E-CORE-NO-BEVY`, `E-DET-TIERS`).

use crate::coords::Level;
use crate::r#gen::{Constraints, Form, Generated, MarkerKind, Point, StarInfo, StarKind};
use crate::layouts::star_radius;
use crate::nest::scale_exponent;
use crate::seed::{Rng, hash_triple};
use crate::system::{ALPHA_CEN_TRIPLE, STAR_STREAM_TAG, sample_star, star_tint};

/// Level whose markers are stars.
pub const STAR_LEVEL: u8 = 5;

/// Stars per cubic parsec near the Sun (study R9).
pub const STARS_PER_PC3: f64 = 0.14;

/// Stars the cell holds around its centre: the High Marker detail budget.
pub const CORE_STARS: u32 = 5000;

/// Parsec in metres.
const PC_M: f64 = 3.0857e16;

/// Alpha Centauri's distance from the Sun, 1.34 pc.
pub const ALPHA_CEN_PC: f64 = 1.34;

/// Cell size in parsecs.
fn cell_pc() -> f64 {
    10f64.powf(scale_exponent(Level::new(STAR_LEVEL).unwrap_or(Level::MIN))) / PC_M
}

/// Radius of the ball holding [`CORE_STARS`] at the real density, in cells.
#[must_use]
pub fn core_radius_cells() -> f64 {
    let volume = f64::from(CORE_STARS) / STARS_PER_PC3;
    (volume * 3.0 / (4.0 * std::f64::consts::PI)).cbrt() / cell_pc()
}

/// Stars of the L5 cell seeded `seed`; `home` puts the Sun at the centre
/// and Alpha Centauri at `home_pick`.
#[must_use]
pub fn generate(seed: u64, parent: &Constraints, home_pick: Option<u32>) -> Generated {
    let total = CORE_STARS.min(parent.max_count) as usize;
    let radius = core_radius_cells();
    let mut points: Vec<Point> = (0..total)
        .map(|index| {
            let mut rng = Rng::new(hash_triple(seed, STAR_STREAM_TAG, index as u64));
            // Uniform in the ball: a random direction and radius^(1/3).
            let z = rng.next_f64() * 2.0 - 1.0;
            let angle = rng.next_f64() * std::f64::consts::TAU;
            let ring = (1.0 - z * z).max(0.0).sqrt();
            let r = radius * rng.next_f64().cbrt();
            let star = sample_star(&mut rng);
            star_point(
                [ring * angle.cos() * r, ring * angle.sin() * r, z * r],
                star,
            )
        })
        .collect();
    // Nearest the centre first, so the order reads outward (E-FLOAT-SORT).
    points.sort_by(|a, b| norm(a.position).total_cmp(&norm(b.position)));
    let bright: Vec<usize> = points
        .iter()
        .enumerate()
        .filter(|(_, p)| is_bright(p))
        .map(|(i, _)| i)
        .collect();
    for &index in &bright {
        if let Some(point) = points.get_mut(index) {
            point.kind = MarkerKind::Portal;
        }
    }
    if let Some(pick) = home_pick {
        place_home(seed, &mut points, pick, &bright);
    }
    let child = Constraints {
        density_multiplier: parent.density_multiplier / 2.0,
        min_count: parent.min_count.min(parent.max_count / 2),
        max_count: parent.max_count / 2,
        allowed_extent: parent.allowed_extent,
    };
    Generated {
        points,
        child_constraints: vec![child; 8],
        ids: Vec::new(),
    }
}

/// The Sun at the centre and Alpha Centauri at `pick`, 1.34 pc out; the
/// journey keeps its recorded marker.
fn place_home(seed: u64, points: &mut [Point], pick: u32, bright: &[usize]) {
    let sun = StarInfo {
        kind: StarKind::G,
        mass_solar: 1.0,
    };
    if let Some(first) = points.first_mut() {
        *first = Point {
            kind: first.kind,
            ..star_point([0.0, 0.0, 0.0], sun)
        };
    }
    crate::home::pin_journey_pick_ranked(seed, points, pick, bright.len(), bright);
    let [(kind, mass_solar), _, _] = ALPHA_CEN_TRIPLE;
    let out = ALPHA_CEN_PC / cell_pc();
    if let Some(alpha) = points.get_mut(pick as usize) {
        *alpha = Point {
            kind: MarkerKind::Portal,
            ..star_point([out, 0.0, 0.0], StarInfo { kind, mass_solar })
        };
    }
}

fn star_point(position: [f64; 3], star: StarInfo) -> Point {
    // A star's drawn size from the L6 rule, in L5 cells.
    let scale = 10f64.powf(
        scale_exponent(Level::new(STAR_LEVEL + 1).unwrap_or(Level::MIN))
            - scale_exponent(Level::new(STAR_LEVEL).unwrap_or(Level::MIN)),
    );
    let radius = star_radius(star.mass_solar) * scale;
    Point {
        position,
        radius,
        kind: MarkerKind::Population,
        form: Form::Dot,
        star: Some(star),
        tint: Some(star_tint(star.kind)),
        ..Point::bare(position, radius, MarkerKind::Population)
    }
}

fn is_bright(point: &Point) -> bool {
    point
        .star
        .is_some_and(|s| matches!(s.kind, StarKind::A | StarKind::B | StarKind::O))
}

fn norm(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

#[cfg(test)]
#[path = "stars_l5_tests.rs"]
mod tests;
