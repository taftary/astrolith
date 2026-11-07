//! The drawn ground as one surface for the body (rebuild step 6, #779 Q1 A).
//!
//! A [`DrawnSurface`](crate::drawn_surface::DrawnSurface) holds what one
//! frame draws: the planet frame, the camera the morph blends from, the
//! detail threshold, and every drawn patch with its shared mesh and morph
//! mask. [`DrawnSurface::ground_at`](crate::drawn_surface::DrawnSurface::ground_at)
//! finds the finest drawn patch over a point, the triangle that patch draws
//! there (`a, c, b / b, c, d`, the same diagonal on every face), blends its
//! three corners exactly as the vertex shader does
//! ([`crate::lod_patch_morph::blended_vertex`]), and intersects the radial
//! line through the point with that triangle. Collision and the picture are
//! one surface. Up is radial (Q3 A).
//!
//! Pure and deterministic (`E-DET-TIERS`, `E-CORE-NO-BEVY`).

use crate::body::Ground;
use crate::cube_sphere::from_sphere;
use crate::lod::{PATCH_QUADS, PatchKey};
use crate::lod_mesh::PatchMesh;
use crate::lod_patch_morph::blended_vertex;
use crate::planet_frame::{PlanetFrame, PlanetPos};
use std::sync::Arc;

/// Grid points per patch edge.
const SIDE: usize = 33;

/// Deepest level searched for a covering patch (Earth's cap is 18).
const MAX_LEVEL: u32 = 24;

/// One drawn patch: key, shared mesh, morph mask.
pub type DrawnPatch = (PatchKey, Arc<PatchMesh>, u8);

/// Actual shader-blended triangle at a radial ground query (#1238).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawnTriangle {
    /// Patch owning these vertices, not a procedural-detail prediction.
    pub patch: PatchKey,
    /// Production vertices in planet-fixed metres, after morph/edge masks.
    pub vertices: [PlanetPos; 3],
    /// Same radial intersection returned by the production ground query.
    pub point: PlanetPos,
    /// Oriented geometric triangle normal, distinct from collision's radial up.
    pub normal: crate::frame_diagnostic::CellDirection,
    /// Angle between triangle normal and radial up in degrees.
    pub slope: crate::frame_diagnostic::DirectionErrorDegrees,
}

#[cfg(test)]
mod observation_tests {
    use super::*;

    #[test]
    fn observed_triangles_are_the_production_ground_at_faces_seams_and_poles() {
        let frame = PlanetFrame::for_radius_earth(1.0);
        let relief = crate::relief::Relief::for_recipe(42, 1.0);
        let patches = crate::cube_sphere::Face::all()
            .into_iter()
            .map(|face| {
                let key = PatchKey::new(face, 0, 0, 0).unwrap();
                (
                    key,
                    Arc::new(crate::lod_mesh::build_patch(key, frame.radius_m(), &relief).unwrap()),
                    0,
                )
            })
            .collect();
        let surface = DrawnSurface::new(
            frame,
            PlanetPos([frame.radius_m() + 2.0, 0.0, 0.0]),
            2.0,
            patches,
        );
        for direction in [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
            [-1.0, 0.0, -1.0],
        ] {
            let query = PlanetPos(direction.map(|v| v * frame.radius_m()));
            let triangle = surface.drawn_triangle_at(query).unwrap();
            let (ground, _) = surface.ground_at_planet(query).unwrap();
            assert_eq!(
                triangle.point, ground,
                "uses precisely the production triangle intersection"
            );
            assert!(triangle.slope.0.is_finite() && (0.0..=90.0).contains(&triangle.slope.0));
            assert_eq!(triangle.patch, surface.covering(query).unwrap());
        }
        let empty = DrawnSurface::new(frame, PlanetPos([0.0; 3]), 2.0, Vec::new());
        assert!(
            empty
                .drawn_triangle_at(PlanetPos([frame.radius_m(), 0.0, 0.0]))
                .is_none()
        );
    }
}

/// What one frame draws, for the ground under the body.
#[derive(Debug, Clone)]
pub struct DrawnSurface {
    frame: PlanetFrame,
    camera: PlanetPos,
    detail_px: f64,
    patches: Vec<DrawnPatch>,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn unit(a: [f64; 3]) -> Option<[f64; 3]> {
    let len = dot(a, a).sqrt();
    if !len.is_finite() || len <= 0.0 {
        return None;
    }
    Some([a[0] / len, a[1] / len, a[2] / len])
}

impl DrawnSurface {
    /// The surface drawn with `patches` (any order) seen from `camera` at
    /// `detail_px`, on the planet `frame`.
    #[must_use]
    pub fn new(
        frame: PlanetFrame,
        camera: PlanetPos,
        detail_px: f64,
        mut patches: Vec<DrawnPatch>,
    ) -> DrawnSurface {
        patches.sort_by_key(|(key, _, _)| *key);
        patches.dedup_by_key(|(key, _, _)| *key);
        DrawnSurface {
            frame,
            camera,
            detail_px,
            patches,
        }
    }

    /// The planet frame.
    #[must_use]
    pub fn frame(&self) -> PlanetFrame {
        self.frame
    }

    /// Moves the morph camera (the blend follows the camera every frame,
    /// the drawn patches only when the selection changes).
    pub fn set_camera(&mut self, camera: PlanetPos) {
        self.camera = camera;
    }

    /// Where a body at open-cell `position` stands: its feet on the drawn
    /// ground under it (#779 Q7 A). `None` where nothing is drawn.
    #[must_use]
    pub fn place_on_ground(&self, position: [f64; 3]) -> Option<[f64; 3]> {
        self.ground_at(position).map(|g| g.point)
    }

    /// Number of drawn patches.
    #[must_use]
    pub fn len(&self) -> usize {
        self.patches.len()
    }

    /// Whether nothing is drawn.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.patches.is_empty()
    }

    /// The drawn patch `key` (mesh and morph mask), if drawn.
    #[must_use]
    pub fn patch(&self, key: PatchKey) -> Option<&DrawnPatch> {
        self.drawn(key)
    }

    fn drawn(&self, key: PatchKey) -> Option<&DrawnPatch> {
        self.patches
            .binary_search_by_key(&key, |(k, _, _)| *k)
            .ok()
            .and_then(|i| self.patches.get(i))
    }

    /// The drawn ground under `position` (open-cell units), in planet
    /// metres: the point on the drawn triangle along the radial line, and
    /// the radial up. `None` where no drawn patch covers the direction.
    #[must_use]
    pub fn ground_at_planet(&self, position: PlanetPos) -> Option<(PlanetPos, [f64; 3])> {
        let dir = unit(position.0)?;
        let face_pos = from_sphere(dir)?;
        // Finest drawn patch over the direction.
        for level in (0..=MAX_LEVEL).rev() {
            let per = 1u32.checked_shl(level)?;
            let n = f64::from(per) * f64::from(PATCH_QUADS);
            let gu = ((face_pos.u + 1.0) / 2.0 * n).clamp(0.0, n);
            let gv = ((face_pos.v + 1.0) / 2.0 * n).clamp(0.0, n);
            let q = f64::from(PATCH_QUADS);
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "E-CAST: clamped patch index in 0..per, exact"
            )]
            let (x, y) = (
                (gu / q).floor().min(f64::from(per - 1)) as u32,
                (gv / q).floor().min(f64::from(per - 1)) as u32,
            );
            let Some(key) = PatchKey::new(face_pos.face, level, x, y) else {
                continue;
            };
            let Some((_, mesh, mask)) = self.drawn(key) else {
                continue;
            };
            let li = gu - f64::from(x) * q;
            let lj = gv - f64::from(y) * q;
            let point = self.on_triangle(mesh, *mask, li, lj, dir)?;
            return Some((PlanetPos(point), dir));
        }
        None
    }

    /// The drawn patch [`DrawnSurface::ground_at_planet`] reads at
    /// `position` (tests and the headless line).
    #[must_use]
    pub fn covering(&self, position: PlanetPos) -> Option<PatchKey> {
        let dir = unit(position.0)?;
        let face_pos = from_sphere(dir)?;
        for level in (0..=MAX_LEVEL).rev() {
            let per = 1u32.checked_shl(level)?;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "E-CAST: clamped patch index in 0..per, exact"
            )]
            let cell = |t: f64| {
                ((t + 1.0) / 2.0 * f64::from(per))
                    .floor()
                    .clamp(0.0, f64::from(per - 1)) as u32
            };
            let key = PatchKey::new(face_pos.face, level, cell(face_pos.u), cell(face_pos.v))?;
            if self.drawn(key).is_some() {
                return Some(key);
            }
        }
        None
    }

    /// [`DrawnSurface::ground_at_planet`] in open-cell units, as the body
    /// uses it.
    #[must_use]
    pub fn ground_at(&self, position: [f64; 3]) -> Option<Ground> {
        let (point, up) = self.ground_at_planet(self.frame.from_cell(position))?;
        Some(Ground {
            point: self.frame.to_cell(point),
            up,
        })
    }

    /// Read-only triangle/normal/slope from the same meshes and morph path as
    /// production collision. Absent where no actual drawn patch covers the query.
    #[must_use]
    pub fn drawn_triangle_at(&self, position: PlanetPos) -> Option<DrawnTriangle> {
        let dir = unit(position.0)?;
        let face_pos = from_sphere(dir)?;
        let key = self.covering(position)?;
        let (_, mesh, mask) = self.drawn(key)?;
        let per = 1u32.checked_shl(key.level)?;
        let n = f64::from(per) * f64::from(PATCH_QUADS);
        let gu = ((face_pos.u + 1.0) / 2.0 * n).clamp(0.0, n);
        let gv = ((face_pos.v + 1.0) / 2.0 * n).clamp(0.0, n);
        let li = gu - f64::from(key.x) * f64::from(PATCH_QUADS);
        let lj = gv - f64::from(key.y) * f64::from(PATCH_QUADS);
        let vertices = self.triangle_vertices(mesh, *mask, li, lj)?;
        let [a, b, c] = vertices;
        let mut normal = unit(cross(sub(b, a), sub(c, a)))?;
        if dot(normal, dir) < 0.0 {
            normal = normal.map(|v| -v);
        }
        let normal = crate::frame_diagnostic::CellDirection(normal);
        Some(DrawnTriangle {
            patch: key,
            vertices: vertices.map(PlanetPos),
            point: PlanetPos(self.on_triangle(mesh, *mask, li, lj, dir)?),
            normal,
            slope: crate::frame_diagnostic::direction_error(
                normal,
                crate::frame_diagnostic::CellDirection(dir),
            )?,
        })
    }

    /// Where the radial line along `dir` meets the triangle `mesh` draws at
    /// local grid place `(li, lj)` (0..=32), corners blended as the shader
    /// blends them.
    fn on_triangle(
        &self,
        mesh: &PatchMesh,
        mask: u8,
        li: f64,
        lj: f64,
        dir: [f64; 3],
    ) -> Option<[f64; 3]> {
        let [p0, p1, p2] = self.triangle_vertices(mesh, mask, li, lj)?;
        let normal = cross(sub(p1, p0), sub(p2, p0));
        let along = dot(normal, dir);
        if !along.is_finite() || along.abs() <= f64::EPSILON * dot(normal, normal).sqrt() {
            return None;
        }
        let t = dot(normal, p0) / along;
        if !t.is_finite() || t <= 0.0 {
            return None;
        }
        Some([dir[0] * t, dir[1] * t, dir[2] * t])
    }

    fn triangle_vertices(
        &self,
        mesh: &PatchMesh,
        mask: u8,
        li: f64,
        lj: f64,
    ) -> Option<[[f64; 3]; 3]> {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "E-CAST: clamped cell index 0..=31, exact"
        )]
        let (ci, cj) = (
            li.floor().clamp(0.0, 31.0) as usize,
            lj.floor().clamp(0.0, 31.0) as usize,
        );
        #[expect(
            clippy::cast_precision_loss,
            reason = "E-CAST: cell index under 32, exact"
        )]
        let (fx, fy) = (li - ci as f64, lj - cj as f64);
        let at = |i: usize, j: usize| j * SIDE + i;
        let (a, b, c, d) = (
            at(ci, cj),
            at(ci + 1, cj),
            at(ci, cj + 1),
            at(ci + 1, cj + 1),
        );
        let tri = if fx + fy <= 1.0 { [a, b, c] } else { [d, c, b] };
        let corner = |idx: usize| {
            blended_vertex(mesh, idx, self.camera, self.detail_px, mask).map(|(p, _)| p)
        };
        Some([corner(tri[0])?, corner(tri[1])?, corner(tri[2])?])
    }
}
