# 0034 — Quadtree LOD patches (rebuild step 3)

- Status: accepted
- Date: 2026-10-18
- Issue: #776

## Context

Step 3 of the cube-sphere rebuild (#770 Phase 2) turns the six fixed faces
of step 2 (ADR 0033) into a quadtree of small patches that split where the
camera looks closely and merge where it does not. Owner answers Q1-Q8
(all A) pick 33 x 33-point patches, 1.5 px screen error at 1600x900 with a
half-threshold hysteresis band, worker builds with parents kept, per-patch
origins for the #774 shake target, an F3 debug overlay, depth to about 1 m,
and relief deferred to #799. S1 yes adds Planet detail (Low 2 px, Normal
1.5 px, High 1 px); S2 yes adds the Developer overlay row.

## Decision

- **Quadtree (Q1 A, Q7 A).** New core module `lod.rs` (std only,
  `E-CORE-NO-BEVY`): `PatchKey { face, level, x, y }` (`E-NEWTYPE`),
  `QuadTree` with six roots from `Face::all()`, `max_level_for_radius`
  (`floor(log2(face_edge / 32))`; Earth 18), deterministic
  (`E-DET-TIERS`). New `lod_mesh.rs` holds `PatchMesh` and `build_patch`
  (split under `E-FILE-SIZE`).
- **Screen error (Q2 A, Q3 A).** Sag `R·(1−cos(theta/2))` over distance times
  `focal_px` (1600x900, 70° vertical); split above `detail_px`, merge only
  below half; restricted quadtree (neighbours within one level, cross-face via
  `from_sphere` hair-step); iterative splits, then merges, then neighbour
  enforcement until stable.
- **Patch build (Q5 A).** `build_patch(key, radius_m)` on #775's mapping:
  33 x 33 points via global `grid_point` (same-level edges match bit for bit),
  `f64` centre plus `f32` metre offsets and normals, indices; pure, runs on
  any thread. Shake measured through the patch-origin path stays under 1 mm
  within 1 km at 1 m, 100 m, 10 km and 1,000 km.
- **Workers (Q4 A).** Render `patches.rs`: `AsyncComputeTaskPool` builds,
  bounded LRU `PatchCache` (`PATCH_CACHE_CAP` 2048), per-frame apply budget
  `PATCH_APPLY_BUDGET` 100 (design upper limit 100-200, measured below),
  parents stay until all four children are cached (no holes), `Anchored`
  patch entities with cell-unit offsets; #775's six fixed faces removed, moons
  stay in `planet.rs`.
- **Debug overlay (Q6 A, S2).** New `debug_overlay.rs`: `F3` (`Action::DebugOverlay`,
  default `F3`, rebindable) toggles `store.debug_overlay` mirrored to
  `Navigation.show_overlay`; overlay shows wireframe (gizmo level-coloured
  edges), LOD colour by depth, patch count, split/merge queues, frame time,
  altitude, render origin; off by default; Developer row mirrors `F3`.
- **Planet detail (S1).** Core `PlanetDetail` (`Low`, `Normal`, `High`;
  `E-ENUM-BOOL`) under Display > Scene gives `detail_px` (2, 1.5, 1 px);
  merge stays at half; saved, restored and reset with Display.
- **Headless.** `verify_lod` prints
  `LOD patches=<per height> shake_mm=<worst> neighbours=<PASS|FAIL> same_level=<n> cross_level=<n> PASS`;
  two runs byte-identical; goldens gain the line (documented in the PR,
  `E-GOLDEN`). `--screens` gains `lod-overlay.png` (F3 overlay shot).

## Consequences

Extends ADR 0033 (fixed faces become LOD patches). `E-STDOUT-PROTOCOL` gains
the `LOD` line; `E-RENDER-NO-MESH` now reads one flat-tone patch mesh per
drawn patch plus one body per moon (plus the player figure and billboards).
Measured on the dev PC (Intel UHD 620) at 1600x900: headless counts 1404 at
2,000 km rising to 1884 at 1 m (full sphere, no culling yet); visible is
about half (~942 patches, ~1.9 M tris) inside the 1000-2000 patch and 2-3 M
tri budgets; the 100/frame apply budget holds 60 fps. Cracks, morphing and
skirts (#777), culling (#778), ground (#779) and light (#780) come later.
