# 0033 — Cube-sphere planet (rebuild step 2)

- Status: accepted
- Date: 2026-10-18
- Issue: #775

## Context

Step 2 of the cube-sphere rebuild (#770 Phase 2) brings the planet back as
six cube faces bent onto the sphere, building on the planet frame of step 1
(ADR 0032). Owner answers Q1-Q7 (all A) pick a tangent-adjusted mapping with
an exact backward step, a plain smooth sphere on screen at fixed resolution,
no relief, moons outside the true radius, exact zero-crack seams, a hard
1.5 spacing limit, and the planet's own space for giants.

## Decision

- **Mapping (Q1 A).** New core module `cube_sphere.rs` (std only,
  `E-CORE-NO-BEVY`): `Face` (`PosX`, `NegX`, `PosY`, `NegY`, `PosZ`, `NegZ`;
  `E-ENUM-BOOL`), `FacePos { face, u, v }` with `u`, `v` in `[-1, 1]`,
  `to_sphere` (`tan(u·pi/4)`, `tan(v·pi/4)` on the face plane, then
  normalised), `from_sphere` (dominant axis picks the face,
  `atan·4/pi`, exact), positions as `PlanetPos` at `radius_m x dir`.
- **Seams (Q5 A, Q3 A).** `grid_point(face, i, j, n)` folds every edge and
  corner index to one canonical face before computing, so both faces get the
  same bits; normals use the same path and the test-only height `h(dir)` is
  a pure function of the direction.
- **Spacing (Q6 A).** `spacing(n)` walks grid neighbours over all six faces
  (min, max, ratio plus centre/edge/corner samples); the tangent mapping at
  `64 x 64` measures about `1.414`, under the `1.5` limit.
- **Drawing (Q2 A).** `planet.rs` gains `draw_planet_sphere`: one Bevy `Mesh`
  per face at fixed `SPHERE_GRID` (`64 x 64` quads, engine constant for
  #776), built once per open cell (seed-keyed), one unlit flat colour,
  vertices through the one conversion relative to an `Anchored` origin
  (`E-HOT-NOALLOC`).
- **Planet space (Q7 A, Q4 A, review items 2-5).** `planet_space` gives
  `radius_cells`, `arrival = 3 x radius` (floor only, no entry rest, so
  off-centre window entries never jump; Earth's journey entry already clears
  it), `reach = max(0.5, 4 x radius)`; `nav` pushes entry out to arrival only
  when inside it, `should_close` at L10 uses `max(0.5, radius)`, moons sit at
  `radius x (1.5 + 2.5 x share)` in order, the ring spans `1.02x` to `1.06x`
  (inner `1.02x`, outer `1.06x`), the spin-axis thread pokes out at `1.2x`,
  and the headless in-sphere check at L10 measures against the reach.
  `Universe` caches the seed and planet space in `reload` so the dive close
  check allocates nothing per step (`E-HOT-NOALLOC`).
- **Headless.** `verify_cube_sphere` prints
  `CUBE-SPHERE seams=0 spacing=<ratio> roundtrip=<µm> PASS`; goldens gain it
  plus the moon and ring moves (documented in the PR, `E-GOLDEN`).

## Consequences

Extends ADR 0032 (planet frame now carries the cube-sphere). `E-STDOUT-PROTOCOL`
gains the `CUBE-SPHERE` line; `E-RENDER-NO-MESH` gains the six face meshes
(one flat-tone sphere plus one body per moon at L10). Patches and LOD (#776),
morphing (#777), depth/culling (#778), ground (#779), and light (#780) come
later.
