//! Semantic marker registry: every displayable part of the open cell (#1128 T2).
//!
//! [`crate::semantic::Registry::rebuild`] classifies the open cell's generated points into
//! [`crate::semantic_catalog::SemanticType`] parts from the catalog ([`crate::semantic_catalog`]),
//! with a stable identity, parent, true anchor, extent and orientation.
//! Parts that many members share (a belt band, the host disk, the Oort
//! reservoir) get one identity from their canonical geometry and appear
//! once. Nothing here generates content or changes a point: the registry
//! is a read-only projection, rebuilt only when the cell's content changes
//! (`E-HOT-NOALLOC`), and navigation never reads it.

use crate::coords::Level;
use crate::r#gen::{Environment, Form, Generated, MarkerKind, Point};
pub use crate::semantic_catalog::{
    BudgetGroup, ColourRole, LabelRule, LocalRule, OcclusionRule, PartSpec, SemanticType,
    ShapeFamily, SizeBand, ViewRule,
};

/// What a part belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Parent {
    /// A generated marker of the open cell, by stable marker id.
    Marker(u32),
    /// The open cell itself (shared structures and references).
    Cell,
}

/// Stable identity of one part: the same object keeps it while the camera
/// moves, and a shared part has exactly one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartId {
    /// Level of the cell the part lives in.
    pub level: u8,
    /// Part type.
    pub kind: SemanticType,
    /// Owner key: the marker id, or a canonical-geometry key when shared.
    pub key: u64,
}

/// One registered part with its true geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartRecord {
    /// Stable identity.
    pub id: PartId,
    /// Owning object.
    pub parent: Parent,
    /// True anchor in cell units.
    pub anchor: [f64; 3],
    /// Conservative extent (radius) in cell units.
    pub extent: f64,
    /// Unit normal of flat parts, or the axis of axes.
    pub orientation: Option<[f64; 3]>,
    /// The owner is a portal (it opens deeper).
    pub portal: bool,
}

impl PartRecord {
    /// Complete display metadata of the part's type.
    #[must_use]
    pub const fn spec(&self) -> PartSpec {
        self.id.kind.spec()
    }
}

/// Shape of the host galaxy around an L4 cell, from its L3 marker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HostShape {
    /// Unit normal of the host disk.
    pub normal: [f64; 3],
    /// The host has a central bar.
    pub barred: bool,
}

/// The open cell's parts, rebuilt when its content changes.
#[derive(Clone, Debug, Default)]
pub struct Registry {
    records: Vec<PartRecord>,
    revision: Option<(u64, u64)>,
}

impl Registry {
    /// Registered parts, sorted by identity, each identity once.
    #[must_use]
    pub fn records(&self) -> &[PartRecord] {
        &self.records
    }

    /// The (open seed, content revision) last registered, if any.
    #[must_use]
    pub const fn revision(&self) -> Option<(u64, u64)> {
        self.revision
    }

    /// Rebuilds from `cell` unless `revision` (open seed, content revision)
    /// is the one already registered; returns whether it rebuilt. The
    /// buffer is reused, so a steady cell never allocates.
    pub fn rebuild(
        &mut self,
        level: Level,
        cell: &Generated,
        host: Option<HostShape>,
        revision: (u64, u64),
    ) -> bool {
        if self.revision == Some(revision) {
            return false;
        }
        self.revision = Some(revision);
        self.records.clear();
        classify_cell(level, cell, host, &mut self.records);
        true
    }
}

/// Classifies every point of `cell` at `level` into `out`, then sorts by
/// identity and keeps each identity once (shared parts draw once).
pub fn classify_cell(
    level: Level,
    cell: &Generated,
    host: Option<HostShape>,
    out: &mut Vec<PartRecord>,
) {
    let l = level.get();
    cell_parts(l, cell, host, out);
    for (id, point) in cell.with_ids() {
        classify_point(l, id, point, out);
    }
    out.sort_by_key(|record| record.id);
    out.dedup_by_key(|record| record.id);
}

/// Parts the cell itself owns, independent of its members.
fn cell_parts(level: u8, cell: &Generated, host: Option<HostShape>, out: &mut Vec<PartRecord>) {
    let mut push = |kind: SemanticType, extent: f64, orientation: Option<[f64; 3]>| {
        out.push(PartRecord {
            id: PartId {
                level,
                kind,
                key: 0,
            },
            parent: Parent::Cell,
            anchor: [0.0; 3],
            extent,
            orientation,
            portal: false,
        });
    };
    match level {
        1 => {
            push(SemanticType::UniverseBoundary, 0.5, None);
            push(SemanticType::Graticule, 0.5, None);
        }
        2 => push(SemanticType::RegionContext, 0.5, None),
        3 => push(SemanticType::ClusterContext, 0.5, None),
        4 => {
            let normal = host
                .map(|shape| shape.normal)
                .or_else(|| cloud_normal(cell))
                .unwrap_or([0.0, 0.0, 1.0]);
            push(SemanticType::HostDisk, 0.5, Some(normal));
            push(SemanticType::HostArms, 0.5, Some(normal));
            if host.is_none_or(|shape| shape.barred) {
                push(SemanticType::HostBar, 0.15, Some(normal));
            }
        }
        5 => push(SemanticType::HostContext, 0.5, None),
        7 => {
            push(SemanticType::SystemBoundary, 0.5, None);
            push(SemanticType::OortReservoir, 0.5, None);
        }
        _ => {}
    }
}

/// The disk normal the L4 cloud rings share, when the cell has clouds.
fn cloud_normal(cell: &Generated) -> Option<[f64; 3]> {
    cell.points.iter().find_map(|point| match point.form {
        Form::Ring { normal, .. } if point.cloud.is_some() => Some(normal),
        _ => None,
    })
}

/// The point sits exactly at the cell centre.
fn at_centre(position: [f64; 3]) -> bool {
    position.iter().all(|component| *component == 0.0)
}

/// Canonical key of a shared band: its type plus quantised geometry.
fn band_key(inner: f64, outer: f64, normal: [f64; 3]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for value in [inner, outer, normal[0], normal[1], normal[2]] {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "E-CAST: quantising geometry to a stable integer key, intended"
        )]
        let step = (value * 1e9).round() as i64;
        for byte in step.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
}

/// Classifies one point; a point may own several parts.
fn classify_point(level: u8, id: u32, point: &Point, out: &mut Vec<PartRecord>) {
    if point.surface.is_some() {
        // Physical ground samples are not markers.
        return;
    }
    let portal = point.kind == MarkerKind::Portal;
    let mut push = |kind: SemanticType, parent: Parent, key: u64, anchor, extent, orientation| {
        out.push(PartRecord {
            id: PartId { level, kind, key },
            parent,
            anchor,
            extent,
            orientation,
            portal,
        });
    };
    let own = Parent::Marker(id);
    let key = u64::from(id);
    let at = point.position;
    let radius = point.radius;
    let orbit = (at[0] * at[0] + at[1] * at[1] + at[2] * at[2]).sqrt();
    match (level, point.form) {
        (1, _) => push(SemanticType::Region, own, key, at, radius, None),
        (2 | 3, _) if point.environment.is_some() && !(level == 3 && portal) => {
            if level == 2 && portal {
                push(SemanticType::ClusterGroup, own, key, at, radius, None);
            }
            let (kind, orientation) = match (point.environment, point.form) {
                (Some(Environment::Void), Form::Ring { normal, .. }) => {
                    (SemanticType::Void, Some(normal))
                }
                (Some(Environment::Void), _) => (SemanticType::Void, None),
                (Some(Environment::Wall), Form::Sheet { normal }) => {
                    (SemanticType::Wall, Some(normal))
                }
                (Some(Environment::Wall), _) => (SemanticType::Wall, None),
                (Some(Environment::Filament), _) => (SemanticType::Filament, None),
                _ => (SemanticType::Node, None),
            };
            if level == 3 && !portal {
                push(SemanticType::GalaxyMember, own, key, at, radius, None);
            }
            push(kind, own, key, at, radius, orientation);
        }
        (3, form) => {
            let kind = if portal {
                SemanticType::Galaxy
            } else {
                SemanticType::GalaxyMember
            };
            push(kind, own, key, at, radius, disk_normal(form));
        }
        (4, Form::Ring { normal, outer, .. }) => {
            push(SemanticType::Cloud, own, key, at, radius, None);
            push(
                SemanticType::CloudBoundary,
                own,
                key,
                at,
                outer,
                Some(normal),
            );
        }
        (4, _) => push(SemanticType::Cloud, own, key, at, radius, None),
        (5, _) => {
            let kind = if portal {
                SemanticType::BrightStar
            } else {
                SemanticType::Star
            };
            push(kind, own, key, at, radius, None);
        }
        (6, Form::Arc { normal, .. }) => {
            push(SemanticType::CompanionStar, own, key, at, radius, None);
            push(
                SemanticType::CompanionArc,
                own,
                key,
                at,
                radius,
                Some(normal),
            );
        }
        (6, _) => push(SemanticType::PrimaryStar, own, key, at, radius, None),
        (7 | 8, _) if point.star.is_some() => {
            push(SemanticType::CentralStar, own, key, at, radius, None);
        }
        (9, _) if point.star.is_some() && at_centre(at) => {
            push(SemanticType::CentralStar, own, key, at, radius, None);
        }
        (9, _) if point.star.is_some() => {
            push(SemanticType::CompanionStar, own, key, at, radius, None);
        }
        (
            7,
            Form::Ring {
                normal,
                inner,
                outer,
            },
        ) => {
            let band = if inner == crate::orbits::kuiper_band().0 {
                SemanticType::KuiperBelt
            } else {
                SemanticType::ScatteredDisc
            };
            push(SemanticType::BeltMember, own, key, at, radius, None);
            let shared = band_key(inner, outer, normal);
            push(band, Parent::Cell, shared, [0.0; 3], outer, Some(normal));
        }
        (7, _) => push(SemanticType::OortMember, own, key, at, radius, None),
        (
            8,
            Form::Ring {
                normal,
                inner,
                outer,
            },
        ) => {
            let band = if inner == crate::orbits::asteroid_band().0 {
                SemanticType::AsteroidBelt
            } else {
                SemanticType::KuiperBelt
            };
            push(SemanticType::BeltMember, own, key, at, radius, None);
            let shared = band_key(inner, outer, normal);
            push(band, Parent::Cell, shared, [0.0; 3], outer, Some(normal));
        }
        (8, Form::Shell) => {
            push(
                SemanticType::Heliopause,
                Parent::Cell,
                0,
                [0.0; 3],
                orbit,
                None,
            );
        }
        (8 | 9, Form::Orbit { normal }) if point.planet.is_some() => {
            push(SemanticType::Planet, own, key, at, radius, None);
            push(
                SemanticType::OrbitPath,
                own,
                key,
                [0.0; 3],
                orbit,
                Some(normal),
            );
        }
        (9 | 10, Form::Orbit { normal }) if point.moon.is_some() => {
            push(SemanticType::Moon, own, key, at, radius, None);
            push(
                SemanticType::MoonPath,
                own,
                key,
                [0.0; 3],
                orbit,
                Some(normal),
            );
        }
        (9 | 10, Form::Ring { normal, outer, .. }) => {
            push(
                SemanticType::Equator,
                Parent::Cell,
                0,
                at,
                outer,
                Some(normal),
            );
        }
        (9 | 10, Form::Thread { to, .. }) => {
            let half = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt();
            let axis = if half > 0.0 {
                [to[0] / half, to[1] / half, to[2] / half]
            } else {
                [0.0, 1.0, 0.0]
            };
            push(
                SemanticType::SpinAxis,
                Parent::Cell,
                0,
                at,
                half,
                Some(axis),
            );
        }
        _ => {}
    }
}

/// Disk normal of a galaxy form, when it has one.
const fn disk_normal(form: Form) -> Option<[f64; 3]> {
    match form {
        Form::Disk { normal, .. } => Some(normal),
        _ => None,
    }
}

/// Whether `point` at `level` registers at least one part.
#[must_use]
pub fn is_registered(level: Level, point: &Point) -> bool {
    let mut out = Vec::new();
    classify_point(level.get(), 0, point, &mut out);
    !out.is_empty()
}

#[cfg(test)]
#[path = "semantic_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "semantic_fixtures_tests.rs"]
mod fixtures;
