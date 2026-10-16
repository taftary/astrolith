# 0024 — One icosphere mesh from planet to building

- Status: superseded by 0028
- Date: 2026-10-11
- Issue: #458 (correction round 1)

## Context

ADR 0020 put the planet-to-building leg on one icosphere, but the merged
work drew three unrelated shapes. The open L10 planet was a Bevy
`SphereKind::Ico` body with 4 subdivisions. L11-L13 used a new ground disc
on a curvature bowl, plus round highlight discs. The L11-L13 points were
uniform scatters on that bowl. The core icosphere only fed the L10 region
clusters.

The owner's test report: "no new meshes creation, and it should take the
isosphere and subdivide and scale at each dimension, the isosphere is our
main mesh, plus by default subdivide it once". The answers on #458 were
Q1-Q5 as proposed, then option A: keep the ladder sizes, and let each level
split the entered triangles as often as its size needs.

## Decision

The core icosphere (`universe-core/src/icosphere.rs`) is the only ground
geometry from L10 to L13.

- **Default split.** The planet is the icosphere split once (80
  triangles). Region clusters grow over those triangles.
- **Depth per level.** Each tail level has a surface depth whose triangle
  edge is about a quarter of its cell, derived from the R11 ladder sizes
  (L11 ≈ 5, L12 ≈ 11, L13 ≈ 19). A test ties the table to `child_ratio`.
  Triangle paths widen to `u64`.
- **Tiles and clusters.** A cell's tiles are the in-window descendants of
  the entered parent cluster at the cell's depth. Clusters grow over those
  tiles. Each enterable cluster's portal sits on its anchor tile, the tile
  nearest the cluster mean, so children always nest inside parents. Closed
  clusters are population dots. L13 portals keep their building box, and
  the L14 room is unchanged.
- **One frame map.** `SurfaceFrame` maps planet directions to and from the
  open cell's units through the opened chain, in `f64`. Generation, ground
  clamp, landing and rendering all read it.
- **One mesh.** `universe-render` draws one unlit mesh for the open
  L10-L13 cell, built from the icosphere. It is split near the camera up to
  the next level's depth, with enterable-cluster triangles in the brighter
  tone. The open-L10 Bevy body and the L11-L13 discs are removed. Moons and
  L9 system bodies are not ground and keep their Bevy spheres.

## Consequences

This record supersedes ADR 0020. Its intent stands, and this record fixes
the default depth, the depth table, the anchor tiles, the frame map and
the single-mesh rule. The mesh exception of ADR 0016 and ADR 0020 now
covers exactly one surface mesh at L10-L13, plus the moons.

`verify.txt`, `capture.txt` and the frame proof change for L10-L13. Each
change is explained in the pull request. L1-L9 and L14 stay
byte-identical. Planet relief reads coarser from orbit (80 triangles by
default) and resolves as the camera approaches.
