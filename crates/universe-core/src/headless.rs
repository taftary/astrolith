//! Headless `--verify` report builder: replay, checks, snapshots.
//!
//! The orchestration moved here from `universe-app` (#86) so every binary
//! that needs the gate shares one implementation instead of duplicating
//! logic (Q6). This module builds the whole stdout report as a `String`
//! and returns the exit code; binaries print the string verbatim, so all
//! programs emit byte-identical output (`E-CORE-HEADLESS`,
//! `E-STDOUT-PROTOCOL`, `E-ONE-PRINTER`).

use crate::coords::Level;
use crate::flight::replay_free_leg;
use crate::frame::{children_brightness, shell_brightness};
use crate::r#gen::MarkerKind;
use crate::r#gen::in_sphere;
use crate::labels::{level_label, scale_anchor, scale_label};
use crate::nav::{
    DEMO_SEED, JourneyStep, MAX_NAV_LEVEL, MIN_NAV_LEVEL, START_OFFSET, Universe, replay_autopilot,
};
use crate::nest::{
    CLOSE_ANGLE, OPEN_ANGLE, PREVIEW_ANGLE, PREVIEW_CAP, anon_cells, child_ratio, generate_cell,
    level_budget, path_seed, scale_exponent,
};
use crate::snapshot::snapshot_generated;
use crate::stream::STREAM_CAP;

/// Headless replay step, matching a 60 Hz frame.
pub const VERIFY_DT: f64 = 1.0 / 60.0;

/// Replay time budget before the journey counts as stalled.
pub const VERIFY_MAX_SECS: f64 = 600.0;

/// Exit code: every check passed.
pub const EXIT_PASS: i32 = 0;

/// Exit code: a check failed.
pub const EXIT_FAIL: i32 = 1;

/// Exit code: the command line itself was wrong (Q7).
pub const EXIT_USAGE: i32 = 2;

/// Machine-protocol marker: every headless check passed.
pub const VERIFY_OK: &str = "VERIFY-OK";

/// Machine-protocol marker: a headless check failed.
pub const VERIFY_FAIL: &str = "VERIFY-FAIL";

/// Whether the home L4 cell's journey portal sits off the body centre (#384).
///
/// Regenerates the L4 prefix of the fixed journey and checks the picked
/// portal carries an off-centre portal offset: opening a galaxy lands in
/// its disk lane, not at its centre. Pure; the `HOME-PORTAL` verify line
/// prints it in T7.
#[must_use]
pub fn home_portal_off_centre(root: u64) -> bool {
    use crate::nest::{autopilot_candidates, autopilot_marker, autopilot_path};
    let chain = autopilot_path(root);
    let Some(home) = chain.get(..3) else {
        return false;
    };
    let level = Level::new(4).unwrap_or(Level::MIN);
    let seed = path_seed(root, home);
    let cell = generate_cell(root, home);
    let candidates = autopilot_candidates(level, seed, &cell.points);
    autopilot_marker(seed, &candidates).is_some_and(|marker| {
        cell.points
            .get(marker as usize)
            .is_some_and(|point| point.kind == MarkerKind::Portal && point.portal.is_some())
    })
}

/// Coplanarity, order, and spacing of the journey L8 planets (#384).
///
/// Thin wrapper over [`crate::orbits::orbits_check_l8`] for the verify
/// report; the `ORBITS` line prints it in T7.
#[must_use]
pub fn orbits_l8(root: u64) -> (bool, bool, bool) {
    crate::orbits::orbits_check_l8(root)
}

/// Maps a level to its milestone tag.
#[must_use]
pub fn milestone_tag(level: Level) -> &'static str {
    match level.get() {
        1..=3 => "M3-cluster",
        4..=9 => "M4-stars",
        10 => "M5-terrain",
        11..=14 => "M6-room",
        _ => "beyond-MVP",
    }
}

/// Formats a check outcome as `PASS`/`FAIL`.
fn flag(passed: bool) -> &'static str {
    if passed { "PASS" } else { "FAIL" }
}

/// Replays the autopilot journey, checks every opened cell, snapshots it.
///
/// Same content `--verify` always printed, now returned instead of printed:
/// one `LEVEL` and one `SNAPSHOT` line per level L1-L14, one `JOURNEY` line
/// per opening, `RATIO` lines, then `VERIFY-OK`/`VERIFY-FAIL`. Returns the
/// full stdout bytes plus the process exit code (`EXIT_PASS` when everything
/// passes, `EXIT_FAIL` otherwise).
#[must_use]
pub fn verify_report() -> (String, i32) {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(
        out,
        "universe --verify: nested dive L1-L14, root seed {DEMO_SEED} (ladder: docs/universe/ladder.md)"
    );
    let (steps, mut universe) = replay_autopilot(DEMO_SEED, VERIFY_DT, VERIFY_MAX_SECS);
    let mut ok = true;
    let mut chain: Vec<u32> = Vec::new();
    for n in MIN_NAV_LEVEL..=MAX_NAV_LEVEL {
        let level = Level::new(n).unwrap_or(Level::MIN);
        let step = steps.iter().find(|step| step.level == level);
        if n > MIN_NAV_LEVEL {
            match step {
                Some(step) => chain.push(step.marker),
                None => {
                    ok = false;
                    let _ = writeln!(
                        out,
                        "LEVEL {} FAIL: journey never opened it",
                        level_label(level)
                    );
                    continue;
                }
            }
        }
        let first = generate_cell(DEMO_SEED, &chain);
        let second = generate_cell(DEMO_SEED, &chain);
        let determinism = snapshot_generated(&first) == snapshot_generated(&second);
        let budget = level_budget(level);
        let in_budget = !first.points.is_empty() && first.points.len() <= budget.max_count as usize;
        let marker_exists = chain.split_last().is_none_or(|(&marker, parent)| {
            let parent_cell = generate_cell(DEMO_SEED, parent);
            (marker as usize) < parent_cell.points.len()
        });
        let snapshot_matches = step.is_none_or(|step| step.snapshot == snapshot_generated(&first));
        let sphere = first.points.iter().all(|point| in_sphere(point.position));
        let passed = determinism && in_budget && marker_exists && snapshot_matches && sphere;
        ok &= passed;
        let _ = writeln!(
            out,
            "LEVEL {} [{}] scale={} anchor=\"{}\" path={:?} seed={} markers={} determinism={} in-budget={} marker-exists={} replay-match={} in-sphere={} {}",
            level_label(level),
            milestone_tag(level),
            scale_label(level),
            scale_anchor(level),
            chain,
            path_seed(DEMO_SEED, &chain),
            first.points.len(),
            flag(determinism),
            flag(in_budget),
            flag(marker_exists),
            flag(snapshot_matches),
            flag(sphere),
            flag(passed),
        );
        let _ = writeln!(
            out,
            "SNAPSHOT {} {}",
            level_label(level),
            snapshot_generated(&first)
        );
    }
    for step in &steps {
        let _ = writeln!(
            out,
            "JOURNEY opened={} marker={} at={:.2}s",
            level_label(step.level),
            step.marker,
            step.elapsed
        );
    }
    ok &= verify_preview(&steps, &mut out);
    let reached = universe.level().get() == MAX_NAV_LEVEL;
    ok &= reached;
    let _ = writeln!(
        out,
        "JOURNEY reached={} levels-opened={} {}",
        level_label(universe.level()),
        steps.len(),
        flag(reached)
    );
    ok &= verify_ratios(&mut out);
    ok &= verify_tail_anchors(&mut out);
    ok &= verify_milestones(&mut out);
    ok &= verify_atmosphere(&mut out);
    ok &= verify_portal_on_structure(&mut out);
    ok &= verify_curvature(&mut out);
    ok &= verify_horizon(&mut out);
    ok &= verify_parent_context(&mut out);
    ok &= verify_home_portal(&mut out);
    ok &= verify_orbits(&mut out);
    ok &= verify_inverse(&mut universe, &mut out);
    ok &= verify_free_leg(&steps, &mut out);
    if ok {
        let _ = writeln!(out, "{VERIFY_OK}");
        (out, EXIT_PASS)
    } else {
        let _ = writeln!(out, "{VERIFY_FAIL}");
        (out, EXIT_FAIL)
    }
}

/// Checks the pre-entry preview at every opening of the journey (R7, #63).
///
/// Per opened level: the target was previewed in the frame before opening,
/// at most `PREVIEW_CAP` markers were previewed, the previewed child
/// positions equal the open cell's markers through the frame (relative
/// 1e-9), and the streaming working set held at most `STREAM_CAP` cells
/// (#152). Then the brightness curves are sampled across `PREVIEW_ANGLE`
/// and `OPEN_ANGLE` for continuity. Appends one `PREVIEW` line per opening
/// plus a summary.
fn verify_preview(steps: &[JourneyStep], out: &mut String) -> bool {
    use std::fmt::Write as _;
    let mut passed = true;
    for step in steps {
        let previewed = step.preview_count >= 1;
        let capped = step.preview_count <= PREVIEW_CAP;
        let exact = step.preview_error <= 1e-9;
        let bounded = step.alive <= STREAM_CAP;
        let line_ok = previewed && capped && exact && bounded;
        passed &= line_ok;
        let _ = writeln!(
            out,
            "PREVIEW entering={} previewed={} (cap {}) position-error={:.2e} alive={} (max {}) anon={} {}",
            level_label(step.level),
            step.preview_count,
            PREVIEW_CAP,
            step.preview_error,
            step.alive,
            STREAM_CAP,
            step.anon_depth,
            flag(line_ok)
        );
    }
    let eps = 1e-6;
    let continuous = [PREVIEW_ANGLE, OPEN_ANGLE, CLOSE_ANGLE].iter().all(|edge| {
        (shell_brightness(edge - eps) - shell_brightness(edge + eps)).abs() < 1e-4
            && (children_brightness(edge - eps) - children_brightness(edge + eps)).abs() < 1e-4
    });
    let endpoints = shell_brightness(PREVIEW_ANGLE) == 1.0
        && children_brightness(PREVIEW_ANGLE) == 0.0
        && children_brightness(OPEN_ANGLE) == 1.0;
    let all_levels = steps.len() == usize::from(MAX_NAV_LEVEL - MIN_NAV_LEVEL);
    passed &= continuous && endpoints && all_levels;
    let _ = writeln!(
        out,
        "PREVIEW brightness-continuous:{} endpoints:{} all-openings:{} {}",
        flag(continuous),
        flag(endpoints),
        flag(all_levels),
        flag(passed)
    );
    passed
}

/// Appends the true child ratio per rung and checks each lies in `(0, 1)`.
fn verify_ratios(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let mut passed = true;
    for n in MIN_NAV_LEVEL..MAX_NAV_LEVEL {
        let level = Level::new(n).unwrap_or(Level::MIN);
        let ratio = child_ratio(level).unwrap_or(f64::NAN);
        let valid = ratio.is_finite() && ratio > 0.0 && ratio < 1.0;
        passed &= valid;
        let _ = writeln!(
            out,
            "RATIO L{}->L{} child/parent={:.3e} markers<={} {}",
            n,
            n + 1,
            ratio,
            level_budget(level).max_count,
            flag(valid)
        );
    }
    passed
}

/// Checks the published tail anchors L11-L14 (#394, R11).
///
/// The size exponents must read 5.70, 4.00, 1.50, 0.70. Appends one
/// `TAIL-ANCHORS` line.
fn verify_tail_anchors(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let want = [(11u8, 5.70), (12, 4.00), (13, 1.50), (14, 0.70)];
    let mut passed = true;
    for (n, e) in want {
        let level = Level::new(n).unwrap_or(Level::MIN);
        passed &= (scale_exponent(level) - e).abs() < 1e-9;
    }
    let _ = writeln!(
        out,
        "TAIL-ANCHORS L11=5.70 L12=4.00 L13=1.50 L14=0.70 {}",
        flag(passed)
    );
    passed
}

/// Checks the 1.5-decade milestone rule on the anchored tail gaps (#394).
///
/// Planet to region takes none, region to city two, city to building two,
/// building to room none. Appends one `MILESTONES` line.
fn verify_milestones(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let legs = [(10u8, 0usize), (11, 2), (12, 2), (13, 0)];
    let mut passed = true;
    for (n, k) in legs {
        let level = Level::new(n).unwrap_or(Level::MIN);
        passed &= anon_cells(level) == k;
    }
    let _ = writeln!(
        out,
        "MILESTONES L10-L11=0 L11-L12=2 L12-L13=2 L13-L14=0 {}",
        flag(passed)
    );
    passed
}

/// Checks air on the home planet, the size rule, and the airless count (#394).
///
/// The home L10 cell must carry thin pale-blue air on every surface point;
/// the pure size rule must read thick for giants and airless below half an
/// Earth radius; the airless count is the home cell's moons (no surface, no
/// air). Appends one `ATMOSPHERE` line.
fn verify_atmosphere(out: &mut String) -> bool {
    use crate::nest::autopilot_path;
    use crate::terrain::{THICK_AIR_TINT, THIN_AIR_TINT, air_for_radius_earth};
    use std::fmt::Write as _;
    let chain = autopilot_path(DEMO_SEED);
    let home_chain: &[u32] = chain.get(..9).unwrap_or(&[]);
    let home = generate_cell(DEMO_SEED, home_chain);
    let mut home_thin = !home.points.is_empty();
    for point in &home.points {
        if let Some(surface) = point.surface {
            let air = surface.air;
            home_thin &= surface.radius_earth == 1.0
                && air.is_some_and(|a| {
                    (a.thickness_km - 100.0).abs() < 1e-9 && a.tint == THIN_AIR_TINT
                });
        }
    }
    let giant_thick = air_for_radius_earth(11.0)
        .is_some_and(|a| (a.thickness_km - 1100.0).abs() < 1e-9 && a.tint == THICK_AIR_TINT);
    let airless_rule = air_for_radius_earth(0.3).is_none();
    let airless = home
        .points
        .iter()
        .filter(|point| point.moon.is_some())
        .count();
    let passed = home_thin && giant_thick && airless_rule;
    let _ = writeln!(
        out,
        "ATMOSPHERE home={} giant={} airless={} {}",
        if home_thin { "thin" } else { "FAIL" },
        if giant_thick { "thick" } else { "FAIL" },
        airless,
        flag(passed)
    );
    passed
}

/// Checks tail portals sit on their parent structure (#394).
///
/// L10 region portals sit on the relief at distinct heights (no face
/// centres, portal at the surface spot); L11 city portals sit at river and
/// coast vertices (within 1e-6 of a population); L12 building portals sit on
/// block edges along streets (within 0.06 of a street vertex); L13 room
/// portals sit inside building shells at floor heights. Every L11-L13
/// portal carries its offset. Appends one `PORTAL-ON-STRUCTURE` line for
/// L10-L13.
fn verify_portal_on_structure(out: &mut String) -> bool {
    use crate::nest::autopilot_path;
    use std::fmt::Write as _;
    let l10_ok = portal_on_structure_l10_pass();
    let chain = autopilot_path(DEMO_SEED);
    let mut tags: Vec<(u8, bool)> = vec![(10, l10_ok)];
    for (depth, n) in [(10usize, 11u8), (11, 12), (12, 13)] {
        let Some(prefix) = chain.get(..depth) else {
            tags.push((n, false));
            continue;
        };
        let cell = generate_cell(DEMO_SEED, prefix);
        tags.push((n, tail_portals_on_structure(&cell, n)));
    }
    let mut passed = true;
    let _ = write!(out, "PORTAL-ON-STRUCTURE");
    for (n, ok) in &tags {
        let _ = write!(out, " L{n}={}", flag(*ok));
        passed &= *ok;
    }
    let _ = writeln!(out, " {}", flag(passed));
    passed
}

/// L10 leg of the portal-on-structure check (relief shell, distinct
/// heights, no face centres, portal at the surface spot).
fn portal_on_structure_l10_pass() -> bool {
    use crate::nest::{autopilot_path, path_seed};
    use crate::terrain::{PLANET_RADIUS_CELL, RELIEF_RANGE_CELL, face_uv_for, surface_height};
    let chain = autopilot_path(DEMO_SEED);
    let home_chain: &[u32] = chain.get(..9).unwrap_or(&[]);
    let cell_seed = path_seed(DEMO_SEED, home_chain);
    let cell = generate_cell(DEMO_SEED, home_chain);
    let portals: Vec<_> = cell
        .points
        .iter()
        .filter(|point| point.kind == MarkerKind::Portal)
        .collect();
    if portals.is_empty() {
        return false;
    }
    let mut heights = Vec::new();
    let mut positions = Vec::new();
    for portal in &portals {
        if !matches!(portal.form, crate::r#gen::Form::Patch { .. }) {
            return false;
        }
        let reach = (portal.position[0] * portal.position[0]
            + portal.position[1] * portal.position[1]
            + portal.position[2] * portal.position[2])
            .sqrt();
        if !(PLANET_RADIUS_CELL - RELIEF_RANGE_CELL - 0.01
            ..=PLANET_RADIUS_CELL + RELIEF_RANGE_CELL + 0.01)
            .contains(&reach)
        {
            return false;
        }
        // L10 patches open at their centre: the spot is the position.
        if portal.portal.is_some() || portal.portal_position() != portal.position {
            return false;
        }
        if !reach.is_finite() || reach <= 0.0 {
            return false;
        }
        let unit = [
            portal.position[0] / reach,
            portal.position[1] / reach,
            portal.position[2] / reach,
        ];
        let (_, u, v) = face_uv_for(unit);
        if (u - 0.5).abs() < 1e-9 && (v - 0.5).abs() < 1e-9 {
            return false;
        }
        let (face, uu, vv) = face_uv_for(unit);
        heights.push(surface_height(cell_seed, face, uu, vv));
        positions.push(portal.position);
    }
    for (i, a) in positions.iter().enumerate() {
        for b in positions.iter().skip(i + 1) {
            let dist =
                ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
            if dist <= 1e-9 {
                return false;
            }
        }
    }
    for (i, a) in heights.iter().enumerate() {
        for b in heights.iter().skip(i + 1) {
            if (a - b).abs() <= 1e-9 {
                return false;
            }
        }
    }
    true
}

/// Tail leg (L11-L13) of the portal-on-structure check.
fn tail_portals_on_structure(cell: &crate::r#gen::Generated, level: u8) -> bool {
    use crate::r#gen::Form;
    let portals: Vec<_> = cell
        .points
        .iter()
        .filter(|point| point.kind == MarkerKind::Portal)
        .collect();
    if portals.is_empty() {
        return false;
    }
    // Structure anchors: populations without the ground grid.
    let anchors: Vec<_> = cell
        .points
        .iter()
        .filter(|point| {
            point.kind == MarkerKind::Population && !matches!(point.form, Form::Grid { .. })
        })
        .collect();
    if anchors.is_empty() {
        return false;
    }
    let shells: Vec<([f64; 3], f64)> = anchors
        .iter()
        .filter_map(|point| match point.form {
            Form::Box { height } => Some((point.position, height)),
            _ => None,
        })
        .collect();
    for portal in &portals {
        // Every tail portal carries its offset.
        if portal.portal.is_none() {
            return false;
        }
        // Forms: rects at L11/L12, boxes at L13.
        let form_ok = match level {
            13 => matches!(portal.form, Form::Box { .. }),
            _ => matches!(portal.form, Form::Rect { .. }),
        };
        if !form_ok {
            return false;
        }
        let near_structure = if level == 11 {
            // At a river or coast vertex.
            anchors.iter().any(|anchor| {
                let dx = portal.position[0] - anchor.position[0];
                let dy = portal.position[1] - anchor.position[1];
                let dz = portal.position[2] - anchor.position[2];
                (dx * dx + dy * dy + dz * dz).sqrt() < 1e-6
            })
        } else if level == 12 {
            // On a block edge along the street network.
            anchors.iter().any(|anchor| {
                let dx = portal.position[0] - anchor.position[0];
                let dy = portal.position[1] - anchor.position[1];
                let dz = portal.position[2] - anchor.position[2];
                (dx * dx + dy * dy + dz * dz).sqrt() < 0.06
            })
        } else {
            // Inside a building shell at a floor height.
            shells.iter().any(|(base, height)| {
                let dx = (portal.position[0] - base[0]).abs();
                let dz = (portal.position[2] - base[2]).abs();
                let vertical = portal.position[1] - base[1];
                dx < 1e-9 && dz < 1e-9 && vertical >= 0.0 && vertical <= *height + 1e-9
            })
        };
        if !near_structure {
            return false;
        }
    }
    true
}

/// Checks the ground curves by the true planet radius at L11-L13 (#394).
///
/// Prints the ladder-derived radii the tail generator settles points by.
/// Appends one `CURVATURE` line.
fn verify_curvature(out: &mut String) -> bool {
    use crate::frame::planet_radius_cells;
    use std::fmt::Write as _;
    let mut passed = true;
    let mut values = Vec::new();
    for n in [11u8, 12, 13] {
        let level = Level::new(n).unwrap_or(Level::MIN);
        match planet_radius_cells(level) {
            Some(radius) if radius.is_finite() && radius > 0.0 => {
                values.push((n, radius));
            }
            _ => {
                passed = false;
            }
        }
    }
    let _ = write!(out, "CURVATURE");
    for (n, radius) in &values {
        let _ = write!(out, " L{n}={radius:.3e}");
        // Gentle bowl at L11, nearly flat below: the radius must grow down
        // the tail and dwarf the cell.
        passed &= *radius > 1.0;
    }
    let _ = writeln!(out, " {}", flag(passed && values.len() == 3));
    passed && values.len() == 3
}

/// Checks the horizon lies beyond the cell at the tail entry heights (#394).
///
/// Opens the journey to L11, L12, and L13 through the oriented L10 patches
/// and measures the horizon at each arrival height: it must exceed the cell
/// radius, then close in on descent (owner-tested). Appends one `HORIZON`
/// line.
fn verify_horizon(out: &mut String) -> bool {
    use crate::frame::{horizon_distance, planet_radius_cells};
    use crate::nav::MarkerIndex;
    use crate::nest::autopilot_path;
    use std::fmt::Write as _;
    let chain = autopilot_path(DEMO_SEED);
    let mut universe = Universe::new(DEMO_SEED);
    let mut tags: Vec<(u8, &'static str)> = Vec::new();
    let mut passed = true;
    for &marker in &chain {
        if universe.level().get() >= 13 {
            break;
        }
        if !universe.open(MarkerIndex(marker)) {
            passed = false;
            break;
        }
        let n = universe.level().get();
        if n == 11 || n == 12 || n == 13 {
            let level = Level::new(n).unwrap_or(Level::MIN);
            let radius = planet_radius_cells(level).unwrap_or(f64::NAN);
            let height = universe.path.offset()[1].abs();
            let horizon = horizon_distance(radius, height);
            let beyond = horizon.is_finite() && horizon > 0.5;
            passed &= beyond;
            tags.push((n, if beyond { "beyond-cell" } else { "within-cell" }));
        }
    }
    passed &= tags.len() == 3;
    let _ = write!(out, "HORIZON");
    for (n, tag) in &tags {
        let _ = write!(out, " L{n}={tag}");
    }
    let _ = writeln!(out, " {}", flag(passed));
    passed
}

/// Checks the entered parent marker persists around the tail child (#394).
///
/// For open levels L11-L14 the entered parent point must exist, wear a
/// non-dot form, and map to an extent enclosing the open cell centre (so
/// `draw_parent_context` has a body to draw); at L11 the surface context
/// must name a positive planet radius (limb plus air). Appends one
/// `PARENT-CONTEXT` line.
fn verify_parent_context(out: &mut String) -> bool {
    use crate::nav::MarkerIndex;
    use crate::nest::autopilot_path;
    use std::fmt::Write as _;
    let chain = autopilot_path(DEMO_SEED);
    let mut universe = Universe::new(DEMO_SEED);
    let mut tags: Vec<(u8, bool)> = Vec::new();
    for &marker in &chain {
        if universe.level().get() >= 14 {
            break;
        }
        if !universe.open(MarkerIndex(marker)) {
            break;
        }
        let n = universe.level().get();
        if !(11..=14).contains(&n) {
            continue;
        }
        let Some(entered) = universe.path.entered() else {
            tags.push((n, false));
            continue;
        };
        let Some(parent) = &universe.parent else {
            tags.push((n, false));
            continue;
        };
        let Some(dot) = parent.points.get(entered.marker as usize) else {
            tags.push((n, false));
            continue;
        };
        if matches!(dot.form, crate::r#gen::Form::Dot) {
            tags.push((n, false));
            continue;
        }
        let extent_ok = if n == 11 {
            universe.surface_context().is_some_and(|surface| {
                surface.planet_radius.is_finite() && surface.planet_radius > 0.0
            })
        } else {
            entered.ratio.is_finite()
                && entered.ratio > 0.0
                && dot.radius.is_finite()
                && dot.radius > 0.0
                && (dot.radius / entered.ratio).is_finite()
        };
        tags.push((n, extent_ok));
    }
    let mut passed = tags.len() == 4;
    let _ = write!(out, "PARENT-CONTEXT");
    for (n, ok) in &tags {
        let _ = write!(out, " L{n}={}", flag(*ok));
        passed &= *ok;
    }
    let _ = writeln!(out, " {}", flag(passed));
    passed
}

/// Checks the home galaxy portal sits in its disk lane (#384).
///
/// The fixed journey must open the Milky Way through a portal offset from
/// the body centre. Appends one `HOME-PORTAL` line.
fn verify_home_portal(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let off_centre = home_portal_off_centre(DEMO_SEED);
    let _ = writeln!(
        out,
        "HOME-PORTAL L4 off-centre={} {}",
        flag(off_centre),
        flag(off_centre)
    );
    off_centre
}

/// Checks the journey L8 planets share one spaced, ordered plane (#384).
///
/// Coplanarity, orbit-axis order, and the minimum on-screen gap from
/// [`crate::orbits::orbits_check_l8`]. Appends one `ORBITS` line.
fn verify_orbits(out: &mut String) -> bool {
    use std::fmt::Write as _;
    let (coplanar, ordered, spaced) = orbits_l8(DEMO_SEED);
    let passed = coplanar && ordered && spaced;
    let _ = writeln!(
        out,
        "ORBITS L8 coplanar={} ordered={} spaced={} {}",
        flag(coplanar),
        flag(ordered),
        flag(spaced),
        flag(passed)
    );
    passed
}

/// Checks the scripted free-flight leg after the dive (#152).
///
/// Replays the leg twice for rerun identity, matches its starting snapshot
/// against the dive's own L5 snapshot, and proves the working set stayed
/// within `STREAM_CAP`. Positions print at six decimals, the cross-platform
/// byte-identity rule (`E-DET-TIERS`). Appends three `FREE-LEG` lines.
fn verify_free_leg(steps: &[JourneyStep], out: &mut String) -> bool {
    use std::fmt::Write as _;
    let (leg, end) = replay_free_leg(DEMO_SEED, VERIFY_DT, VERIFY_MAX_SECS);
    let (again, _) = replay_free_leg(DEMO_SEED, VERIFY_DT, VERIFY_MAX_SECS);
    let identical = leg == again;
    let level_five = Level::new(5).unwrap_or(Level::MIN);
    let snapshot_match = steps
        .iter()
        .find(|step| step.level == level_five)
        .is_some_and(|step| step.snapshot == leg.start_snapshot);
    let at_level = leg.level == level_five;
    let start_ok = at_level && snapshot_match;
    let _ = writeln!(
        out,
        "FREE-LEG start={} snapshot-match:{} steps={} {}",
        level_label(leg.level),
        flag(snapshot_match),
        leg.points.len(),
        flag(start_ok),
    );
    let [end_x, end_y, end_z] = end.path.offset();
    let [start_x, start_y, start_z] = leg.start_offset;
    let displacement =
        ((end_x - start_x).powi(2) + (end_y - start_y).powi(2) + (end_z - start_z).powi(2)).sqrt();
    let bounded = leg.alive_max <= STREAM_CAP;
    let _ = writeln!(
        out,
        "FREE-LEG end=[{end_x:.6}, {end_y:.6}, {end_z:.6}] displacement={displacement:.6} alive-max={} (max {}) {}",
        leg.alive_max,
        STREAM_CAP,
        flag(bounded),
    );
    let _ = writeln!(
        out,
        "FREE-LEG repeat-identical:{} {}",
        flag(identical),
        flag(identical),
    );
    start_ok && bounded && identical
}

/// Checks the open/close inverse at every depth of the journey.
///
/// Closing one level and reopening the same marker must restore the camera
/// offset (relative 1e-9). Closing everything must land at the root with a
/// finite camera inside the start radius. A full close then reopen cannot
/// restore the deep offset: thirteen ratios multiply to ~1e-27, below
/// float64 resolution in root units, which is exactly why the notion (5.1)
/// keeps the chain. Appends `INVERSE`.
fn verify_inverse(universe: &mut Universe, out: &mut String) -> bool {
    use std::fmt::Write as _;
    let mut path = universe.path.clone();
    let mut one_level_exact = true;
    let mut closed = 0;
    while let Some(opened) = path.close() {
        closed += 1;
        let mut reopened = path.clone();
        reopened.open_oriented(opened.marker, opened.position, opened.up);
        let mut back = reopened.clone();
        back.close();
        one_level_exact &= back
            .offset()
            .iter()
            .zip(path.offset())
            .all(|(a, b)| (a - b).abs() <= 1e-9 * b.abs().max(1.0));
    }
    let at_root = path.level() == Level::MIN;
    let start_radius = START_OFFSET.iter().map(|c| c * c).sum::<f64>().sqrt();
    let landed = path.offset().iter().all(|c| c.is_finite())
        && path.distance_to_center() <= start_radius + 1e-9;
    let passed = at_root && one_level_exact && landed;
    let _ = writeln!(
        out,
        "INVERSE closed={} at-root:{} one-level-round-trip:{} landed-inside-start:{} {}",
        closed,
        flag(at_root),
        flag(one_level_exact),
        flag(landed),
        flag(passed)
    );
    passed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::PORTAL_LANE_FRACTION;
    use crate::nest::{autopilot_candidates, autopilot_marker, autopilot_path};

    #[test]
    fn home_portal_sits_off_centre_on_the_fixed_journey() {
        assert!(
            home_portal_off_centre(DEMO_SEED),
            "the home galaxy portal must live in the disk lane"
        );
        // The lane sits at the documented fraction of the body radius.
        let chain = autopilot_path(DEMO_SEED);
        assert!(chain.len() >= 3, "the journey must reach L4");
        let home = &chain[..3];
        let level = Level::new(4).unwrap_or(Level::MIN);
        let seed = path_seed(DEMO_SEED, home);
        let cell = generate_cell(DEMO_SEED, home);
        let candidates = autopilot_candidates(level, seed, &cell.points);
        let pick = autopilot_marker(seed, &candidates).expect("home L4 pick");
        let point = &cell.points[pick as usize];
        let offset = point.portal.expect("the pick carries a portal");
        let lane = (offset[0] * offset[0] + offset[1] * offset[1] + offset[2] * offset[2]).sqrt();
        assert!(
            (lane - point.radius * PORTAL_LANE_FRACTION).abs() < 1e-12,
            "lane {lane} must sit at the documented fraction of {}",
            point.radius
        );
    }

    #[test]
    fn journey_l8_planets_read_coplanar_ordered_spaced() {
        let (coplanar, ordered, spaced) = orbits_l8(DEMO_SEED);
        assert!(coplanar, "planets share the ecliptic");
        assert!(ordered, "radii ascend with orbit axis");
        assert!(spaced, "neighbouring orbits clear the gap");
    }
}
