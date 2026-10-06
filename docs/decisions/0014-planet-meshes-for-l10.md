# 0014 — Planet meshes for the L10 terminal cell

- Status: accepted
- Date: 2026-10-05
- Issue: #156 (Spec v1 approved; amends ADR 0002, which stays accepted for every other level)

## Context

ADR 0002 says `universe-render` draws indicators only and creates no
meshes, materials, or textures. The realism review (#143, theme 5.5) asks
for L10 planets as oblate spheroids with terrain, biomes, moons, and air,
which gizmo spheres cannot show: a planet needs a shaded body, shore lines
between ocean and land, and a glowing rim of air.

## Decision

`universe-render` may create meshes, standard materials, and vertex colors
for exactly one case: the open L10 cell (one planet body with biome vertex
colors, one atmosphere rim shell, one small body per moon). Everything else
stays gizmo indicators under ADR 0002. Bodies use unlit standard materials
(no lights are added); the rim is a transparent shell, denser and bluer for
Earth air. Meshes are rebuilt from the deterministic cell seed when the open
cell changes and despawned when the dive leaves L10.

## Consequences

The `bevy_mesh` feature is enabled with its justification in
`docs/universes/stack.md`; no new crate and no license change. Per-object
star and galaxy colors, bloom, exposure, and the scale HUD stay in #157.
Frame proof covers the visual change.
