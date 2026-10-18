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
- **Chain.** Planet (`PlanetPos`) → camera-relative (`CameraRel`) →
  `l10_drawn_f32` (`[f32; 3]` cells, the only L10 narrowing): the same math
  the window uses, so headless measurement and drawing share one
  implementation (review item 1). `LocalFrame`/`LocalPos` serve later steps
  (patches #776, walking #779). Per-frame conversions allocate nothing.
  `RenderOrigin` stays the camera every frame (ADR 0026, extends it) and
  carries the L10 frame: `at`, `cell` and `form` route every L10 position
  (camera, body, spot, moons, markers, saved views, rings, threads) through
  the one conversion by themselves, so no call site can miss it (review
  item 2). There is no `at_l10`. Saved-view files keep cell units, so old
  files load to the same place.
- **One frame source (review item 4).** `PlanetFrame::for_planet` from the
  path is the only builder; `sync_camera` recomputes it only when the open
  cell changes (seed-keyed). A missing planet yields no conversion, never an
  Earth fallback.
- **One scale radius (review item 5).** Frame and gravity distances derive
  from `planet_frame::EARTH_RADIUS_M` (mean 6,371 km); gravity's equatorial
  twin is removed. `terrain::EARTH_RADIUS_KM` stays as the IAU home-catalog
  reference; it feeds no scale.
- **Compressed scenery stays (review item 3, owner Q7 A kept).** Moon
  markers keep their 0.42–0.49 shell and the thread its 0.45 half-length:
  true orbits never fit one cell and the true surface itself nearly fills
  it (Earth 0.4946 of 0.5 half-bound; an Earth 1.05× band already reaches
  0.519, and giants span 5.44 cells), so a surface-scaled band cannot fit
  either. They stay compressed indicators inside the true surface until moon
  rendering is reworked with the planet surface in a later step; nothing
  draws the surface yet, so this is invisible. The ring sits at the true
  radius (outer 0.5044 as a size, marginally past the half-bound).
- **Measurement (Q5 A, Q6 A).** `precision::shake` stores vertices as L10
  cell units and runs them through the exact render path (`l10_drawn_f32`),
  reporting worst shake in mm within 1 km (under 1 mm) and in pixels at
  1600×900, 70° FOV farther out (under one pixel), at 1 cm, 1 m, 100 m,
  10 km, 1,000 km and 2,000 km over face centre, edge and corner, on Earth
  plus two seeded planets. The near-mm value is the `f32` rounding of a
  ~1 km vector by construction, hence near-constant across heights and
  planets while the camera-relative pipeline holds (absolute scale leaking
  into the error would be the bug); the gate is real regardless since it
  exercises the render conversion (a wrong scale fails it). The headless
  check prints `PRECISION h0.01=<mm> h1=<mm> h100=<mm>
  h10000=<mm> h1000000=<mm> h2000000=<mm> <PASS|FAIL>` (six decimals,
  byte-identical). Nothing is drawn yet; L10 behaves as after #770.

## Consequences

Extends ADR 0026 (camera-relative drawing now runs through the planet frame
at L10). `E-STDOUT-PROTOCOL` gains the `PRECISION` line; goldens gain it
plus the L10 ring at the true radius (documented in the PR, `E-GOLDEN`).
`frame::L10_PLANET_RADIUS` (0.35) is replaced by `l10_earth_radius_cells()`
and `planet_radius_cells_for`. Cube-face mapping (#775), patches and LOD
(#776), and the overlay (#776) come later.
