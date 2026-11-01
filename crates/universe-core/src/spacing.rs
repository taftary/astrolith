//! Gap between neighbouring markers against the real gap (#996, Spec v5).
//!
//! The study's method (`guides/universe/documents/marker-spacing.md`, §1):
//! for every marker of a cell, the distance to its nearest neighbour; the
//! median, in cells and in metres (a level-`l` cell is `10^e_l` metres
//! across, [`scale_exponent`]). Real gaps and counts are the study's §2
//! values, each with its source there. A level is checked against its real
//! target once its content has landed; until then its line says `pending`
//! and does not fail the check. L1 and L7 are pinned to their values before
//! #996, which must not change (AC11). Pure (`E-CORE-NO-BEVY`,
//! `E-DET-TIERS`).

use crate::coords::Level;
use crate::r#gen::{MarkerKind, Point};
use crate::home::autopilot_path;
use crate::nav::DEMO_SEED;
use crate::nest::{generate_cell, path_level, scale_exponent};

/// Deepest level the probe measures (L8 to L10 are true scale, #995).
pub const SPACING_DEEPEST: u8 = 7;

/// Allowed factor between the game and the real value (Spec v5 AC1, AC2).
pub const SPACING_FACTOR: f64 = 2.0;

/// Spacing of one cell's markers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spacing {
    /// Level of the cell.
    pub level: Level,
    /// Markers in the cell.
    pub count: usize,
    /// Portal markers in the cell.
    pub portals: usize,
    /// Median nearest-neighbour gap, in cells (0 with under two markers).
    pub median_cells: f64,
    /// The same gap in metres.
    pub median_m: f64,
}

/// What a level is checked against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Check {
    /// Content not landed yet: printed, never failing.
    Pending,
    /// Must stay as it was before #996: count and median in cells.
    Unchanged {
        /// Marker count before #996.
        count: usize,
        /// Median gap in cells before #996 (four decimals).
        median_cells: f64,
    },
    /// A marker count in a range (a cell this small holds about one system
    /// with its companions).
    Count {
        /// Fewest markers.
        min: usize,
        /// Most markers.
        max: usize,
    },
    /// Real median gap and count, each within [`SPACING_FACTOR`].
    Real {
        /// Real median gap in metres.
        gap_m: f64,
        /// Real count in a cell this size.
        count: f64,
    },
}

/// The check for `level` on the journey cell.
#[must_use]
pub fn check_for(level: Level) -> Check {
    match level.get() {
        1 => Check::Unchanged {
            count: 8,
            median_cells: 0.5,
        },
        // Study §2: rich clusters, Abell richness 0 and up (R4).
        2 => Check::Real {
            gap_m: 1.06e24,
            count: 300.0,
        },
        // The home cell: the Local Group and its neighbours, 100 galaxies in
        // a 4.6 Mpc cell, scattered at random (study §2 method).
        3 => Check::Real {
            gap_m: 1.35e22,
            count: 100.0,
        },
        // Study §2: the Milky Way's clouds in a thin disk (R7, R8).
        4 => Check::Real {
            gap_m: 4.4e18,
            count: 8107.0,
        },
        // Study §2: 0.35 stars in a 1.35 pc cell besides its own (R9).
        6 => Check::Count { min: 1, max: 3 },
        7 => Check::Unchanged {
            count: 32,
            median_cells: 0.1185,
        },
        _ => Check::Pending,
    }
}

/// Measures the nearest-neighbour spacing of `points` in a `level` cell.
#[must_use]
pub fn measure(level: Level, points: &[Point]) -> Spacing {
    let mut gaps: Vec<f64> = points
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            points
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, q)| distance(p.position, q.position))
                .min_by(f64::total_cmp)
        })
        .collect();
    gaps.sort_by(f64::total_cmp);
    let median_cells = median(&gaps);
    Spacing {
        level,
        count: points.len(),
        portals: points
            .iter()
            .filter(|p| p.kind == MarkerKind::Portal)
            .count(),
        median_cells,
        median_m: median_cells * 10f64.powf(scale_exponent(level)),
    }
}

/// Spacing of the journey's cells from L1 to [`SPACING_DEEPEST`].
#[must_use]
pub fn journey_spacing(root: u64) -> Vec<Spacing> {
    let path = autopilot_path(root);
    (0..usize::from(SPACING_DEEPEST))
        .filter_map(|depth| path.get(..depth))
        .map(|chain| measure(path_level(chain), &generate_cell(root, chain).points))
        .collect()
}

/// The rich L3 cell next to the journey: entered through the densest
/// portal of the journey's L2 cell (Virgo-like, study §2 and R5).
#[must_use]
pub fn rich_spacing(root: u64) -> Option<Spacing> {
    let parent = autopilot_path(root).get(..1)?.to_vec();
    let cell = generate_cell(root, &parent);
    let seed = crate::nest::path_seed(root, &parent);
    let anchor = parent.last().copied().map_or(0, i64::from);
    let densest = crate::density::densest_portal_index(seed, anchor, &cell.points)?;
    let mut chain = parent;
    chain.push(densest);
    Some(measure(
        path_level(&chain),
        &generate_cell(root, &chain).points,
    ))
}

/// The real target of the rich L3 cell (Virgo: 2,000 galaxies, R5).
pub const RICH_CHECK: Check = Check::Real {
    gap_m: 5.0e21,
    count: 2000.0,
};

/// Whether `spacing` meets `check`.
#[must_use]
pub fn passes(spacing: &Spacing, check: Check) -> bool {
    match check {
        Check::Pending => true,
        Check::Unchanged {
            count,
            median_cells,
        } => spacing.count == count && (spacing.median_cells - median_cells).abs() < 5e-5,
        Check::Count { min, max } => (min..=max).contains(&spacing.count),
        Check::Real { gap_m, count } => {
            within(spacing.median_m, gap_m) && within(count_f64(spacing.count), count)
        }
    }
}

/// Appends one `SPACING` line per journey level; `true` when all pass.
pub fn verify_spacing(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let mut ok = true;
    let rows = journey_spacing(DEMO_SEED)
        .into_iter()
        .map(|s| (s, check_for(s.level), ""))
        .chain(rich_spacing(DEMO_SEED).map(|s| (s, RICH_CHECK, " cell=rich")));
    for (spacing, check, tag) in rows {
        let passed = passes(&spacing, check);
        ok &= passed;
        let (target, word) = match check {
            Check::Pending => ("pending".to_string(), "pending"),
            Check::Unchanged { .. } => ("unchanged".to_string(), pass_word(passed)),
            Check::Count { min, max } => (format!("count_in={min}-{max}"), pass_word(passed)),
            Check::Real { gap_m, count } => (
                format!("real_m={gap_m:.2e} real_count={count:.0}"),
                pass_word(passed),
            ),
        };
        let _ = writeln!(
            out,
            "SPACING level=L{}{tag} count={} portals={} median_cells={:.4} median_m={:.3e} {target} {word}",
            spacing.level.get(),
            spacing.count,
            spacing.portals,
            spacing.median_cells,
            spacing.median_m,
        );
    }
    ok
}

fn within(game: f64, real: f64) -> bool {
    game > 0.0 && real > 0.0 && (game / real).max(real / game) <= SPACING_FACTOR
}

fn count_f64(count: usize) -> f64 {
    f64::from(u32::try_from(count).unwrap_or(u32::MAX))
}

fn median(sorted: &[f64]) -> f64 {
    match sorted.len() {
        0 => 0.0,
        n if n % 2 == 1 => sorted.get(n / 2).copied().unwrap_or(0.0),
        n => {
            let low = sorted.get(n / 2 - 1).copied().unwrap_or(0.0);
            let high = sorted.get(n / 2).copied().unwrap_or(0.0);
            (low + high) / 2.0
        }
    }
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

fn pass_word(passed: bool) -> &'static str {
    if passed { "PASS" } else { "FAIL" }
}

#[cfg(test)]
#[path = "spacing_tests.rs"]
mod tests;
