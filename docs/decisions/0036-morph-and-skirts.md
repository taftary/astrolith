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
- **Geomorph in core, same math in the shader (Q2 A, Q6 A, Q7 A).** New core
  `lod_morph.rs` (std only): `morph_factor(distance, start, end)` over the
  last 30% of the band plus `morph_band_m(key, err, detail)` from the split
  and merge thresholds. `build_patch` adds per-point parent offsets and
  normals (even grid points keep their place, odd points store the midpoint
  of their even neighbours) and a skirt ring below every edge at
  `max(4 × patch error, 1 m)` with its own indices, morphing with its edge
  point (`points_blend_to_their_parent_shape`, `every_edge_has_a_skirt`).
- **Custom shader, re-added (Q2 A).** Render `patch_material.rs`:
  `PlanetPatchMaterial = ExtendedMaterial<StandardMaterial, PatchMorph>`
  with embedded `patch_morph.wgsl`, the pattern of ADR 0027 (#626). The
  base stays white and unlit with vertex-colour height tints; per-vertex
  parent positions/normals plus edge flags ride as custom attributes
  (locations 17/18/19); per-patch camera offset, band and edge mask ride as
  uniforms updated per frame. The vertex shader computes the same factor
  core does and pins masked odd edge points to factor 1. No mesh is rebuilt
  per frame. This re-adds a custom shader (removed in #770) and extends
  ADR 0027; the agreement test pins the WGSL 0.3 zone against
  `lod_morph::morph_factor`.
- **Edge mask (Q3 A).** `QuadTree::edge_mask` (4 bits, East/West/North/South)
  after selection via `covering_level` across faces. Masked odd edge points
  use factor 1, so both sides draw the same line
  (`edges_pin_to_coarser_neighbours`: evens match bit for bit, odds are
  midpoints). A no-window app test checks every patch uses the morph
  material.
- **Skirts (Q4 A).** Safety net only, hidden under the ground; any residual
  gap shows ground, never sky.
- **Headless (Q5 A).** New core `headless_morph.rs` (`E-FILE-SIZE`):
  scripted 2,000 km to 1 m and back at 1/60 s through the real selection,
  build and morph functions, projecting every drawn point at 1600x900/70°.
  Prints `MORPH popping_px=<worst> normal_deg=<worst> cracks=<n>
  mountain_depth=<m> plains_depth=<n> still_splits=<s> still_merges=<m>
  PASS`; two runs byte-identical (`no_cracks_on_the_scripted_path`,
  `popping_stays_under_one_pixel`: under 1 px and 2°). The `LOD` line now
  uses relief-aware selection and reports `cross_level=0` (geometric gaps
  after pinning; the restricted quadtree is still enforced topologically).
  Goldens rewritten and explained (`E-GOLDEN`). `--screens` keeps the F3
  `lod-overlay.png` shot, now with the morph material.

## Consequences

Extends ADRs 0034 (LOD) and 0035 (relief); extends ADR 0027 (custom
shaders) with the first vertex shader. `E-STDOUT-PROTOCOL` gains the
`MORPH` line. `E-RENDER-NO-MESH` now reads one morph patch mesh (surface
plus skirts, with parent and edge attributes) per drawn patch plus one body
per moon. Measured on the dev PC (Intel UHD 620) at 1600x900, real window
with F3: frame times at 2,000 km, 10 km, 100 m and 1 m posted on #777 before
the PR opened (T5), all at about 16.7 ms or under; `--screens` is
`SCREENS-OK` with the F3 shot. Culling (#778), walking (#779) and light
(#780) come later; normals are ready for #780.
