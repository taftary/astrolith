# 0017 — Body-and-portal markers with drawn forms

- Status: accepted
- Date: 2026-10-09
- Issue: #384 (Spec v1 approved; Amends ADR 0004 and ADR 0010, which stay
  accepted for nesting, ratios, frames, and the portal/population split)

## Context

ADR 0004 says every marker is the next level's cell; ADR 0010 splits
markers into portals (open) and population points (shown only). Both draw
every marker as one wireframe sphere at one radius per level, so a galaxy,
a star, a planet, a comet shell, a continent, a building, and a chair all
look alike, and the per-object sizes the generators compute never reach
the screen. The guides describe bodies with shapes (disks with bars,
rings, shells, grids, boxes) and portals as spots on or in those bodies
(the Sun's lane in the galaxy, a patch on the planet).

## Decision

Every marker has a **body** (what is drawn, at the object's own size and
form) and, for portals, a **portal** (the child cell at the true ladder
ratio, located on or in the body, defaulting to its centre). Opening,
closing, previews, siblings, crossings, hover, click, and the autopilot
read the portal position; drawing reads the body. Cells are spheres: every
generator keeps its points within half a cell of the centre, checked by
`in-sphere` on every `--verify` `LEVEL` line.

`Point` carries `form` (a plain-data shape: dot, body, disk, spheroid,
clump, ring, shell, orbit, arc, arm, thread, sheet, patch, grid, rect,
box), `portal` (the child offset, `None` at the centre), and
`environment` (L1-L3 only: void, wall, filament, node). Snapshots print
`F <name> [params]`, `Q dx dy dz` for off-centre portals, and
`E void|wall|filament|node`. Rendering maps forms to gizmo primitives
under ADR 0002 (no meshes); markers below `FORM_ANGLE = 0.006` rad draw
dots at their own size, above it their form, above `PREVIEW_ANGLE` their
preview, at `OPEN_ANGLE` they open.

`--verify` gains `in-sphere=` on every `LEVEL` line plus `HOME-PORTAL L4
off-centre=` (the journey opens the Milky Way in its disk lane) and
`ORBITS L8 coplanar= ordered= spaced=` (one ecliptic plane, log-ordered
non-overlapping orbits) lines, recorded here the way ADR 0013 recorded
`FREE-LEG`.

## Consequences

Every snapshot byte moves (new tokens, sphere rule, portal split,
re-probed count salts); the golden files are rewritten once with the
reason per line class. The fixed journey keeps its home fixtures (Milky
Way, Alpha Centauri, Sun, Earth) on possibly different marker indices.
`E-RENDER-NO-MESH` stands: every form is a gizmo primitive.
