//! The semantic marker catalog: one display record per part type (#1128 T2).
//!
//! Every L1–L10 family of the approved catalog (Spec v3 section 4) has one
//! [`SemanticType`] and one static [`PartSpec`]: the developer filter that
//! owns it, its shape family, colour role, the levels it is admitted at, its
//! apparent-size band, view-angle and local rules, label and occlusion rules,
//! budget group and importance. [`crate::semantic`] classifies generated
//! points into these types; rendering reads the specs and never re-derives
//! them. Option B (Spec v3): `baseline` parts are already drawn by today's
//! view, which stays unchanged; the developer layer adds only the rest.

use crate::settings_developer::DeveloperPart;

/// One semantic part type of the approved L1–L10 catalog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SemanticType {
    /// L1: an L2-sized region.
    Region,
    /// L1: the observable-universe boundary.
    UniverseBoundary,
    /// L1: the graticule reference.
    Graticule,
    /// L2: a cluster or group portal.
    ClusterGroup,
    /// L2–L3: a void.
    Void,
    /// L2–L3: a wall sheet.
    Wall,
    /// L2–L3: a filament thread.
    Filament,
    /// L2–L3: a node.
    Node,
    /// L2: the parent region context.
    RegionContext,
    /// L3: a portal galaxy.
    Galaxy,
    /// L3: a population galaxy.
    GalaxyMember,
    /// L3: the parent cluster or group context.
    ClusterContext,
    /// L4: a molecular cloud body (or the home region).
    Cloud,
    /// L4: a cloud boundary ring in the host disk.
    CloudBoundary,
    /// L4: the shared host galaxy disk.
    HostDisk,
    /// L4: the shared host galaxy arms.
    HostArms,
    /// L4: the shared host galaxy bar.
    HostBar,
    /// L5: a bright (portal) star.
    BrightStar,
    /// L5: an ordinary population star.
    Star,
    /// L5: the host cloud and arm context.
    HostContext,
    /// L6: a primary star of a neighbouring system.
    PrimaryStar,
    /// L6, L9: a companion star.
    CompanionStar,
    /// L6: a companion's orbit arc.
    CompanionArc,
    /// L7–L9: the central star.
    CentralStar,
    /// L7–L8: the shared Kuiper belt.
    KuiperBelt,
    /// L7: the shared scattered disc.
    ScatteredDisc,
    /// L8: the shared asteroid belt.
    AsteroidBelt,
    /// L7–L8: one member of a belt or disc.
    BeltMember,
    /// L7: one member of the Oort reservoir.
    OortMember,
    /// L7: the shared Oort reservoir shell.
    OortReservoir,
    /// L7: the system boundary.
    SystemBoundary,
    /// L8–L9: a planet.
    Planet,
    /// L8–L9: a planet's true orbit path.
    OrbitPath,
    /// L8: the heliopause shell.
    Heliopause,
    /// L10 (and an L9 planet interior): a moon.
    Moon,
    /// L10: a moon's path.
    MoonPath,
    /// L10: the planet's equator reference.
    Equator,
    /// L10: the planet's spin-axis reference.
    SpinAxis,
}

/// Shape family: which view-angle and inside rules apply (Spec v3 section 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShapeFamily {
    /// A point or small glyph.
    Point,
    /// A sphere body.
    Sphere,
    /// A spheroid of nested ellipses.
    Spheroid,
    /// A flat disk.
    Disk,
    /// A flat ring band or orbit.
    Ring,
    /// A sphere outline.
    Shell,
    /// Lines between true endpoints.
    Thread,
    /// A flat sheet.
    Sheet,
    /// A clump of dots.
    Clump,
    /// A short arc.
    Arc,
    /// A straight reference axis.
    Axis,
    /// A latitude/longitude reference.
    Graticule,
}

/// Colour role (Spec v3 section 6): a part type, never a random instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ColourRole {
    /// Star blackbody/class or galaxy morphology colour, kept as generated.
    Classification,
    /// The level's era colour, as today's view uses.
    Era,
    /// Void violet.
    Void,
    /// Wall ochre.
    Wall,
    /// Filament cyan.
    Filament,
    /// Node amber.
    Node,
    /// Cloud-boundary teal.
    CloudBoundary,
    /// Belt tan.
    Belt,
    /// Shell blue.
    Shell,
    /// Equator green.
    Equator,
    /// Spin-axis yellow.
    SpinAxis,
    /// Cell-reference grey.
    CellReference,
    /// A muted variant of the owner's colour (orbits and paths).
    OwnerMuted,
}

/// Apparent angular-radius band (radians) with separate enter and exit
/// thresholds, so a part never flickers at its edge (Spec v3 section 3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SizeBand {
    /// Angular radius at which the part enters while growing.
    pub enter: f64,
    /// Angular radius below which it leaves while shrinking; `<= enter`.
    pub exit: f64,
}

/// View-angle rule (Spec v3 section 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ViewRule {
    /// Looks the same from every direction.
    Isotropic,
    /// Full outline face-on, simplifying toward edge-on.
    FaceOn,
    /// Outer silhouette from afar, a boundary portion or fade inside.
    Silhouette,
    /// True endpoints kept; crowding removes secondary connections.
    Endpoints,
    /// Distribution kept with category glyphs.
    Distribution,
}

/// Local-position rule: what happens when the camera is inside or near.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LocalRule {
    /// Nothing special.
    Keep,
    /// Fades once the camera is inside the volume.
    FadeInside,
    /// Fades near its plane.
    FadeNearPlane,
    /// Fades near the ground.
    FadeNearGround,
}

/// Developer label rule; navigation labels are separate and never filtered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LabelRule {
    /// Never labelled by the developer layer.
    Never,
    /// Labelled once readable, under the label cap and collision avoidance.
    WhenReadable,
}

/// Occlusion rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OcclusionRule {
    /// Hidden behind nearer bodies.
    Depth,
    /// Drawn as a faint overlay.
    Overlay,
}

/// Budget group: every group has a fixed cap; populations follow Marker
/// detail instead (Spec v3 section 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BudgetGroup {
    /// Population members, capped by Marker detail.
    Populations,
    /// Object bodies.
    Bodies,
    /// Internal and shared structures.
    Structures,
    /// Orbits, arcs and other paths.
    Paths,
    /// Areas, volumes and boundaries.
    Boundaries,
    /// Axes, graticules and other references.
    References,
}

impl BudgetGroup {
    /// Most parts of this group drawn at once; `None` means Marker detail
    /// decides (populations).
    #[must_use]
    pub const fn cap(self) -> Option<usize> {
        match self {
            BudgetGroup::Populations => None,
            BudgetGroup::Bodies => Some(512),
            BudgetGroup::Structures => Some(128),
            BudgetGroup::Paths => Some(128),
            BudgetGroup::Boundaries => Some(64),
            BudgetGroup::References => Some(16),
        }
    }
}

/// Complete display metadata of one part type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartSpec {
    /// Developer filter that owns the part.
    pub class: DeveloperPart,
    /// Shape family.
    pub shape: ShapeFamily,
    /// Colour role.
    pub colour: ColourRole,
    /// Levels (camera contexts) at which the part is admitted.
    pub levels: &'static [u8],
    /// Apparent-size band.
    pub band: SizeBand,
    /// View-angle rule.
    pub view: ViewRule,
    /// Local-position rule.
    pub local: LocalRule,
    /// Developer label rule.
    pub label: LabelRule,
    /// Occlusion rule.
    pub occlusion: OcclusionRule,
    /// Budget group.
    pub budget: BudgetGroup,
    /// Priority within its budget; higher wins.
    pub importance: u8,
    /// One identity shared by many members (drawn once).
    pub shared: bool,
    /// Already drawn by today's view, which stays unchanged (Spec v3, B).
    pub baseline: bool,
}

/// Bands by role, from the existing projected-size baseline: forms enter at
/// 0.006 rad, previews at 0.02, close at 0.05 (Spec v3 section 3).
const ALWAYS: SizeBand = SizeBand {
    enter: 0.0,
    exit: 0.0,
};
const MEMBER: SizeBand = SizeBand {
    enter: 0.000_5,
    exit: 0.000_4,
};
const FORM: SizeBand = SizeBand {
    enter: 0.006,
    exit: 0.005,
};
const PATH: SizeBand = SizeBand {
    enter: 0.01,
    exit: 0.008,
};
const AREA: SizeBand = SizeBand {
    enter: 0.02,
    exit: 0.016,
};
const REFERENCE: SizeBand = SizeBand {
    enter: 0.05,
    exit: 0.04,
};

/// Shorthand for the catalog rows below.
#[expect(
    clippy::too_many_arguments,
    reason = "E-TOKENS: one catalog row per call keeps the table readable"
)]
const fn spec(
    class: DeveloperPart,
    shape: ShapeFamily,
    colour: ColourRole,
    levels: &'static [u8],
    band: SizeBand,
    view: ViewRule,
    local: LocalRule,
    importance: u8,
    shared: bool,
    baseline: bool,
) -> PartSpec {
    let budget = match class {
        DeveloperPart::Populations => BudgetGroup::Populations,
        DeveloperPart::Bodies | DeveloperPart::Labels => BudgetGroup::Bodies,
        DeveloperPart::Structures => BudgetGroup::Structures,
        DeveloperPart::Paths => BudgetGroup::Paths,
        DeveloperPart::Boundaries => BudgetGroup::Boundaries,
        DeveloperPart::References => BudgetGroup::References,
    };
    let label = match class {
        DeveloperPart::Bodies | DeveloperPart::Structures => LabelRule::WhenReadable,
        _ => LabelRule::Never,
    };
    let occlusion = match class {
        DeveloperPart::Bodies | DeveloperPart::Populations => OcclusionRule::Depth,
        _ => OcclusionRule::Overlay,
    };
    PartSpec {
        class,
        shape,
        colour,
        levels,
        band,
        view,
        local,
        label,
        occlusion,
        budget,
        importance,
        shared,
        baseline,
    }
}

use ColourRole as C;
use DeveloperPart as P;
use LocalRule as Lr;
use ShapeFamily as S;
use ViewRule as V;

impl SemanticType {
    /// Every part type in catalog order.
    pub const ALL: [SemanticType; 38] = [
        SemanticType::Region,
        SemanticType::UniverseBoundary,
        SemanticType::Graticule,
        SemanticType::ClusterGroup,
        SemanticType::Void,
        SemanticType::Wall,
        SemanticType::Filament,
        SemanticType::Node,
        SemanticType::RegionContext,
        SemanticType::Galaxy,
        SemanticType::GalaxyMember,
        SemanticType::ClusterContext,
        SemanticType::Cloud,
        SemanticType::CloudBoundary,
        SemanticType::HostDisk,
        SemanticType::HostArms,
        SemanticType::HostBar,
        SemanticType::BrightStar,
        SemanticType::Star,
        SemanticType::HostContext,
        SemanticType::PrimaryStar,
        SemanticType::CompanionStar,
        SemanticType::CompanionArc,
        SemanticType::CentralStar,
        SemanticType::KuiperBelt,
        SemanticType::ScatteredDisc,
        SemanticType::AsteroidBelt,
        SemanticType::BeltMember,
        SemanticType::OortMember,
        SemanticType::OortReservoir,
        SemanticType::SystemBoundary,
        SemanticType::Planet,
        SemanticType::OrbitPath,
        SemanticType::Heliopause,
        SemanticType::Moon,
        SemanticType::MoonPath,
        SemanticType::Equator,
        SemanticType::SpinAxis,
    ];

    /// Stable short name for logs and developer labels.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            SemanticType::Region => "region",
            SemanticType::UniverseBoundary => "universe-boundary",
            SemanticType::Graticule => "graticule",
            SemanticType::ClusterGroup => "cluster-group",
            SemanticType::Void => "void",
            SemanticType::Wall => "wall",
            SemanticType::Filament => "filament",
            SemanticType::Node => "node",
            SemanticType::RegionContext => "region-context",
            SemanticType::Galaxy => "galaxy",
            SemanticType::GalaxyMember => "galaxy-member",
            SemanticType::ClusterContext => "cluster-context",
            SemanticType::Cloud => "cloud",
            SemanticType::CloudBoundary => "cloud-boundary",
            SemanticType::HostDisk => "host-disk",
            SemanticType::HostArms => "host-arms",
            SemanticType::HostBar => "host-bar",
            SemanticType::BrightStar => "bright-star",
            SemanticType::Star => "star",
            SemanticType::HostContext => "host-context",
            SemanticType::PrimaryStar => "primary-star",
            SemanticType::CompanionStar => "companion-star",
            SemanticType::CompanionArc => "companion-arc",
            SemanticType::CentralStar => "central-star",
            SemanticType::KuiperBelt => "kuiper-belt",
            SemanticType::ScatteredDisc => "scattered-disc",
            SemanticType::AsteroidBelt => "asteroid-belt",
            SemanticType::BeltMember => "belt-member",
            SemanticType::OortMember => "oort-member",
            SemanticType::OortReservoir => "oort-reservoir",
            SemanticType::SystemBoundary => "system-boundary",
            SemanticType::Planet => "planet",
            SemanticType::OrbitPath => "orbit-path",
            SemanticType::Heliopause => "heliopause",
            SemanticType::Moon => "moon",
            SemanticType::MoonPath => "moon-path",
            SemanticType::Equator => "equator",
            SemanticType::SpinAxis => "spin-axis",
        }
    }

    /// Complete display metadata of this part type.
    #[must_use]
    #[rustfmt::skip]
    pub const fn spec(self) -> PartSpec {
        match self {
            SemanticType::Region => spec(P::Bodies, S::Spheroid, C::Era, &[1], ALWAYS, V::Silhouette, Lr::FadeInside, 6, false, true),
            SemanticType::UniverseBoundary => spec(P::Boundaries, S::Shell, C::Shell, &[1], AREA, V::Silhouette, Lr::FadeInside, 3, true, false),
            SemanticType::Graticule => spec(P::References, S::Graticule, C::CellReference, &[1], REFERENCE, V::Isotropic, Lr::FadeInside, 1, true, true),
            SemanticType::ClusterGroup => spec(P::Bodies, S::Spheroid, C::Era, &[2], ALWAYS, V::Silhouette, Lr::FadeInside, 6, false, true),
            SemanticType::Void => spec(P::Boundaries, S::Ring, C::Void, &[2, 3], FORM, V::Silhouette, Lr::FadeInside, 3, false, true),
            SemanticType::Wall => spec(P::Structures, S::Sheet, C::Wall, &[2, 3], FORM, V::FaceOn, Lr::FadeNearPlane, 4, false, true),
            SemanticType::Filament => spec(P::Structures, S::Thread, C::Filament, &[2, 3], FORM, V::Endpoints, Lr::Keep, 4, false, true),
            SemanticType::Node => spec(P::Structures, S::Sphere, C::Node, &[2, 3], FORM, V::Isotropic, Lr::FadeInside, 5, false, true),
            SemanticType::RegionContext => spec(P::Boundaries, S::Shell, C::CellReference, &[2], AREA, V::Silhouette, Lr::FadeInside, 2, true, true),
            SemanticType::Galaxy => spec(P::Bodies, S::Disk, C::Classification, &[3], ALWAYS, V::FaceOn, Lr::FadeInside, 7, false, true),
            SemanticType::GalaxyMember => spec(P::Populations, S::Disk, C::Classification, &[3], MEMBER, V::FaceOn, Lr::Keep, 3, false, true),
            SemanticType::ClusterContext => spec(P::Boundaries, S::Shell, C::CellReference, &[3], AREA, V::Silhouette, Lr::FadeInside, 2, true, true),
            SemanticType::Cloud => spec(P::Bodies, S::Point, C::Era, &[4], ALWAYS, V::Isotropic, Lr::FadeInside, 6, false, true),
            SemanticType::CloudBoundary => spec(P::Boundaries, S::Ring, C::CloudBoundary, &[4], FORM, V::FaceOn, Lr::FadeInside, 4, false, true),
            SemanticType::HostDisk => spec(P::Structures, S::Disk, C::Classification, &[4], FORM, V::FaceOn, Lr::FadeNearPlane, 5, true, false),
            SemanticType::HostArms => spec(P::Structures, S::Thread, C::Classification, &[4], FORM, V::FaceOn, Lr::FadeNearPlane, 4, true, false),
            SemanticType::HostBar => spec(P::Structures, S::Axis, C::Classification, &[4], FORM, V::FaceOn, Lr::FadeNearPlane, 4, true, false),
            SemanticType::BrightStar => spec(P::Bodies, S::Sphere, C::Classification, &[5], ALWAYS, V::Isotropic, Lr::Keep, 7, false, true),
            SemanticType::Star => spec(P::Populations, S::Point, C::Classification, &[5], MEMBER, V::Isotropic, Lr::Keep, 3, false, true),
            SemanticType::HostContext => spec(P::Structures, S::Disk, C::CloudBoundary, &[5], AREA, V::FaceOn, Lr::FadeInside, 2, true, true),
            SemanticType::PrimaryStar => spec(P::Bodies, S::Sphere, C::Classification, &[6], ALWAYS, V::Isotropic, Lr::Keep, 7, false, true),
            SemanticType::CompanionStar => spec(P::Bodies, S::Sphere, C::Classification, &[6, 9], ALWAYS, V::Isotropic, Lr::Keep, 5, false, true),
            SemanticType::CompanionArc => spec(P::Paths, S::Arc, C::OwnerMuted, &[6], PATH, V::FaceOn, Lr::FadeNearPlane, 3, false, true),
            SemanticType::CentralStar => spec(P::Bodies, S::Sphere, C::Classification, &[7, 8, 9], ALWAYS, V::Isotropic, Lr::Keep, 8, false, true),
            SemanticType::KuiperBelt => spec(P::Structures, S::Ring, C::Belt, &[7, 8], FORM, V::FaceOn, Lr::FadeNearPlane, 4, true, true),
            SemanticType::ScatteredDisc => spec(P::Structures, S::Ring, C::Belt, &[7], FORM, V::FaceOn, Lr::FadeNearPlane, 3, true, true),
            SemanticType::AsteroidBelt => spec(P::Structures, S::Ring, C::Belt, &[8], FORM, V::FaceOn, Lr::FadeNearPlane, 4, true, true),
            SemanticType::BeltMember => spec(P::Populations, S::Point, C::Belt, &[7, 8], MEMBER, V::Distribution, Lr::Keep, 1, false, true),
            SemanticType::OortMember => spec(P::Populations, S::Point, C::Shell, &[7], MEMBER, V::Distribution, Lr::Keep, 1, false, true),
            SemanticType::OortReservoir => spec(P::Boundaries, S::Shell, C::Shell, &[7], AREA, V::Silhouette, Lr::FadeInside, 3, true, false),
            SemanticType::SystemBoundary => spec(P::Boundaries, S::Shell, C::Shell, &[7], AREA, V::Silhouette, Lr::FadeInside, 2, true, false),
            SemanticType::Planet => spec(P::Bodies, S::Sphere, C::Era, &[8, 9], ALWAYS, V::Isotropic, Lr::Keep, 7, false, true),
            SemanticType::OrbitPath => spec(P::Paths, S::Ring, C::OwnerMuted, &[8, 9], PATH, V::FaceOn, Lr::FadeNearGround, 4, false, true),
            SemanticType::Heliopause => spec(P::Boundaries, S::Shell, C::Shell, &[8], AREA, V::Silhouette, Lr::FadeInside, 2, true, true),
            SemanticType::Moon => spec(P::Bodies, S::Sphere, C::Era, &[10, 9], ALWAYS, V::Isotropic, Lr::Keep, 6, false, true),
            SemanticType::MoonPath => spec(P::Paths, S::Ring, C::OwnerMuted, &[10, 9], PATH, V::FaceOn, Lr::FadeNearGround, 3, false, true),
            SemanticType::Equator => spec(P::References, S::Ring, C::Equator, &[10, 9], REFERENCE, V::FaceOn, Lr::FadeNearGround, 2, true, true),
            SemanticType::SpinAxis => spec(P::References, S::Axis, C::SpinAxis, &[10, 9], REFERENCE, V::Isotropic, Lr::FadeNearGround, 2, true, true),
        }
    }
}
