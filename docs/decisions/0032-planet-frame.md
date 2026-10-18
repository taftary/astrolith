# 0032 — Planet frame in metres (rebuild step 1)

- Status: accepted
- Date: 2026-10-18
- Issue: #774

## Context

Step 1 of the cube-sphere rebuild (#770 Phase 2) needs one exact coordinate
system to build on. At L10 every position was an `f64` in open-cell units;
drawing was already camera-relative (ADR 0026), but there was no
planet-centred frame in real units, no local frames derived from it, and no
explicit conversion chain. Planning found two different metres at L10: the
body metre (ladder, 1 unit = 10^7.11 m) and the planet metre (0.35 units =
6,371 km). Owner answers Q1-Q7 (all A) pick one metre, one frame, and
measured targets over the future cube faces.

## Decision

- **Frame (Q1 A, Q2 A, Q3 A).** One `PlanetFrame` per L10 planet in metres,
  `f64`, centred on the planet and fixed to it (no spin; `tilt_deg` and
  `spin_hours` stay recorded for step 7, #780). Radius comes from the cell's
  own `body_recipe` (home is Earth, 6,371 km); same seed, same frame.
  Conversion between L10 cell units and metres happens in `planet_frame.rs`
  only (`from_cell` / `to_cell`).
- **One L10 metre (Q7 A).** The ladder's `body::metre(L10)` wins. A planet's
  radius in cells is its true radius from its record (Earth about 0.49);
  gravity and the spawn spot move to that true surface, so the pull stays
  Earth 9.81 m/s². Moons keep their compressed places. The 0.35 literal is
  gone.
- **Local frame.** `LocalFrame::at` gives radial up, north toward `+Y`
  (`+Z` fallback at the poles), and east completing the right-handed frame;
  all unit and orthogonal. Later steps use it for patches (#776) and walking
  (#779).
- **Chain.** Planet (`PlanetPos`) → local (`LocalPos`) → camera-relative
  (`CameraRel`) → `to_gpu()` (`[f32; 3]`), the only narrowing. Per-frame
  conversions allocate nothing. `RenderOrigin` stays the camera every frame
  (ADR 0026, extends it); at L10 `at_l10` routes camera, body, spot, moons,
  markers and saved views through the one conversion. Saved-view files keep
  cell units, so old files load to the same place.
- **Measurement (Q5 A, Q6 A).** `precision::shake` reports worst shake in mm
  within 1 km (under 1 mm) and in pixels at 1600×900, 70° FOV farther out
  (under one pixel), at 1 cm, 1 m, 100 m, 10 km, 1,000 km and 2,000 km over
  face centre, edge and corner, on Earth plus two seeded planets. The
  headless check prints `PRECISION h0.01=<mm> h1=<mm> h100=<mm>
  h10000=<mm> h1000000=<mm> h2000000=<mm> <PASS|FAIL>` (six decimals,
  byte-identical). Nothing is drawn yet; L10 behaves as after #770.

## Consequences

Extends ADR 0026 (camera-relative drawing now runs through the planet frame
at L10). `E-STDOUT-PROTOCOL` gains the `PRECISION` line; goldens gain it
plus the L10 ring at the true radius (documented in the PR, `E-GOLDEN`).
`frame::L10_PLANET_RADIUS` (0.35) is replaced by `l10_earth_radius_cells()`
and `planet_radius_cells_for`. Cube-face mapping (#775), patches and LOD
(#776), and the overlay (#776) come later.
