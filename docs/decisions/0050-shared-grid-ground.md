# 0050 - Ground patches from one shared grid

- Status: accepted
- Date: 2026-10-26
- Issue: #920 (Spec v3)

## Context

After #918 (ADR 0049) a drawn ground patch still cost about 72 KB on the GPU: 1,221 vertices with positions, normals, parent positions, packed parent normals and morph errors, plus 6,912 indices. The 64 MB Ground memory limit held about 900 patches, and a phone needs far less (#920 notion: "phones can run it").

Owner answers: Q1 C (more room in the same memory, and phone sizes S1), Q2 A (a study first, then go / no-go), Q3 C (normals in the shader or stored, whichever the numbers favour), Q4 A (collision keeps its full CPU copy near the player), Q5 A (phone-safe features only), Q6 A (no visible change). After the study the owner wrote `go`; when the far view came out about 5 ms slower, the owner picked option C, keep optimising; at about 3 ms slower the owner picked A, accept, and Spec v3 reworded AC8 to "no worse on average over the five ground views, no view more than about 3 ms slower at the default limit".

## Decision

- **One shared grid.** Every patch draws one of two shared meshes (the 33 x 33 grid plus skirts, one per triangle winding). Its shape comes from its own tile in a shared heights texture, found through the entity's `MeshTag`. The vertex shaders (`patch_grid*.wgsl`) rebuild offsets, normals and the parent shape from the tile as `universe_core::grid_rebuild` does: offsets from the patch centre built only from small differences, with the tangent steps worked out in `f64` on the CPU, so no trigonometry runs on the GPU (Vulkan bounds GPU `sin` only to about 2^-11).
- **The tile** (`universe_core::grid_tile`): 37 x 63 texels of 32 bits (`R32Uint`), **9,324 bytes a patch**:
  - heights over the centre height on the patch and a ring of two points around it (`f32` bits);
  - morph errors as 16-bit halves, each a share of the patch's quad length, because coarse levels' errors would overflow a half in metres;
  - the per-patch values (face, frame, skirt depth, parent flag, centre tangents, quad length) and the morph mask (one texel, rewritten when the neighbours change);
  - the tangent steps per ring column and row;
  - the parent level's normals at the 17 x 17 even points, octahedral, 16 bits a component.
- **Normals.** Own normals are rebuilt in the shader from the four neighbours, which the parent pair shares on odd rows and columns. Parent normals are stored (1.2 KB): rebuilding them cost up to twelve extra grid points per morphing vertex, the largest part of the far-view gap (2.3 ms of 4.8 ms on the dev PC).
- **Draw order.** Bevy draws one shared mesh's instances in one batch, in an order fixed by when each patch arrived, so in some runs far ground drew first and the far view shaded 50% more pixels. Material batches draw in the order Bevy prepares their materials, so there is one material per level, made one a frame, finest level first: near ground always draws first. A scale change keeps each material's place, since Bevy frees and re-takes the same slot.
- **Pages.** The heights texture is an array of 1,024 x 1,024 pages (432 tiles each, within GLES3's guarantees of 2,048 texels and 256 layers), as many as the Ground memory limit needs at one tile per patch. A new limit remakes it and rebuilds the ground.
- **Memory.** The cache counts a tile per patch against the Ground memory limit, so 64 MB holds about 5,300 patches (a tile plus the slim `PatchInfo` each) instead of about 870 (Q1 C), and the setting gains 8 and 16 MB (S1): 8 -> 16 -> 32 -> 64 -> 128, default 64.
- **Builds.** A build packs its tile from the heights it already sampled (bit-identical to packing afterwards). Builds more than 100 m from the player are tile-only: no own or skirt normals, parent normals at the even points only. Builds near the player stay full: the body stands on their CPU mesh (Q4 A).
- **The body's ground.** The CPU meshes the body stands on stay near the camera and near the ground below the feet. Before, only the camera counted: a figure shot drops the body about 1.4 km while the camera is still high, and with the shared grid's busier draw set the body's ground rebuilt empty and the body fell through the planet.
- **`E-RENDER-NO-MESH`.** `universe-render` makes the shared heights texture, the two shared grid meshes and the per-level ground materials: the ground's own exception, as the per-patch meshes and morph materials were before (ADR 0016, ADR 0027, ADR 0036).

## Tests

- Core: `a_tile_rebuilds_its_patch_under_a_millimetre`, `stored_parent_normals_match_todays_within_a_twentieth_of_a_degree`, `a_tile_built_with_its_patch_matches_one_packed_afterwards`, `a_tile_only_build_packs_the_same_tile`, `halves_round_to_nearest_like_the_gpu`, `a_tile_is_at_most_ten_kilobytes`; the slow-tier precision probe (`headless_grid_precision`: 0.044 mm on plains, 0.013 mm in mountains, normals within 0.034 degrees).
- Render: `ground_shader_agrees_with_core`, `the_shader_uniform_is_the_ground_scale`, `the_shader_reads_the_core_tile_layout`, `pages_follow_the_ground_memory_limit`, `grid_materials_are_made_one_a_frame_finest_level_first`, `a_mask_change_writes_one_texel_after_its_tile`, `patches_share_one_material_per_level`, `a_drawn_patch_costs_its_tile`, `phone_memory_limits_leave_no_holes`, `the_ground_under_the_feet_keeps_its_cpu_mesh_with_the_camera_far_above`, `the_default_limit_holds_far_more_patches`, `ground_memory_limit_setting`.
- The headless game check prints exactly what it printed before (LOD, MORPH, CULL, RING, GROUND).
- Window: the smoke and planet shots match the per-patch meshes within two colour levels; timing runs take `ASTROLITH_GPU_TIMERS=1`, `ASTROLITH_TIMING_NO_VSYNC=1` and `ASTROLITH_GROUND_MEMORY=<MB>` (developer aids).

## Consequences

- About 7.7 times less graphics memory per patch (72 KB to 9.3 KB); phones get 8 and 16 MB limits.
- Per drawn patch the GPU cost matches the per-patch meshes on the dev PC (UHD 620): about 27 against 28 microseconds of main pass per patch in the far view. With the default 64 MB the cache holds far more patches, so views that were held back by the limit draw more of them. Mean frame on the five ground views of `ground.shots` (release, vsync on, three runs each, 2026-10-26), against the per-patch meshes with the same body fix:

  | View | Per-patch meshes | Shared grid, 64 MB | Shared grid, 8 MB |
  |---|---|---|---|
  | warm | 42.4 ms | 38.6 ms | 39.1 ms |
  | plain | 41.2 ms | 37.3 ms | 38.4 ms |
  | far | 40.8 ms | 43.9 ms | 42.4 ms |
  | down | 33.5 ms | 32.0 ms | 34.7 ms |
  | figure | 42.9 ms | 42.1 ms | 42.4 ms |
  | average | 40.2 ms | 38.8 ms | 39.4 ms |

  The far view draws 579 patches against 413 (no pixel changes), about 3 ms slower; the owner accepted that (Spec v3 AC8). The near ground is ready (`ring_ms`) within 1.1 s as before: down 0.60 s against 0.44 s, every other view within 5% of before or faster.
- Rebuilding positions and normals in the vertex shader reads the tile with `textureLoad` in the vertex stage, which phones support (GLES3 `texelFetch`, Vulkan); phone timing is not measured yet.
- The shared grid's per-patch values cannot use mesh instancing attributes in Bevy 0.19; the `MeshTag` lookup is the one per-instance input.
