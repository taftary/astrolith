//! Pure deterministic universe logic (M2 core framework, sub-issue #39,
//! plus M3-M5 content levels, sub-issues #40-#42).
//!
//! This crate has zero Bevy and zero rendering dependencies — standard
//! library only — so it stays headless-testable. It implements the M2 core
//! framework from spec v4 (Issue #35):
//!
//! - [`astro`]: galaxy-to-star sampling with anonymous octree cells (M4).
//! - [`body`]: the player body in normal mode: walk, jump, fly, turn (#558).
//! - [`contrast`]: contrast, colour-vision, and brightness-ladder rules.
//! - [`coords`]: nested integer-cell coordinates and floating-origin frames
//!   (naming any place, moving the origin without jumps).
//! - [`density`]: L1-L3 density field and cluster placement (M3).
//! - [`destination`]: the camera's final destination, a seeded surface chain (#423).
//! - [`figure`]: the player's low-poly figure: head, torso, arms, legs (#755).
//! - [`light`]: star light direction and face shading for the figure (#755).
//! - [`motion`]: gradual velocity and heading response for the body (#756).
//! - [`flight`]: free-flight synthesis and the scripted headless leg (#152).
//! - [`frame`]: observer-frame geometry: orientation, planet radius, horizon
//!   (split from `nest` under `E-FILE-SIZE`, #394).
//! - [`gravity`]: real gravity from every massive body around the player (#559).
//! - [`guides`]: player guides text, About and Help pages (#473).
//! - [`names`]: seeded marker names and label text (#446).
//! - [`pick`]: pick ranking, click cycling, and the label rule (#446).
//! - [`settings`]: player settings, ranges, defaults, and the file format (#473).
//! - [`seed`]: deterministic seed derivation `H(parent, level, x, y, z)` and
//!   the matching small PRNG (regenerating a place identically every time).
//! - [`cache`]: fixed-capacity LRU cell store (nearby places kept, far ones
//!   dropped).
//! - [`gen`]: pure generator contracts turning `(seed, constraints)` into
//!   indicator content.
//! - [`headless`]: headless `--verify` report builder shared by every gate
//!   binary.
//! - [`headless_relief`]: headless `RELIEF` line for the terrain relief (#799).
//! - [`home`]: journey matching, home fixtures, and the autopilot (split from
//!   `nest`, #156).
//! - [`keymap`]: which key or mouse button does each action (#560).
//! - [`labels`]: level labels, scale readouts, and window titles.
//! - [`layouts`]: L4-L6 marker layouts: galaxy forms, cloud rings, star radii (#384).
//! - [`nav`]: navigation state machine, dive math, and the headless journey
//!   replay.
//! - [`nest`]: marker-tree nesting, where every marker is the next level's
//!   cell at the true ladder ratio, with the observer's marker path (R6).
//! - [`orbits`]: L7-L9 system layouts: rings, shells, orbits, star bodies (#384).
//! - [`cube_sphere`]: tangent-adjusted cube-to-sphere mapping plus seams (#775).
//! - [`lod`]: quadtree LOD per cube face: patches and selection (#776).
//! - [`lod_mesh`]: patch meshes with centre origins (#776).
//! - [`relief`]: deterministic terrain relief on the cube-sphere (#799).
//! - [`planet_frame`]: one planet-centred metre frame per L10 planet (#774).
//! - [`precision`]: planet-frame shake measurement and the `PRECISION` line (#774).
//! - [`preview`]: pre-entry preview cache, merge, and preview-set computation.
//! - [`snapshot`]: canonical text snapshots for `--verify` mode (M3-M5).
//! - [`stream`]: streaming working-set ledger with longest-unseen-first unload.
//! - [`terrain`]: L10 planet record, body recipe, and sampler (#770: surface
//!   removed, rebuild in #774-#780).
//! - [`touch`]: touch layout and gesture math: stick, buttons, pinch, tap
//!   (#561).
//! - [`verify`]: determinism and border-agreement predicates backing the
//!   automated checks and the headless `--verify` mode.

/// Anonymous octree and galaxy-to-star sampling (M4, sub-issue #41).
pub mod astro;
/// The player body in normal mode (#558).
pub mod body;

/// Fixed-capacity least-recently-used cell cache.
pub mod cache;

/// Nested integer-cell coordinates and floating-origin frames.
pub mod coords;

/// L1-L3 density field and cluster placement (M3, sub-issue #40).
pub mod density;

/// Final destination: the surface the camera always aims at (#423).
pub mod destination;

/// Free-flight synthesis and the scripted headless leg (#152).
pub mod flight;

/// The player's low-poly figure: head, torso, arms, legs (#755).
pub mod figure;

/// Star light direction and face shading for the player figure (#755).
pub mod light;

/// Gradual velocity and heading response for the player body (#756).
pub mod motion;

/// Real gravity from every massive body around the player (#559).
pub mod gravity;
/// Player guides text: About and Help pages (#473).
pub mod guides;

/// Observer-frame geometry: orientation, planet radius, horizon (#394).
pub mod frame;
/// Pure procedural generation contracts.
pub mod r#gen;

/// Headless `--verify` report builder shared by every gate binary.
pub mod headless;

/// Headless `RELIEF` line for the terrain relief (#799, T4).
pub mod headless_relief;

/// Headless `MORPH` line for cracks and morphing (#777 T4).
pub mod headless_morph;

/// Headless `CULL` and `DEPTH` lines for depth and culling (#778 T5).
pub mod headless_cull;

/// Crack, popping and normal measures on blended frames (#777 review).
pub(crate) mod headless_morph_measure;

/// Headless `LOD` line for the quadtree LOD (#776, split from `headless`).
pub(crate) mod headless_lod;

/// Journey matching, home fixtures, and the autopilot (split from `nest`).
pub mod home;

/// Level labels, scale readouts, and window titles.
/// Which key or mouse button does each action (#560).
pub mod keymap;
pub mod labels;

/// L4-L6 marker layouts: galaxy forms, cloud rings, star radii (#384).
pub mod layouts;

/// Contrast, colour-vision, and brightness-ladder rules (#446).
pub mod contrast;

/// Seeded marker names and label text (#446).
pub mod names;

/// Pick ranking, click cycling, and the label rule (#446).
pub mod pick;
/// Where the player stands: the seeded spawn and the dive to it (#607).
pub mod player;

/// Headless navigation state machine, dive math, and journey replay.
pub mod nav;

/// One planet-centred coordinate system per L10 planet (#774, step 1).
pub mod planet_frame;

/// Cube-sphere mapping for the L10 planet: six faces onto the sphere (#775).
pub mod cube_sphere;

/// Quadtree LOD per cube face: patches, screen error, selection (#776).
pub mod lod;

/// A patch's 3D box at its real height range (#817 T1).
pub mod lod_box;

/// CDLOD distance ranges per level, scaled by relief roughness (#817 T2).
pub mod lod_range;

/// Range tests (#817 T2).
#[cfg(test)]
mod lod_range_tests;

/// CDLOD selection tests (#817 T3).
#[cfg(test)]
mod lod_cdlod_tests;

/// Box distance tests (#817 T1).
#[cfg(test)]
mod lod_box_tests;

/// Quadtree LOD selection and updates (split from `lod` under `E-FILE-SIZE`, #777 T1).
pub mod lod_select;

/// Geomorph blending between LOD levels (CDLOD, #777 T2).
pub mod lod_morph;

/// Frustum and horizon visibility for the LOD selection (#778 T1).
pub mod lod_cull;

/// Visibility tests (#778 T1).
#[cfg(test)]
mod lod_cull_tests;

/// Culled selection tests (#778 T2).
#[cfg(test)]
mod lod_cull_select_tests;

/// Near-plane tests (#778 T4).
#[cfg(test)]
mod near_plane_tests;

/// Player up stopgap tests (#778 F2 A).
#[cfg(test)]
mod body_up_tests;

/// Quadtree LOD tests (split from `lod.rs` under `E-FILE-SIZE`, #776 review).
#[cfg(test)]
mod lod_tests;

/// Patch meshes on the quadtree: centre plus `f32` offsets (#776).
pub mod lod_mesh;

/// Morph shape, terrain normals and skirts of one patch (#777, split from `lod_mesh`).
pub mod lod_patch_morph;

/// Morph shape tests (#777 review item 7).
#[cfg(test)]
mod lod_patch_morph_tests;

/// Winding tests: every patch triangle faces out (#778 T3).
#[cfg(test)]
mod lod_winding_tests;

/// Deterministic terrain relief on the cube-sphere (rebuild step 3b, #799).
pub mod relief;

/// Precision measurement for the planet frame (#774, Q5 A, T3).
pub mod precision;

/// Marker-tree nesting: every marker is the next level's cell (R6, #58).
pub mod nest;

/// L7-L9 system layouts: rings, shells, orbits, and the star body (#384).
pub mod orbits;

/// Pre-entry previews: interiors drawn before entry (R7, #63; #152 streaming).
pub mod preview;

/// Deterministic seed derivation and pseudo-random numbers.
pub mod seed;

/// Player settings, ranges, defaults, and the settings file format (#473).
pub mod settings;

/// Settings unit tests (split from `settings.rs` under `E-FILE-SIZE`, #776).
#[cfg(test)]
mod settings_tests;

/// Canonical text snapshots for `--verify` mode and validator reads.
pub mod snapshot;

/// Streaming working set: bound accounting with longest-unseen-first unload (#152).
pub mod stream;

/// L4-L9 indicator generator: systems, stars, planets, clouds (#155).
pub mod sysgen;

/// Stellar and planetary sampling for L5-L8: types, companions, planets, clouds (#155).
pub mod system;

/// L10 planet record, body recipe, and sampler (#770: surface removed,
/// rebuild in #774-#780).
pub mod terrain;

/// Touch layout and gesture math (#561).
pub mod touch;

/// Headless verification helpers for the automated checks.
pub mod verify;

// Shared hash-based value-noise primitive (private to this crate).
mod noise;
