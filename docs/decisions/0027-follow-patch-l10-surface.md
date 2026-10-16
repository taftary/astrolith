# 0027 — Follow patch on the L10 surface

- Status: accepted
- Date: 2026-10-16
- Issue: #723 (step 1)

## Context

The open L10 surface drew the whole icosphere refined around the camera's
view cone (#598). The owner's notion (#723) calls that wrong twice over:
it looks wrong, and a full ball cannot follow the player or scale down
through the dimension levels. The locked answers: a small dome patch
around the player, ground detail fixed to the planet, the full ball kept
far away with the patch fading in on approach, the patch under the body
when walking and under the camera's ground point when flying, built in
steps starting with the L10 surface.

## Decision

The L10 surface draws as ball plus patch, tiling the sphere exactly once:

- **Centre.** `SurfaceFrame::patch_centre` (core, pure) maps the focus
  from cell units back to planet axes, so the patch recentres while every
  sample stays planet-fixed. `patch_focus` (render) picks the placed
  walking body when normal mode runs, else the camera.
- **Size.** `PATCH_RADIUS` is 0.2 radians: the near field with margin for
  the re-centre triggers. An internal quality choice, asserted in a const
  block, not a setting.
- **Partition.** `patch_faces` (depth-1 faces inside the radius) plus
  `patch_partition`: one coarse triangle per remaining base face, refined
  triangles inside the patch by the unchanged view/target rule. A test
  proves the two sets tile the 80 faces exactly once: no hole, no double
  draw, no z-fighting.
- **Gate.** `patch_active` is L10 with fade above zero only. The far ball
  and L11-L13 draw exactly as before; `--verify` and the goldens are
  untouched.
- **Rebuild.** `SurfaceMeshState` also tracks the focus; the mesh rebuilds
  when it moves past `REBUILD_SHARE` of the height, reusing the existing
  move/turn/target triggers.
- **One mesh.** The single-`SurfaceMesh` rule stands: ball and patch merge
  into one triangle list, one material, one fade. This record refines ADR
  0024; 0024 stays accepted.

## Consequences

Close-range L10 screenshots change: coarse ball outside the patch,
refined ground inside it, recentring as the player moves. The scale-down
through L11-L14 is a follow-up issue and must revisit `patch_active` and
`PATCH_RADIUS`.
