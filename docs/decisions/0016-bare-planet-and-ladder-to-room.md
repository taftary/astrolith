# 0016 — Bare planet and the ladder down to the room

- Status: accepted
- Date: 2026-10-09
- Issue: #375 (Spec v1 approved; supersedes ADR 0014; narrows the
  planet-colors clause of ADR 0015, which stays accepted for everything
  else; the ladder wording change is covered by the spec approval per
  `E-LADDER-FROZEN`)

## Context

ADR 0002 says `universe-render` draws indicators only and creates no
meshes, materials, or textures; ADR 0014 relaxes it for exactly the open
L10 cell (one planet body with biome vertex colors, one atmosphere rim
shell, one small body per moon); ADR 0015 keeps the biome colors. The
owner notion (#375) removes biomes from current development and finishes
the scale ladder down to the room: biome classification sits below the
resolution the dive shows, sea level and atmosphere go with it, and L10
must stop being terminal so the dive can continue into L11.

## Decision

`universe-core` drops the whole climate layer: no `Biome` tag, no
`biome_for`, no sea level, no `AirKind`. `SurfaceInfo` keeps height,
flattening, tilt, and spin; the snapshot surface token becomes
`T height flattening tilt spin`. The moon `N` token is unchanged.

`MAX_OPEN_LEVEL` and `MAX_NAV_LEVEL` become 14. L10 cells carry region
portals that open into L11; L11–L13 are sparse deterministic scatters
under a new tail generator; L14 is one room of furniture outlines and
never opens. Labels, anchors, and tints cover L11–L14; the words
"Beyond MVP" disappear; the `--verify` journey runs L1–L14.

`universe-render` keeps meshes for exactly one case, narrowed: the open
planet cell builds one flat-tone unlit body (`PLANET_TONE`) plus one
small body per moon. No vertex colors, no rim shell. Everything else
stays gizmo indicators under ADR 0002.

Golden files move twice (removal, then the ladder), each rewrite
explained in the pull request; the frame-proof expected file count moves
from 11 to 15.

## Consequences

Supersedes ADR 0014. ADR 0015 stays accepted except
its "planets keep their biome colors" clause. Every L10 snapshot byte
changes; the `--verify` header reads `nested dive L1-L14`. Frame proof
covers the bare planet and the four new level tints; nothing else in the
L1–L9 look moves.
