# 0037 — Depth and culling (rebuild step 5)

- Status: accepted
- Date: 2026-10-18
- Issue: #778

## Context

Step 5 of the cube-sphere rebuild (#770 Phase 2, after #777) asks for
dynamic near and far planes on the existing infinite reverse-Z depth, and
for frustum, horizon, back-face and distance culling, with depth fighting
checked at 1 cm, 1 m and 1,000 km and counted budgets (#770 Q5 A, Q8 A).

What was there:
- At L10 the near plane was 5% of the distance to the dive target or the
  cell centre, capped at 0.1 cell (about 1,290 km). It was never the height
  above the ground, so near the ground it sat about 300 km away (#820,
  "can't reach the ground").
- Nothing was culled: the tree refined all around the planet.
- Patches were two-sided. Three of the six cube faces (PosX, NegY, PosZ)
  and half of every face's skirts were wound inward.
- The window used Bevy's default 45° vertical field of view, while every
  pixel measure and the LOD detail rule assume 70° (ADR 0032, 0034).

Owner answers Q1-Q9 were all A, and F1 was A.

## Decision

- **Near plane from the height above the terrain (Q1 A).**
  - Core `planet_frame::near_plane_m(h) = max(0.05 * |h|, 1 mm)`.
  - `Relief::height_above_m` gives the height above the drawn terrain
    under the camera.
  - At L10, `sync_camera` takes the smaller of the old gap rule and this
    rule, in every mode, using the seed-keyed relief in `PatchTree`.
    Moons close to the camera are still never clipped.
- **No far plane (Q2 A, Q6 A).** Bevy's infinite reverse-Z projection
  stays, and `projection_is_infinite_reverse_z` pins it. The horizon is the
  distance limit.
- **Visibility in core (Q3 A, Q4 A).** `lod_cull` holds a `View` (camera,
  forward, up, vertical FOV, aspect) and a conservative patch sphere.
  - The sphere covers the cap within the patch's half angle, between
    `(radius + min_h) * cos(theta)` and `radius + max_h`. That truncated
    cone holds the triangles, the chord sag and the #777 parent shape.
  - Heights are the built patch's range, or `Relief::height_bounds_m` while
    it is unbuilt.
  - Frustum: four side planes plus the camera plane.
  - Horizon: hidden only when fully beyond the horizon plane and inside the
    shadow cone of the lowest-relief sphere. Mountain tops beyond the
    horizon stay.
- **Selection culls (Q3 A).** `QuadTree::update_culled_reused`:
  - a hidden leaf is never split;
  - a hidden parent merges whatever its error;
  - the tree stays a full cover (neighbour rule unchanged);
  - `append_visible_leaves_to` gives the wanted and drawn leaves, and only
    those are built.

  A fast turn is covered at once: the selection splits in the same frame,
  and cached parents draw until builds land (`fast_turn_has_no_holes`).
  Masks stay against the draw set. A shared edge with any visible point
  makes both patches visible.
- **Engine check kept.** Each patch entity carries an explicit `Aabb` over
  its own and parent positions, with `NoAutoAabb`, so Bevy's frustum check
  never drops a morphing patch.
- **One-sided triangles (Q5 A).**
  - `lod::face_grid_right_handed` picks the winding per face. Right-handed
    grids reverse the order and keep the same diagonal, so the #777 morph
    is unchanged.
  - Skirts flip on the North and West edges of right-handed faces and on
    the South and East edges of the others.
  - `triangles_face_out` and `skirts_face_out` check all six faces, and the
    material uses `cull_mode: Some(Face::Back)`.
- **Selection follows the rendered camera.** `sync_patches` selects and
  culls from `RenderOrigin.camera` (last frame's pose), not the path offset.
  In third person the camera sits behind the body, and the ground under the
  body must stay in view and refined.
- **Player up stopgap (F2 A).** `body::level_up` points away from the planet
  centre at L10 while `level_ground` has no ground (until #779), not `+Y`.
  At the seeded spot `+Y` was about 28° off, and the third-person camera
  sank under the drawn ground, which one-sided patches no longer show.
- **The camera uses 70° (F1 A).** `camera_projection()` sets
  `precision::FOV_DEG`, so on-screen pixels match the measures.
- **Headless lines (Q7 A, Q8 A, `E-STDOUT-PROTOCOL`, `E-GOLDEN`).**
  - `CULL` (`headless_cull`) flies the #777 descent and climb looking 20°
    below the horizon. Builds are capped at 100 a frame, as
    `PATCH_APPLY_BUDGET`. It prints the worst visible patches (≤ 2,000),
    triangles (≤ 3 million, 2,304 a patch), builds and culled leaves, plus
    two zero counts: drawn but hidden, and hidden with a sampled grid point
    in view above the horizon.
  - `DEPTH` checks `f32` `near / distance` with the real near rule. At 1 cm
    and 1 m it uses 1 mm gaps. At 1,000 km it uses a one-pixel gap on the
    ground (about 1.56 km at 643 px focal), because 1 mm there is far below
    a pixel.

## Consequences

- Close to the ground the planet is no longer cut away. On the culled
  descent the worst frame draws 133 patches, against 336 without culling,
  and builds only visible leaves.
- The wider 70° view changes every level's look: more is visible and things
  look smaller.
- The `planet-l10` screens shot now waits 480 frames
  (`PLANET_SETTLE_FRAMES`), because the morph pipeline can compile slower
  than 240. The new `planet-1m.png` shot sits 1 m above the ground.
- Frame time on the dev PC is level with `main` within run-to-run noise.
  The selection costs about 3 ms a frame at 750 leaves. The frame-pacing
  floor stays in #819.
- Not done here: instancing (Q9 A), pop-free switches up high (#817), and
  the dive camera turn (#820).
