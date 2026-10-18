# 0036 — Crack-free morphing and skirts (rebuild step 4)

- Status: accepted
- Date: 2026-10-18
- Issue: #777

## Context

Step 4 of the cube-sphere rebuild (#770 Phase 2, after #799) hides the two
artefacts step 3 left visible: one-frame shape jumps when a patch splits or
merges (popping) and gaps where a fine patch meets a coarser one (984
cross-level cracks counted headless). Owner answers Q1-Q7 (all A) pick the
CDLOD recipe from the design doc: fix the relief merge rule first, geomorph
in the vertex shader, pin edges as the primary crack fix, skirts as the
safety net, headless popping under 1 px plus zero cracks, blend positions
and normals now, 30% blend zone.

## Decision

- **Relief-aware merge (Q1 A).** Selection moves to core `lod_select.rs`
  (`E-FILE-SIZE`); `lod.rs` keeps keys, geometry and error functions. One
  error function `max(quad_sag, relief_error)` over the relief-grown
  nearest distance drives both splits (above `detail_px`) and merges (below
  half, only when the parent's relief data is known under the parents-stay
  rule). The single relief pass and the undo-in-the-same-frame path are
  gone; splits cascade over frames as children get built. Mountain detail
  builds up and stays; a still camera gives 0 splits and 0 merges
  (`mountain_detail_builds_up_and_stays`).
- **Per-vertex geomorph in core, mirrored in the shader (Q2 A, Q6 A,
  Q7 A; #777 review items 1, 3, 5, 6).** Core `lod_morph.rs` (std only) is
  the one formula: `vertex_factor(distance, err, detail, edge_bits, mask)`.
  Each vertex blends by **its own** distance to the camera over the last
  30% of a band ending at `err × focal / detail`, the distance where its
  parent splits, so a patch that has just split starts in its parent's
  shape and a patch about to merge is already blended to it. A per-patch
  factor was rejected: two neighbours blended a shared edge by different
  amounts (up to 7.9 px gaps measured in review).
- **Parent shape, normals and morph error (Q6 A; review items 2 and 5).**
  Core `lod_patch_morph.rs` (split from `lod_mesh`, `E-FILE-SIZE`): even
  points keep their place; odd edge points take the midpoint along the
  edge; odd-odd points take the midpoint of the diagonal the parent draws
  (`a, c, b / b, c, d` splits along `b-c`), so a fully blended patch is
  exactly its parent's triangles (`fully_blended_patch_is_its_parent`
  against the real parent). Normals are terrain normals from the relief
  (central differences along two tangents fixed by the direction, step =
  the level's grid angle), the parent normal uses the parent's step, so it
  equals the parent patch's own normal. Each vertex carries a morph error
  (its largest move to the parent shape): each edge's from that edge's own
  points only (same bits on both sides of the edge,
  `shared_edges_blend_alike`), the rest from the whole patch. Skirts copy
  their edge point's morph data.
- **Custom shader, re-added (Q2 A; review items 3 and 4).** Render
  `patch_material.rs`: `PlanetPatchMaterial = ExtendedMaterial<
  StandardMaterial, PatchMorph>` with embedded `patch_morph.wgsl`, the
  pattern of ADR 0027 (#626). The base stays white and unlit with
  vertex-colour height tints; parent positions, parent normals, edge flags
  and the morph error ride as custom attributes (locations 17-20). The
  shader takes the distance from the vertex's world position and the view
  position, so nothing per patch changes per frame. The uniform holds only
  the morph mask and the band scale (`metres per unit`, `focal / detail`);
  patches share one material per mask (`PatchMaterials`, at most 256, made
  on first use), switch handles only when their mask changes, and a steady
  frame returns early without allocating (`E-HOT-NOALLOC`). The agreement
  test runs the WGSL formula in `f32` against `lod_morph::vertex_factor`.
- **Edge mask (Q3 A).** `lod_morph::morph_mask_in(key, drawn)`, `O(depth)`
  per edge against the drawn set: low nibble = edges touching a coarser
  neighbour (in-between points pinned, factor 1), high nibble = edges
  touching a finer neighbour (held, factor 0). Both sides therefore draw
  the same line (`edges_pin_exactly_to_the_coarse_edge`). `covering_level`
  walks the subtree instead of scanning every leaf.
- **Skirts (Q4 A).** Safety net only, hidden under the ground; any residual
  gap shows ground, never sky.
- **Headless (Q5 A; review items 1 and 2).** Core `headless_morph.rs` runs
  the scripted 2,000 km to 1 m and back through the real selection, build
  and blend; `headless_morph_measure.rs` blends every drawn patch as the
  shader does and matches points by grid position, never by blended
  position. Cracks: every shared edge point across the patches drawing it,
  every in-between point of a finer edge against the coarse edge around it;
  a gap of 0.5 px or more counts (the only gaps left are `f32` rounding,
  0.13 px one metre from a 5 km patch). Popping and normals: each point against the
  other frame's surface at the same grid position, read from the patch
  covering it (same, coarser through its triangles, finer through its
  vertex), both ways, so points appearing on a split or vanishing on a
  merge are compared. The path flies at the default free-flight speed
  (step 5: the height shrinks by `e^(-1/60)` per frame, 871 frames each
  way). The line prints `MORPH popping_px normal_deg high_popping_px
  high_normal_deg cracks gap_px mountain_depth plains_depth still_splits
  still_merges`.
- **AC6 split at 3 km (owner decision, Telegram 2026-10-18, option 2).**
  Below 3 km: popping under 1 px and normals under 2°. At or above 3 km:
  popping under 2.5 px, normals reported only (no light before #780). Up
  high the relief error shrinks slowly from level to level, so three or
  more blend bands overlap and a patch can still be mid-blend when it
  splits or merges, or when a neighbour splits under a shared edge. Tried
  and rejected: a two-level blend toward the parent's blended shape, a fine
  edge following the coarse blended edge (0.57 px cracks), a 3D relief
  error (plains split as deep as mountains). The fix is true CDLOD
  selection by per-level distance ranges, #817. The `LOD`
  line's `cross_level` is the same crack count on its descent (moved to
  `headless_lod.rs`, `E-FILE-SIZE`).

## Consequences

Extends ADRs 0034 (LOD) and 0035 (relief); extends ADR 0027 (custom
shaders) with the first vertex shader. `E-STDOUT-PROTOCOL` gains the
`MORPH` line. `E-RENDER-NO-MESH` now reads one morph patch mesh (surface
plus skirts, with parent and edge attributes) per drawn patch plus one body
per moon. Frame time, measured on the dev PC (Intel UHD 620), dev build, real
window, F3 on, mountain direction, 60 steady frames after 240 settling
frames: `main` before #777 at 55.4, 48.5, 51.3 and 50.7 ms at 2,000 km,
10 km, 100 m and 1 m; this step at 55.7, 53.2, 48.1 and 52.3 ms (87 to 399
patches against 45 to 363); the same window with no patch at all at 48.7,
48.9, 45.7 and 49.2 ms. Patches cost 3-7 ms; the rest of the ~48 ms is the
app outside the planet patches, not this step. `--screens` is `SCREENS-OK`
with the F3 shot. Culling (#778), walking (#779) and light (#780) come
later; normals are ready for #780.
