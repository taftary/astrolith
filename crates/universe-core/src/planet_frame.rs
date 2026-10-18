//! One planet-centred coordinate system per L10 planet (#774, step 1).
//!
//! The planet frame is in metres, `f64`, centred on the planet's middle and
//! fixed to the planet (ECEF-like, Q2 A: no spin yet; `tilt_deg` and
//! `spin_hours` stay recorded for step 7, #780). Each planet's radius comes
//! from its own record ([`crate::terrain::body_recipe`], Q3 A; home is Earth,
//! 6,371 km). The conversion between L10 open-cell units and metres happens
//! in one place only ([`PlanetFrame::from_cell`] / [`to_cell`](PlanetFrame::to_cell), Q1 A).
//!
//! [`LocalFrame`] is the tangent frame at any point on or above a planet
//! (up, north, east); later steps use it for patches (step 3, #776) and for
//! walking (step 6, #779). The full chain runs in four explicit typed steps
//! (T3): [`PlanetPos`] (f64 metres) → [`LocalPos`] (f64 metres in a local
//! frame) → [`CameraRel`] (f64 metres from the camera) → [`CameraRel::to_gpu`]
//! (the only narrowing, `E-CAST`). Per-frame conversions allocate nothing
//! (`E-HOT-NOALLOC`). Pure and deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::coords::Level;

/// Mean Earth radius in metres for the planet frame (Q3 A, AC1).
///
/// The home planet is Earth at 6,371 km. This is the spec value from #774
/// (AC1); [`crate::gravity::EARTH_RADIUS_M`] (equatorial 6,378.137 km) and
/// [`crate::terrain::EARTH_RADIUS_KM`] keep their catalog values for mass and
/// air. Frame radii use this value so every planet's radius in cells is its
/// true radius from its record through the one L10 metre (Q7 A).
pub const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// Metres per L10 open-cell unit: the one L10 metre (Q7 A).
///
/// `10^e(L10)` with `e = 7.11` from the frozen ladder, so
/// `body::metre(L10) = 1 / metres_per_cell_l10()`. One place only.
#[must_use]
pub fn metres_per_cell_l10() -> f64 {
    10f64.powf(crate::nest::scale_exponent(Level::MAX))
}

/// Cells per metre at L10 (the inverse of [`metres_per_cell_l10`]).
#[must_use]
pub fn cells_per_metre_l10() -> f64 {
    1.0 / metres_per_cell_l10()
}

/// Planet radius in metres for `radius_earth` Earth radii.
#[must_use]
pub fn radius_m_for(radius_earth: f64) -> f64 {
    radius_earth * EARTH_RADIUS_M
}

/// Planet radius in L10 open-cell units for `radius_earth` Earth radii.
///
/// The true radius through the one L10 metre: `radius_m / metres_per_cell`.
/// Earth is about 0.49 cells.
#[must_use]
pub fn radius_cells_for(radius_earth: f64) -> f64 {
    radius_m_for(radius_earth) * cells_per_metre_l10()
}

/// Earth radius in L10 open-cell units (about 0.49).
#[must_use]
pub fn earth_radius_cells() -> f64 {
    radius_cells_for(1.0)
}

/// Position in the planet frame: metres, `f64`, centred on the planet (Q1 A).
///
/// Newtype so cell units and metres never mix (`E-NEWTYPE`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanetPos(pub [f64; 3]);

impl PlanetPos {
    /// Length from the planet centre in metres.
    #[must_use]
    pub fn radius(&self) -> f64 {
        (self.0[0] * self.0[0] + self.0[1] * self.0[1] + self.0[2] * self.0[2]).sqrt()
    }
}

/// One planet's frame: its radius plus the one L10 metre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanetFrame {
    /// Planet radius in metres (from its record).
    radius_m: f64,
    /// Metres per L10 open-cell unit (the one L10 metre).
    metres_per_cell: f64,
}

impl PlanetFrame {
    /// Frame for a planet of `radius_earth` Earth radii.
    #[must_use]
    pub fn for_radius_earth(radius_earth: f64) -> PlanetFrame {
        PlanetFrame {
            radius_m: radius_m_for(radius_earth),
            metres_per_cell: metres_per_cell_l10(),
        }
    }

    /// Frame for the L10 planet at `chain` under `root` (Q3 A).
    ///
    /// Radius comes from [`crate::terrain::body_recipe`] for the cell seed,
    /// so the same seed always gives the same frame. The planet does not
    /// turn yet (Q2 A).
    #[must_use]
    pub fn for_planet(root: u64, chain: &[u32]) -> PlanetFrame {
        let seed = crate::nest::path_seed(root, chain);
        let home = crate::home::is_home_planet_cell(root, chain);
        let recipe = crate::terrain::body_recipe(seed, home);
        PlanetFrame::for_radius_earth(recipe.radius_earth)
    }

    /// Planet radius in metres.
    #[must_use]
    pub fn radius_m(&self) -> f64 {
        self.radius_m
    }

    /// Metres per L10 open-cell unit.
    #[must_use]
    pub fn metres_per_cell(&self) -> f64 {
        self.metres_per_cell
    }

    /// Planet radius in L10 open-cell units.
    #[must_use]
    pub fn radius_cells(&self) -> f64 {
        self.radius_m / self.metres_per_cell
    }

    /// L10 open-cell units into the planet frame (the one place, Q1 A).
    #[must_use]
    pub fn from_cell(&self, cell: [f64; 3]) -> PlanetPos {
        PlanetPos([
            cell[0] * self.metres_per_cell,
            cell[1] * self.metres_per_cell,
            cell[2] * self.metres_per_cell,
        ])
    }

    /// Planet frame back into L10 open-cell units (exact inverse of
    /// [`PlanetFrame::from_cell`]).
    #[must_use]
    pub fn to_cell(&self, pos: PlanetPos) -> [f64; 3] {
        [
            pos.0[0] / self.metres_per_cell,
            pos.0[1] / self.metres_per_cell,
            pos.0[2] / self.metres_per_cell,
        ]
    }
}

/// Local tangent frame at a point on or above a planet (AC2).
///
/// `up` is radial, `north` points toward the planet's `+Y` axis projected on
/// the tangent plane (fixed `+Z` fallback at the poles), `east` completes the
/// right-handed frame (`east x north = up`). All three are unit length and
/// pairwise orthogonal. Pure and deterministic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalFrame {
    /// Radial up in planet axes.
    pub up: [f64; 3],
    /// Toward the planet's `+Y` axis along the tangent plane.
    pub north: [f64; 3],
    /// Completing the right-handed frame.
    pub east: [f64; 3],
    /// The planet point this frame was built at (its origin).
    pub origin: PlanetPos,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(v: [f64; 3]) -> f64 {
    dot(v, v).sqrt()
}

impl LocalFrame {
    /// Local frame at planet position `at`.
    ///
    /// At the planet centre (zero length) the frame is the identity
    /// (`+Y` up). At the poles the north fallback is `+Z`.
    #[must_use]
    pub fn at(at: PlanetPos) -> LocalFrame {
        let r = length(at.0);
        let up = if r.is_finite() && r > 0.0 {
            [at.0[0] / r, at.0[1] / r, at.0[2] / r]
        } else {
            [0.0, 1.0, 0.0]
        };
        // North: +Y projected on the tangent plane, +Z fallback at poles.
        let axis = [0.0, 1.0, 0.0];
        let along = dot(axis, up);
        let mut north = [axis[0] - up[0] * along, axis[1] - up[1] * along, axis[2] - up[2] * along];
        if !(length(north).is_finite() && length(north) > 1e-12) {
            let fallback = [0.0, 0.0, 1.0];
            let along_fallback = dot(fallback, up);
            north = [
                fallback[0] - up[0] * along_fallback,
                fallback[1] - up[1] * along_fallback,
                fallback[2] - up[2] * along_fallback,
            ];
        }
        let north_len = length(north);
        let north = if north_len.is_finite() && north_len > 0.0 {
            [north[0] / north_len, north[1] / north_len, north[2] / north_len]
        } else {
            [0.0, 0.0, 1.0]
        };
        let mut east = cross(north, up);
        let east_len = length(east);
        east = if east_len.is_finite() && east_len > 0.0 {
            [east[0] / east_len, east[1] / east_len, east[2] / east_len]
        } else {
            [1.0, 0.0, 0.0]
        };
        LocalFrame {
            up,
            north,
            east,
            origin: at,
        }
    }

    /// Planet point into local metres relative to this frame's origin.
    #[must_use]
    pub fn to_local(&self, point: PlanetPos) -> LocalPos {
        let d = [
            point.0[0] - self.origin.0[0],
            point.0[1] - self.origin.0[1],
            point.0[2] - self.origin.0[2],
        ];
        LocalPos([dot(d, self.east), dot(d, self.north), dot(d, self.up)])
    }

    /// Local metres back into the planet frame.
    #[must_use]
    pub fn to_planet(&self, local: LocalPos) -> PlanetPos {
        PlanetPos([
            self.origin.0[0]
                + self.east[0] * local.0[0]
                + self.north[0] * local.0[1]
                + self.up[0] * local.0[2],
            self.origin.0[1]
                + self.east[1] * local.0[0]
                + self.north[1] * local.0[1]
                + self.up[1] * local.0[2],
            self.origin.0[2]
                + self.east[2] * local.0[0]
                + self.north[2] * local.0[1]
                + self.up[2] * local.0[2],
        ])
    }
}

/// Position in a [`LocalFrame`]: metres, `f64`, relative to its origin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalPos(pub [f64; 3]);

/// Camera-relative position: metres, `f64`, from the camera.
///
/// Built from [`LocalPos`] differences so the chain stays explicit; the
/// magnitude equals the planet-frame distance (rotation preserves it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraRel(pub [f64; 3]);

impl CameraRel {
    /// Camera-relative metres from local positions of a point and the camera
    /// in the same [`LocalFrame`].
    #[must_use]
    pub fn from_local(point: LocalPos, camera: LocalPos) -> CameraRel {
        CameraRel([
            point.0[0] - camera.0[0],
            point.0[1] - camera.0[1],
            point.0[2] - camera.0[2],
        ])
    }

    /// Camera-relative metres directly from planet positions.
    #[must_use]
    pub fn from_planet(point: PlanetPos, camera: PlanetPos) -> CameraRel {
        CameraRel([
            point.0[0] - camera.0[0],
            point.0[1] - camera.0[1],
            point.0[2] - camera.0[2],
        ])
    }

    /// Narrow to the GPU: the only `f64 -> f32` step in the chain (`E-CAST`).
    #[must_use]
    pub fn to_gpu(&self) -> [f32; 3] {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: camera-relative metres narrowed for the GPU, intended"
        )]
        let gpu = [self.0[0] as f32, self.0[1] as f32, self.0[2] as f32];
        gpu
    }
}

/// L10 cell-unit offset of `point_cells` from `camera_cells` through the one
/// conversion (Q4 A).
///
/// `from_cell` both into metres, subtract in metres, scale back to cells.
/// Equals the direct subtraction up to floating-point round-off (about 1e-16
/// relative); every L10 position (camera, body, spot, moons, markers, saved
/// views) shares this path.
#[must_use]
pub fn l10_camera_offset(
    point_cells: [f64; 3],
    camera_cells: [f64; 3],
    frame: &PlanetFrame,
) -> [f64; 3] {
    let point = frame.from_cell(point_cells);
    let camera = frame.from_cell(camera_cells);
    let rel = CameraRel::from_planet(point, camera);
    [
        rel.0[0] / frame.metres_per_cell(),
        rel.0[1] / frame.metres_per_cell(),
        rel.0[2] / frame.metres_per_cell(),
    ]
}

/// Saved-view offset through the one conversion (Q4 A).
///
/// The settings file keeps cell units, so old files load unchanged; at L10
/// the offset is converted on load and save through [`PlanetFrame::from_cell`]
/// and [`PlanetFrame::to_cell`]. The round trip returns the start point
/// within 1 µm in metres.
#[must_use]
pub fn convert_saved_view_offset(offset: [f64; 3], frame: &PlanetFrame) -> [f64; 3] {
    frame.to_cell(frame.from_cell(offset))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn earth() -> PlanetFrame {
        PlanetFrame::for_radius_earth(1.0)
    }

    #[test]
    fn planet_frame_per_planet_in_metres() {
        // AC1 / C1: each L10 planet has its own radius from its record; same
        // seed gives the same frame.
        let earth_m = earth().radius_m();
        assert!(
            (earth_m - 6_371_000.0).abs() < 1.0,
            "home planet is Earth, 6,371 km: {earth_m}"
        );
        let earth_cells = earth().radius_cells();
        assert!(
            (0.49..0.50).contains(&earth_cells),
            "Earth fills the L10 cell as the ladder says: {earth_cells}"
        );
        // Two other seeded planets have their own radii.
        let a = PlanetFrame::for_planet(42, &[7, 2, 7, 1, 2, 0, 0, 1, 3]);
        let b = PlanetFrame::for_planet(999, &[1, 1, 1, 1, 1, 1, 1, 1, 1]);
        assert!(a.radius_m() > 0.0 && b.radius_m() > 0.0);
        // Same seed and chain give the same frame (byte-identical).
        let again = PlanetFrame::for_planet(42, &[7, 2, 7, 1, 2, 0, 0, 1, 3]);
        assert_eq!(a, again, "same seed, same planet");
        // The journey home cell is Earth.
        let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        let home_chain: &[u32] = journey.get(..9).unwrap_or(&[]);
        let home = PlanetFrame::for_planet(crate::nav::DEMO_SEED, home_chain);
        assert!(
            (home.radius_m() - 6_371_000.0).abs() < 1.0,
            "journey planet is Earth: {}",
            home.radius_m()
        );
    }

    #[test]
    fn local_frame_is_orthonormal() {
        // AC2 / C2: axes at right angles and of unit length, including poles
        // and a face corner.
        let frame = earth();
        let at = [
            PlanetPos([0.0, frame.radius_m(), 0.0]),
            PlanetPos([0.0, -frame.radius_m(), 0.0]),
            PlanetPos([frame.radius_m(), 0.0, 0.0]),
            PlanetPos([frame.radius_m(), frame.radius_m(), frame.radius_m()]),
            PlanetPos([1_000.0, frame.radius_m(), -2_000.0]),
        ];
        for pos in at {
            let local = LocalFrame::at(pos);
            for (name, axis) in [("up", local.up), ("north", local.north), ("east", local.east)] {
                let len = length(axis);
                assert!(
                    (len - 1.0).abs() < 1e-12,
                    "{name} unit at {pos:?}: {len}"
                );
            }
            assert!(dot(local.up, local.north).abs() < 1e-12, "up.north at {pos:?}");
            assert!(dot(local.up, local.east).abs() < 1e-12, "up.east at {pos:?}");
            assert!(
                dot(local.north, local.east).abs() < 1e-12,
                "north.east at {pos:?}"
            );
            // Round trip through the local frame returns the start point.
            let probe = PlanetPos([pos.0[0] + 10.0, pos.0[1] - 5.0, pos.0[2] + 3.0]);
            let back = local.to_planet(local.to_local(probe));
            let err = length([
                back.0[0] - probe.0[0],
                back.0[1] - probe.0[1],
                back.0[2] - probe.0[2],
            ]);
            assert!(err < 1e-9, "local round trip: {err} m at {pos:?}");
        }
    }

    #[test]
    fn cell_to_planet_round_trip_within_a_micrometre() {
        // AC3 / C3: L10 cell units to the planet frame and back within 1 µm
        // at every tested height.
        let frame = earth();
        let heights = [0.01, 1.0, 100.0, 10_000.0, 1_000_000.0, 2_000_000.0];
        let dirs = [
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
        ];
        for &h in &heights {
            for dir in dirs {
                let len = length(dir);
                let unit = [dir[0] / len, dir[1] / len, dir[2] / len];
                let r = frame.radius_m() + h;
                let planet = PlanetPos([unit[0] * r, unit[1] * r, unit[2] * r]);
                let cell = frame.to_cell(planet);
                let back = frame.from_cell(cell);
                let err = length([
                    back.0[0] - planet.0[0],
                    back.0[1] - planet.0[1],
                    back.0[2] - planet.0[2],
                ]);
                assert!(
                    err < 1e-6,
                    "round trip {err} m at height {h} m, dir {dir:?}"
                );
            }
        }
    }

    #[test]
    fn l10_positions_share_one_conversion() {
        // AC5 / C5: camera, body, spot, moons, markers and saved views at L10
        // go through the one conversion; an old settings file still loads to
        // the same place. Core, no window (E-TEST-NO-WINDOW).
        let journey = crate::nest::autopilot_path(crate::nav::DEMO_SEED);
        let mut universe = crate::nav::Universe::new(crate::nav::DEMO_SEED);
        for &marker in &journey {
            assert!(
                universe.open(crate::nav::MarkerIndex(marker)),
                "journey opens"
            );
        }
        assert_eq!(universe.level().get(), 10);
        let chain = universe.path.indices();
        let l10: &[u32] = chain.get(..9).unwrap_or(&[]);
        let frame = PlanetFrame::for_planet(crate::nav::DEMO_SEED, l10);
        let mpc = frame.metres_per_cell();
        let check = |name: &str, cell: [f64; 3]| {
            let back = frame.to_cell(frame.from_cell(cell));
            let err_m = length([
                (back[0] - cell[0]) * mpc,
                (back[1] - cell[1]) * mpc,
                (back[2] - cell[2]) * mpc,
            ]);
            assert!(err_m < 1e-6, "{name} round trip {err_m} m at {cell:?}");
            // Through the shared camera offset as well.
            let camera = universe.path.offset();
            let via = l10_camera_offset(cell, camera, &frame);
            let direct = [cell[0] - camera[0], cell[1] - camera[1], cell[2] - camera[2]];
            let gap_m = length([
                (via[0] - direct[0]) * mpc,
                (via[1] - direct[1]) * mpc,
                (via[2] - direct[2]) * mpc,
            ]);
            assert!(
                gap_m < 1e-6,
                "{name} shared offset {gap_m} m at {cell:?}"
            );
        };
        // Camera.
        check("camera", universe.path.offset());
        // Body and spot.
        let spot = crate::player::PlayerSpot::seeded(crate::nav::DEMO_SEED).expect("spot");
        check("spot", spot.position);
        let body = crate::body::Body::standing(spot.position, spot.yaw);
        check("body", body.position);
        // Moons and markers.
        let mut moons = 0;
        for point in &universe.open.points {
            if point.moon.is_some() {
                moons += 1;
            }
            check("marker", point.position);
        }
        assert!(moons >= 1, "L10 has moons");
        // Saved views: an old settings line loads to the same L10 place.
        let old = "view1 = dive, 0.1, 0.494, -0.2, 0.0, 0.0, 5, -\n";
        let settings = crate::settings::Settings::parse(old);
        let view = settings.views[0].expect("old view loads");
        let back = convert_saved_view_offset(view.offset, &frame);
        let err_m = length([
            (back[0] - view.offset[0]) * mpc,
            (back[1] - view.offset[1]) * mpc,
            (back[2] - view.offset[2]) * mpc,
        ]);
        assert!(err_m < 1e-6, "old saved view round trip {err_m} m");
    }
}
