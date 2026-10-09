# Forms and markers: every marker gets a body, a form, a size, and a portal

Author: Claude (agent-authored draft, requested by the owner in session on 2026-10-09). The four decisions below were chosen by the owner from a shortlist after a review of the universe guides and the running application; the wording is the agent's.

## Resume

Markers are the base of everything that comes after the universe MVP (the game-style notion starts with "tap a marker to target it"), and today they carry no meaning. Every marker at every level is the same Bevy gizmo wireframe sphere, drawn at one radius per level (`marker_radius = child_ratio × 0.5`) clamped to a 0.004 rad minimum, so almost every marker on screen is the same 5 px circle. The per-object sizes the generators compute (`Point.radius`: galaxy size, star radius, cloud size) are discarded by the renderer. Galaxy orientation, bar, kind, planet orbit and moon orbit are generated and never drawn.

This notion replaces that with a per-object **form** drawn at the object's own **size**, under a four-stage level of detail (impostor dot, form, preview, open), and splits each marker into a **body** (what you see, at its real size) and a **portal** (the child cell at the ladder ratio, located on or in the body). Cells become spheres. Layouts follow each level's `entry.md` page in `guides/universe/`, whose "what you see, in order" list becomes the acceptance checklist.

## What the application shows today

Read from the code on `main` (commit `5ad6081`) and from a `--capture` run of the fixed journey.

| Level | What the guide describes | What is generated and drawn |
|---|---|---|
| L1 | The observable sphere | 8 portals on a cube lattice (±0.25): a cube grid |
| L2–L3 | Filaments (threads), walls (sheets), voids (empty bubbles), nodes | Uniform dots; `Environment` is computed to pick kinds but not stored on the point and not drawn, so a void and a node look the same |
| L4 | Spirals as tilted disks with bars and arms, ellipticals as smooth spheroids, dwarfs ragged | Uniform-random dots; `GalaxyInfo` only sets the tint |
| L5 | Arms, clouds sized by mass, open and globular clusters, the disk frame | A spiral layout exists (`attach_l5_clouds`) but is drawn as same-size dots, so the arm reads as a blob |
| L6 | Stars colored by class, close doubles split, usually one system | Uniform dots; companions nudged beside the primary |
| L7 | Sun glare, Kuiper doughnut, scattered disc, two Oort shells | Star centered, Oort shells sampled, drawn as random dots with no ring or shell visual |
| L8 | Sun, planets in order on the ecliptic, belts, heliopause bubble | Sun centered; the eight planets at random 3D positions although `orbit_au` rides on each point |
| L9 | "One glare takes everything": the star body, planets as nearby points | No star drawn at all; eight random planet dots unrelated to the L8 positions |
| L10 | Planet disk, moons orbiting, region portals on the surface | Planet mesh; 23 surface dots sitting on the mesh; moons in a ring; 6 region portals floating on the face axes, off the surface |
| L11–L13 | Coasts, ranges, grids, roads, blocks, roofs | 8 uniform random dots each, half portals, all radius 0.01 |
| L14 | Shell, furniture | 12 random boxes (the only non-sphere form) |

Two scale-nesting problems sit on top of this:

- Cells are cubes but drawn as spheres. Points scatter in `[-0.5, 0.5]^3`, so corner markers sit at distance 0.85, outside the radius-0.5 shell you entered. The guides say sphere throughout.
- A marker is both the object and the portal, at the child-cell size. At L4 a galaxy's sampled size is about 0.05 cell units while the L5 cell it opens into (a 100 pc complex) is 0.003, fifteen times smaller. The guide's own words ("target the Orion Spur portal, the Sun's lane 26,000 light-years from the galactic center") describe a body you see and a portal that is a spot on it. Today the whole galaxy is the dot.

Smaller: the RGB debug axes are always drawn at the origin; previews stack 32 clamped circles inside a 27 px circle and read as a blob.

## Owner decisions (chosen in session)

1. **Body and portal are split.** A marker has a drawn body at its own size and a portal (the child cell at the ladder ratio) located on or in the body. This amends the wording of ADR 0004 and ADR 0010: the portal is still the next cell; the body is what it sits in.
2. **Forms live in `universe-core`.** A `Form` descriptor on `Point`, plain data, Bevy-free, snapshot-visible. The renderer only maps a form to gizmo calls.
3. **One notion, sub-issues per level group.** A single Issue with tasks for the core and render foundation, then L1–L3, L4–L6, L7–L9, L10–L14, then proof.
4. **Cells are spheres.** Generators keep points within radius 0.5 of the cell center; the drawn shell and the generated content agree.

## What gets built

1. **Core `Form` and the body/portal split.** `Point.form: Form` with variants along the lines of `Dot`, `Disk { normal, barred }`, `Spheroid`, `Ring { normal, inner, outer }`, `Shell`, `Thread { to }`, `Sheet { normal }`, `Patch`, `Rect`, `Box { height }`; `Environment` carried on L1–L3 points; `Point.portal` as the child cell's offset inside the body (defaulting to the center). `nav` opens at the portal offset; `preview_positions` and `sibling_in_open_units` follow it. Generators keep points inside the unit sphere. The snapshot gains the form and portal tokens, so the golden files move once, explained in the pull request.
2. **Render `forms.rs` and a form-aware level of detail.** Form to gizmo mapping replaces the three `gizmos.sphere` sites. Below an impostor threshold the marker is a dot sized and glowing by brightness; above it, its form at `Point.radius`; above `PREVIEW_ANGLE` the preview inside it; at `OPEN_ANGLE` it opens. Four continuous stages instead of two. The debug axes go behind a toggle. Indicators only: gizmo lines, circles, ellipses, arcs, polylines, grids, rects, cuboids and spheres, so ADR 0002 and `E-RENDER-NO-MESH` stand.
3. **L1–L3 cosmic web.** The observable sphere with a faint graticule and eight octant sub-spheres; node glows; filament line links to the two densest neighbours; wall sheets; void circles (they dominate the volume); clusters as concentric glows; groups as tight knots of three to five dots.
4. **L4–L6 galaxies to neighbourhood.** Galaxy disks, spheroids and clumps from `GalaxyInfo` at `galaxy.size` with the portal at the spur; `far_view` galaxies as dots. L5 disk ellipse plus two arm polylines from the cell's arm frame, clouds as circles sized by mass, clusters as knots. L6 stars sized by mass with class tint and a companion arc beside the primary.
5. **L7–L9 systems.** L7 central glow, Kuiper and scattered rings, two Oort shell circles with the populations on them. L8 an ecliptic plane, planets on orbit circles at log-compressed radii from `orbit_au` sized by `radius_earth`, belts as annuli, the heliopause as the outer shell. L9 the star body drawn at the center with class tint and glow, planets as ordered nearby points in the same plane, the companion as a dot.
6. **L10–L14 planet to room.** Region portals as patches on the planet surface, moon orbit circles, equator and tilt axis, the redundant surface dots dropped. L11–L13 a ground plane with a grid: regions and cities as rect outlines, roads and rivers as polylines, lakes as circles, buildings as cuboids of varied height. L14 the room shell box around the furniture cuboids.
7. **Proof.** Frame proof per level; each level's `entry.md` ordered list is the acceptance checklist; guides cross-links updated where the drawn result is described.

## Relation to existing decisions

- ADR 0002 (indicators only) stands: every form is a gizmo primitive. ADR 0015 (billboards, bloom, HUD) and ADR 0016 (bare planet mesh) stand.
- ADR 0004 and ADR 0010 get a wording amendment for the body/portal split; the ladder table, anchors, ratios and the journey rule are unchanged (`E-LADDER-FROZEN` untouched).
- `E-GOLDEN`: one rewrite, explained per line class. `E-CORE-NO-BEVY`: `Form` is plain data. `E-TRANSCENDENTAL`: all geometry stays in core generators.
- The realism review (`guides/universe/documents/realism-review.md`, themes 5.2 to 5.5 and 5.7) already proposes most of these layouts; this notion is the visual-language half those themes assumed.

## Open questions for clarification

1. Log-compressed radii for L7 and L8 orbits: pure log, or linear inside and log outside so the inner planets do not collapse onto the star?
2. L2 filament links: computed in core as a `Thread { to }` form with a neighbour index (deterministic, snapshot-visible), or left to the renderer?
3. Impostor threshold: a fixed angle like today, or a pixel size (which needs the viewport height on the render side)? What value?
4. The body/portal split moves the journey positions and the golden files once. Confirm one rewrite is acceptable.
5. L9 star body radius: match the L10 planet body (0.35 cell units) or derive from `mass_solar`?
6. Debug axes: remove, or keep behind a key?
