# 0030 — One metre per planet

- Status: proposed
- Date: 2026-10-14
- Issue: #648 (Spec v1, decisions D1-D4, Telegram 2026-10-14)
- Supersedes: point 4 of ADR 0028 (heights in the ladder metre)

## Context

Every planet is drawn with the same radius, `PLANET_RADIUS_CELL` = 0.35
L10 cell units, whatever its real size. Since #632 the heights, the L11-L14
shells, the body, the squares and the planet LOD measured in the ladder
metre, `body::metre(L10)`: one L10 unit is 10^7.11 m, about 12,883 km. So
Earth's ground sat 0.35 × 12,883 = 4,509 km from its centre instead of
6,371 km, and every planet came out at 4,509 km. Gravity already used a
different metre, the real radius over the drawn radius. The planet lab
(#640) made the gap visible.

## Decision

1. One metre per planet:
   `shell::metres_per_unit(radius_earth) = radius_earth × EARTH_RADIUS_KM × 1,000 / PLANET_RADIUS_CELL`.
   A planet's ground corners sit at its real radius. Heights, shell enter
   and leave heights, the ground standoff, the dive's rest point, the
   body, the squares, the LOD's metre-based tests, gravity and the lab
   readouts all use it.
2. The open planet's radius comes from where it is already held:
   `MarkerPath` (`planet_metres_per_unit`), `SurfaceFrame`
   (`metres_per_unit`, from its record), `body::metre_at(&Universe)`, and
   `PlayerSpot::metre(root)`. Off a planet the ladder metre stays.
3. Since #632 everything below the planet is in the L10 cell, so one
   planet metre covers L10-L14 with no per-level factor.
4. The drawing doesn't change: `PLANET_RADIUS_CELL`, `LADDER_EXPONENTS`,
   the markers and the moons are as before. Things fixed in metres are
   1.41 times smaller in units on Earth: the body, the standoff, the
   squares and the shell heights.
5. One Earth radius: the mean 6,371 km (`terrain::EARTH_RADIUS_KM`;
   `gravity::EARTH_RADIUS_M` derives from it), in the game and the lab.

## Consequences

- Earth's ground reads 6,371 km at the face corners and about 5,950 to
  6,015 km in the middle of a face (the faces are flat). A 0.3-Earth
  planet is 1,911 km, and a giant is 70,081 km.
- Walking across a given part of a planet takes about 1.41 times longer
  on Earth, because speeds stay in m/s. The L11-L14 changes come at the
  heights the Navigation help gives, about 1.41 times closer to the ground
  than before compared with the planet's size.
- Earth's surface gravity moves from 9.80 to 9.82 m/s² with the mean
  radius. Air thickness ratios move by 0.1 %.
- The golden L11-L14 lines move (the dive passes more metres); L1-L10 are
  byte-identical.
- The planet LOD's finest depth-24 edge is about 40 cm on Earth, up from
  about 28 cm, still well under the 1.8 m body.

## Alternatives

- Change the L10 ladder exponent to 7.26 so Earth fits. Rejected: it moves
  the L9/L10/L11 ratios and every marker, and other planets would still be
  wrong.
- Draw each planet at its real size in cells (0.49 for Earth). Rejected:
  Earth would barely fit its cell, and larger planets would not fit.
