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
//! - [`settings_camera`]: camera presets of the settings, look smoothing and behind follow (#899).
//! - [`settings_ground`]: the Ground memory limit of the settings (#918).
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
//! - [`clock`]: the sky's clock and pure orbit and spin angles (#981).
//! - [`sky_motion`]: planets and moons moving with the clock (#981).
//! - [`system_space`]: one star system in one space at true scale (#995).
//! - [`system_handover`]: star, orbit and spin frame handovers (#995).
//! - [`headless_handover`]: the `HANDOVER` verify line (#995).
//! - [`headless_ride`]: the `RIDE` verify line (#981).
//! - [`headless_clock`]: the `ORBIT` and `CLOCK` verify lines (#981).
//! - [`grid_rebuild`], [`grid_tile`]: the shared ground grid (#920).
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

/// Dive orbit and turn math: same-distance circling and in-place roll (#873).
pub mod dive_orbit;

/// Free-flight synthesis and the scripted headless leg (#152).
pub mod flight;

/// The player's low-poly figure: head, torso, arms, legs (#755).
pub mod figure;

/// The planet's air as one atmosphere model and its CPU reference sky (#780).
pub mod atmosphere;
/// Headless `LIGHT` line: light and atmosphere from orbit to the ground (#780).
pub mod headless_light;
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

/// Headless `GROUND` line for near-field physics (#779 T3).
pub mod headless_ground;

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

/// Planet dive tests: arrival, close, no pass-through and planet-centre target (split from nav under E-FILE-SIZE).
#[cfg(test)]
mod nav_planet_tests;

/// Planet-to-ground dive tests: no sea level, real-ground stop, slow-down (#892).
#[cfg(test)]
mod nav_ground_tests;

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

/// The ground ring and the nearest-first build order (#918 T2).
pub mod lod_ring;

/// Ring and build order tests (#918 T2).
#[cfg(test)]
mod lod_ring_tests;

/// Headless `RING` line (#918 T2).
pub mod headless_ring;

/// Quick-turn ground coverage probe (#948 T1).
#[cfg(test)]
mod headless_quick_turn;

/// Shared-grid precision probe (#920 T1).
#[cfg(test)]
mod headless_grid_precision;

/// Shared-grid `f32` rebuild (#920 T1).
pub mod grid_rebuild;

/// One patch's tile in the shared grid's heights texture (#920).
pub mod grid_tile;

/// The sky's clock (#981).
pub mod clock;

/// Moving the star system with the sky's clock (#981).
pub mod sky_motion;

/// One star system in one space at true scale (#995 study).
pub mod system_space;

/// Frame handovers inside one star system (#995 study).
pub mod system_handover;

/// The `HANDOVER` verify line (#995 T8).
pub mod headless_handover;

/// The `RIDE` verify line (#981 T2).
pub mod headless_ride;

/// The `ORBIT` and `CLOCK` verify lines (#981 T3).
pub mod headless_clock;

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

/// The drawn ground as one surface for the body (#779 T1).
pub mod drawn_surface;

/// The body's local tangent frame near the ground (#779 T2).
pub mod body_frame;

/// Local frame tests (#779 T2).
#[cfg(test)]
mod body_frame_tests;

/// Drawn ground tests (#779 T1).
#[cfg(test)]
mod drawn_surface_tests;

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

/// Free flight passes through levels (#998 build T4).
pub mod free_pass;

/// What a continuous zoom draws, and the fade by size (#998 build T2).
pub mod drawn_set;

/// One zoom speed both ways (#998 build T1).
pub mod zoom_speed;

/// The continuous zoom in the headless check (#998 build T7).
pub mod headless_continuity;

/// Continuous scaling study through L1-L7 (#998 T1, study).
pub mod headless_scale;

/// Continuous scaling study, crowds and budget (#998 T2, study).
pub mod headless_scale_crowd;

/// The local hour where you stand (#981 correction round 2).
pub mod local_time;

/// Night light from the moons and stars (#981 correction round 2).
pub mod moonlight;

/// Parent-to-child height reuse between patch builds (#994 T3).
pub mod lod_reuse;

/// The fixed patch set the build speed is measured on (#994 T1).
pub mod patch_bench;

/// Precision measurement for the planet frame (#774, Q5 A, T3).
pub mod precision;

/// Marker-tree nesting: every marker is the next level's cell (R6, #58).
pub mod nest;

/// L7-L9 system layouts: rings, shells, orbits, and the star body (#384).
pub mod orbits;

/// Pre-entry previews: interiors drawn before entry (R7, #63; #152 streaming).
pub mod preview;

/// Effective graphics quality that Auto adjusts (#1022 T5).
pub mod quality;
/// The Auto quality controller (#1022 T5).
pub mod quality_controller;
/// Trace tests of the quality controller (#1022 T5).
#[cfg(test)]
mod quality_tests;
/// One-second observation windows of the quality controller (#1022 T5).
pub mod quality_window;

/// Deterministic seed derivation and pseudo-random numbers.
pub mod seed;

/// Player settings, ranges, defaults, and the settings file format (#473).
pub mod settings;

/// Camera presets of the settings (look smoothing, behind follow), re-exported from `settings` (#899).
pub mod settings_camera;
/// Ground memory limit of the settings, re-exported from `settings` (#918).
pub mod settings_ground;
/// Graphics quality preferences of the settings, re-exported from `settings` (#1022).
pub mod settings_quality;

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
