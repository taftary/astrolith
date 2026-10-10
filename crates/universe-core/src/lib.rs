//! Pure deterministic universe logic (M2 core framework, sub-issue #39,
//! plus M3-M5 content levels, sub-issues #40-#42).
//!
//! This crate has zero Bevy and zero rendering dependencies — standard
//! library only — so it stays headless-testable. It implements the M2 core
//! framework from spec v4 (Issue #35):
//!
//! - [`astro`]: galaxy-to-star sampling with anonymous octree cells (M4).
//! - [`contrast`]: contrast, colour-vision, and brightness-ladder rules.
//! - [`coords`]: nested integer-cell coordinates and floating-origin frames
//!   (naming any place, moving the origin without jumps).
//! - [`density`]: L1-L3 density field and cluster placement (M3).
//! - [`destination`]: the camera's final destination, a seeded room chain (#423).
//! - [`flight`]: free-flight synthesis and the scripted headless leg (#152).
//! - [`frame`]: observer-frame geometry: orientation, planet radius, horizon
//!   (split from `nest` under `E-FILE-SIZE`, #394).
//! - [`names`]: seeded marker names and label text (#446).
//! - [`seed`]: deterministic seed derivation `H(parent, level, x, y, z)` and
//!   the matching small PRNG (regenerating a place identically every time).
//! - [`cache`]: fixed-capacity LRU cell store (nearby places kept, far ones
//!   dropped).
//! - [`gen`]: pure generator contracts turning `(seed, constraints)` into
//!   indicator content.
//! - [`headless`]: headless `--verify` report builder shared by every gate
//!   binary.
//! - [`home`]: journey matching, home fixtures, and the autopilot (split from
//!   `nest`, #156).
//! - [`labels`]: level labels, scale readouts, and window titles.
//! - [`layouts`]: L4-L6 marker layouts: galaxy forms, cloud rings, star radii (#384).
//! - [`nav`]: navigation state machine, dive math, and the headless journey
//!   replay.
//! - [`nest`]: marker-tree nesting, where every marker is the next level's
//!   cell at the true ladder ratio, with the observer's marker path (R6).
//! - [`orbits`]: L7-L9 system layouts: rings, shells, orbits, star bodies (#384).
//! - [`preview`]: pre-entry preview cache, merge, and preview-set computation.
//! - [`snapshot`]: canonical text snapshots for `--verify` mode (M3-M5).
//! - [`stream`]: streaming working-set ledger with longest-unseen-first unload.
//! - [`tail`]: sparse tail generators for L11-L14 (regions, cities,
//!   buildings, room; #375).
//! - [`terrain`]: L10 cube-sphere terrain and LOD selection (M5; bare planet
//!   since #375).
//! - [`verify`]: determinism and border-agreement predicates backing the
//!   automated checks and the headless `--verify` mode.

/// Anonymous octree and galaxy-to-star sampling (M4, sub-issue #41).
pub mod astro;

/// Fixed-capacity least-recently-used cell cache.
pub mod cache;

/// Nested integer-cell coordinates and floating-origin frames.
pub mod coords;

/// L1-L3 density field and cluster placement (M3, sub-issue #40).
pub mod density;

/// Final destination: the room the camera always aims at (#423).
pub mod destination;

/// Free-flight synthesis and the scripted headless leg (#152).
pub mod flight;

/// Ground clearance for window dives and window free flight (#430).
pub mod ground;

/// Observer-frame geometry: orientation, planet radius, horizon (#394).
pub mod frame;
/// Window-only landing approach for surface portals (#430).
pub mod landing;

/// Pure procedural generation contracts.
pub mod r#gen;

/// Headless `--verify` report builder shared by every gate binary.
pub mod headless;

/// Journey matching, home fixtures, and the autopilot (split from `nest`).
pub mod home;

/// Level labels, scale readouts, and window titles.
pub mod labels;

/// L4-L6 marker layouts: galaxy forms, cloud rings, star radii (#384).
pub mod layouts;

/// Contrast, colour-vision, and brightness-ladder rules (#446).
pub mod contrast;

/// Seeded marker names and label text (#446).
pub mod names;

/// Headless navigation state machine, dive math, and journey replay.
pub mod nav;

/// Marker-tree nesting: every marker is the next level's cell (R6, #58).
pub mod nest;

/// L7-L9 system layouts: rings, shells, orbits, and the star body (#384).
pub mod orbits;

/// Pre-entry previews: interiors drawn before entry (R7, #63; #152 streaming).
pub mod preview;

/// Deterministic seed derivation and pseudo-random numbers.
pub mod seed;

/// Canonical text snapshots for `--verify` mode and validator reads.
pub mod snapshot;

/// Streaming working set: bound accounting with longest-unseen-first unload (#152).
pub mod stream;

/// L4-L9 indicator generator: systems, stars, planets, clouds (#155).
pub mod sysgen;

/// Stellar and planetary sampling for L5-L8: types, companions, planets, clouds (#155).
pub mod system;

/// Sparse tail generators for L11-L14: regions, cities, buildings, room (#375).
pub mod tail;

/// L10 cube-sphere terrain and LOD selection (M5, sub-issue #42; bare planet
/// since #375).
pub mod terrain;

/// Headless verification helpers for the automated checks.
pub mod verify;

// Shared hash-based value-noise primitive (private to this crate).
mod noise;
