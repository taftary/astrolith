# Planet to room: atmosphere, curved ground, and sub-places in their true places

Author: Claude (agent-authored draft, requested by the owner in session on 2026-10-09). The five decisions below were chosen by the owner from a shortlist after a review of the L10-L14 code and guides on `main` (commit `ad72410`); the wording is the agent's.

## Owner's words

> Like we did last notion I need to do more enhancement on universe scaling and dimensions. I want to see better the movement from planet to room. I should see atmosphere plus the curved view plus the relation and positions of sub place. More enhancement in this area please.

## Resume

The dive from the planet (L10) to the room (L14) exists since #375 and got its forms in #384, but the leg still reads as five unrelated scenes: a bare planet with six portals at fixed face centres, then three scatters of eight outlines on a ground that is curved once and flat twice, then a box of furniture. Nothing says you are falling through air onto a round world, nothing ties a city to the region it sits in, and the planet vanishes the moment you leave it. The tail levels also run on guessed sizes: L1 to L10 are anchored to published values, L11 to L14 are range midpoints.

This notion makes the planet-to-room movement continuous and truthful: an **atmosphere** you see from outside and fall through, **ground curvature** derived from the real planet radius at every surface level with a horizon, **sub-places placed by structure** (cities on rivers and coasts, buildings on blocks, the room inside the building) with the parent's context staying visible around you, and **published anchors for L11 to L14** so the ratios planet : region : city : building : room are real. Everything stays indicators under ADR 0002; the planet body keeps its single mesh under ADR 0016.

## What the application shows today

Read from the code on `main` (ADR 0016, ADR 0017, `nest.rs`, `terrain.rs`, `tail.rs`, `planet.rs`, `forms.rs`) and the L10-L14 `entry.md` guides.

| Topic | What the guides describe | What is generated and drawn |
|---|---|---|
| Atmosphere | L10: "the thin rim of air"; L11: "the curved face, lit on one side", clouds, local light; L12: a sky | None. ADR 0016 (#375) dropped the rim shell and `AirKind`; `SurfaceInfo` is height, flattening, tilt, spin. No sky, no haze, no horizon at any level |
| Curvature | L11: "the planet disk thins out into ground ... the curved face"; L12 and L13: ground under a sky | L11 only: points sag by `d^2 / (2 * 4.0)` and a `Form::Grid { curvature: 4.0 }` is appended (constant chosen by eye, not from the planet radius). L12 and L13 are flat planes (`y = jitter`). Decided in #384 Q8 |
| L10 region portals | "one marker brightens among the points: the region portal ... holding one L11 patch" | 6 patches at the cube-face centres (`REGION_PORTALS = 6`), independent of the relief |
| L11-L13 sub-places | Coasts, ranges, rivers, towns along the valleys; cities with grids, roads, ports; blocks, streets, roofs | `TailGenerator`: a uniform scatter of 8 per cell, first half portals (rect or box outlines), second half populations alternating threads (to the next two populations) and rings. Portals are not placed on the threads or rings drawn beside them |
| Portal location | ADR 0017: the child cell sits on or in the body, off-centre allowed | Every L10-L13 portal keeps the default centre offset; only the L4 home galaxy uses `Point.portal` today |
| Parent context | L11: "the full round Earth ... dissolves into the bright ground around you" | The L10 planet mesh and moons are despawned when the L10 cell closes; nothing of the planet, region, city, or building remains visible once you are inside the child |
| Dimensions | Ladder R5: one published anchor per rung | `LADDER_EXPONENTS` ends `5.5, 4.0, 1.5, 0.5`: range midpoints, no published object (`nest.rs:30`). Child/parent ratios: L10→L11 0.0245, L11→L12 0.0316, L12→L13 0.0032, L13→L14 0.1 |
| Milestones | Ladder gap note and the L11 guide: no milestones on the L10-L11 leg | `anon_cells` inserts 2 invisible milestones for L10→L11 and 2 for L12→L13 |

## Owner decisions (chosen in session)

1. **Atmosphere as indicators only.** Atmosphere data (height, tint) lives on `SurfaceInfo` in `universe-core`; the renderer draws it as tinted limb circles or a thin shell around the planet at L10 and as a sky arc with horizon haze at L11. ADR 0002 and ADR 0016 stand unchanged.
2. **Physical curvature at L11, L12, and L13.** One formula from the real planet radius expressed in each level's cell units: strong at L11, slight at L12, near flat at L13, plus a horizon line where ground meets sky. This amends #384 decision 8 (flat L12 and L13).
3. **Sub-places related to their parent, all four ways.** (a) Structure-driven placement: city portals sit on the region's rivers and coasts, building portals on city blocks along streets, the room portal inside the building box at a floor; all computed in core and snapshot-visible. (b) Portal-in-body continuity: each child cell is centred on the parent's portal offset (ADR 0017 `Point.portal`), so the spot you see in the preview is exactly where you land and leave from. (c) Parent context stays visible: after opening, the parent's ground or outline persists faintly around you (region outline around the city, city blocks around the building, building walls around the room) until you dive deeper. (d) L10 region portals sit on the relief (land patches) instead of the six cube-face centres.
4. **Anchor the tail.** L11-L14 get published anchors like L1-L10, so the tail ratios and milestones follow real sizes. This is a ladder wording change and goes through spec approval under `E-LADDER-FROZEN`, as ADR 0016 did.
5. **One notion, sub-issues per theme.** A single Issue with tasks for the core foundation, atmosphere, curvature and horizon, structure-driven placement, parent context, and proof.

## What gets built

1. **Core foundation.** `LADDER_EXPONENTS` for L11-L14 move to published anchors (ladder R11 amendment, anchor table extended, gap note and `anon_cells` reconciled). `SurfaceInfo` gains an atmosphere (scale height as a fraction of the planet radius, tint); the snapshot `T` token extends accordingly. A shared `planet_curvature(level)` in core returns the planet radius in that level's cell units, replacing `L11_CURVATURE`.
2. **L10 atmosphere and portals on the relief.** The open planet draws its air as concentric tinted limb circles (or a thin indicator shell) just outside the body, fading outward; the day side is not modelled (no lighting, ADR 0016). Region portals are chosen on the relief field (land patches at distinct heights) rather than at the six face centres, and each carries a `Point.portal` offset on the surface so the L11 cell opens there.
3. **Curved ground and horizon at L11-L13.** `TailGenerator` settles every point on `y = -d^2 / (2 R_cell)` with `R_cell` from the core function, appends the ground grid with that curvature at all three levels, and adds a horizon ring at the distance where the curved ground leaves the view. L11 also draws a sky arc with haze bands from the atmosphere data, so entry from L10 reads as falling through air onto a round world.
4. **Structure-driven placement.** Populations are generated first (rivers, coasts, lakes at L11; roads, rivers, lakes at L12; streets and blocks at L13), then portals are placed on them: city portals at river and coast points, building portals on block edges along streets, the single room portal inside the building box at a floor height. Each portal carries its `Point.portal` offset so preview, open, close, and sibling placement all follow it (ADR 0017).
5. **Parent context persists.** When a child opens, the parent's ground and the outline you entered through stay drawn faintly in the child's frame (the region's rect around the city, the city's blocks around the building, the building's walls around the room, the planet's limb and air around the region) at the true ratio, until the next open or until they fall below the dot threshold. Pure indicators, no new meshes; the planet mesh still despawns with its cell.
6. **Proof.** Frame proof per level L10-L14 before and after; each level's `entry.md` "what you see, in order" list is the acceptance checklist; `--verify` gains lines for the tail anchors, the curvature per level, and the portal-on-structure checks; guides cross-links updated where the drawn result is described.

## Relation to existing decisions

- ADR 0002 (indicators only) stands: air, horizon, sky, and context are gizmo circles, arcs, lines, and grids. ADR 0016 stands: the open planet keeps its one flat-tone body and moons; no rim shell mesh, no climate layer (the atmosphere here is a size and a tint, not weather or biomes). ADR 0015 stands.
- ADR 0017 (body and portal) is applied, not changed: tail portals get real `Point.portal` offsets.
- #384 decision 8 (flat L12 and L13) is superseded by decision 2 above.
- `E-LADDER-FROZEN`: the level table is unchanged; the anchor table gains L11-L14 rows and the gap note is reconciled with `anon_cells`, both through spec approval.
- `E-GOLDEN`: one rewrite, explained per line class (anchors move every L11-L14 snapshot and the journey positions; the `T` token grows; L10 portals move).
- `E-CORE-NO-BEVY`, `E-TRANSCENDENTAL`: curvature, horizon distance, atmosphere size, and placement are computed in core; the renderer only maps them to gizmo calls.
- The realism review (`guides/universe/documents/realism-review.md`, theme 5.5, FR5/FE5) lists the atmosphere rim and horizon as follow-ups; this notion is that follow-up in indicator form.

## Open questions for clarification

1. Which published anchors for L11-L14? Candidates: a 500 km range or basin (e 5.7), a 10 km city (e 4.0), a 30 m building (e 1.5), a 5 m room (e 0.7). Other choices change the tail ratios.
2. True curvature or declared exaggeration? At the real Earth radius the L11 sag across a cell is about 0.006 cell units (the current constant is five times stronger) and L12-L13 are visually flat; the L10 relief already declares about 8x exaggeration. Keep true values (horizon does the work) or share one declared exaggeration factor with the L10 relief?
3. Milestones: the code inserts two for L10→L11 and L12→L13, the gap note and the L11 guide list none. Once the tail is anchored, which rule wins (the 1.5-decade rule applied to the new gaps, or the guide)?
4. Horizon and sky at L12-L13: a horizon line only, or also the sky arc and haze bands (L11 gets both per decision 1)?
5. Parent context: fade with distance from the portal, or stay at constant brightness until the next open?
6. Atmosphere data: an Earth-only fixture (home planet) or a per-planet value from the generator (size-based, airless below a threshold, thick for giants)?
7. One golden rewrite covering anchors, portal offsets, the `T` token, and the journey positions: confirm acceptable.
