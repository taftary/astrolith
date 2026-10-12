# 0025 — One planet record, shaped rooms, and floor furniture

- Status: proposed
- Date: 2026-10-12
- Issue: #543

## Context

The planet-to-room levels carried weight that showed nothing and content
that read wrong. A review by several agents found these problems:

- An L10 cell emitted about 21 surface sample points that were never drawn.
  Each repeated the same cell-wide recipe (tilt, spin, size, air), and every
  reader took only the first one. Reading the air context for L11-L14
  regenerated the whole L10 cell.
- The recipe carried a flattening that nothing showed: the icosphere surface
  is round.
- L9 planet interiors placed region portals at cluster means, while L10 used
  anchor tiles. That is two implementations of one rule.
- The equator ring sat at mid-relief, so the surface hid about half of it.
- L14 furniture came from a uniform scatter in the whole cell, so most of it
  floated outside the fixed room shell. The shell's wide, flat shape ignored
  the building it was entered from, which is a tower one to four times
  taller than wide.

## Decision

1. Each planet cell emits one surface record (`SURFACE_RECORDS = 1`) at the
   north pole on the relief. The recipe is a pure function
   `terrain::body_recipe(seed, home)`. The L11-L14 air context reads it from
   the planet's seed without generating the L10 cell.
2. Flattening is removed from `BodyRecipe`, `SurfaceInfo`, and the snapshot
   `T` token (owner answer Q1 = B). The class draw that picked the planet
   size stays on the same stream with the same thresholds, so planet sizes,
   air, and moons do not move.
3. L9 and L10 both place region portals on `SurfaceFrame` anchor tiles.
   `region_clusters`, `cluster_direction`, and `REGION_DEPTH` are deleted.
   Every cell has its own seed, so no L9-to-L10 jump existed; this is
   cleanup.
4. The equator ring spans `PLANET_RADIUS_CELL + RELIEF_RANGE_CELL` outward
   by 2% of the radius, above the highest relief and below the moon band.
5. The L14 room shell takes the entered building's height-to-width ratio
   (`TailGenerator::building_height` over `2 * TAIL_POINT_RADIUS`, read from
   the building's own lane). It is fitted so its corner reaches 0.48. The
   eleven furniture pieces stand on the shell floor inside its walls, one
   seeded lane per piece. Each is sized by its kind (`FURNITURE_KINDS`:
   bed, sofa, table, desk, chair), scaled with the room width.

## Consequences

- Goldens change on purpose: the L10-L14 level snapshots and frames
  (`capture.txt`, `verify.txt`). The L10 cell holds 10 markers instead of 30
  on the home path. Counts above L10 are unchanged.
- The snapshot `T` token loses its flattening field.
- Region portals now sit at lower marker indices in the planet cell (after
  one record and the moons, instead of after about 21 records). Cell seeds
  hash the marker index, so every region, city, plot, and room below a
  planet gets new content. The demo journey still opens the same depth with
  the same timing rule (`JOURNEY` lines in `verify.txt`).
- A tall tower opens into a narrow, tall room; furniture scales down with
  the room so it always fits.
- The same seed still yields the same universe on every run (verify repeat
  identical).
