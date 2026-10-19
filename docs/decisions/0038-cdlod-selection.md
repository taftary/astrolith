# 0038 — True CDLOD selection (pop-free level switches)

- Status: accepted
- Date: 2026-10-19
- Issue: #817

## Context

After #777 (ADR 0036), patches still switched level while their vertices were part-way through the blend:
- up high: about 2.3 px popping and normals jumping 10-20°;
- near the ground: pops were measured only once a hidden bug was fixed. The selection measured each patch at the mean sphere, not at its terrain height, so on mountains and ocean floors kilometres off it the tree stopped near level 11 (3 km patches). Found in #779, fixed here (Q1 A).

The cause was the selection rule: screen error with a merge band, from numbers unrelated to the morph band. Owner answers Q1-Q6 were all A. Spec v4 adds F1 A (ranges from patch size), F2 A (High 4 / Normal 8 / Low 16 px per quad), the level ramp of the roughness scale, and edge masks against the whole tree.

## Decision

- **Real height, patch-aligned box (Q1 A, Q5 A).**
  - `lod_box`: each patch's box has the patch centre's up and two tangents as axes, at its real height range. It is grown by the cap bulge and the parent's chord sag, so the box distance is a lower bound of every own and parent vertex distance.
  - A planet-axis box was tried first. It spread a tilted patch's height range sideways, and about 14,000 patches split for nothing.
  - Unbuilt patches take their nearest built ancestor's heights.
  - The relief lookup carries `(relief_error, min_h, max_h)`.
- **Ranges (Q2 A, F1 A, F2 A).**
  - `lod_range::range_m(level, s)` is `(edge / 32) × s × focal / quad_px`: at the end of a range one quad covers `quad_px` pixels. Ranges double per coarser level.
  - `PlanetDetail::quad_px` is High 4, Normal 8, Low 16. At Normal 4 px the worst culled frame needed 2,765 patches and 6.37 M triangles, over #778's budgets.
- **Roughness scale (Q3 A).**
  - `Relief::roughness` is the mountain-belt mask in `[0, 1]`. The scale `1 + 3 × roughness` lives on a per-face lattice with 64 cells and canonical seams, interpolated bilinearly, so both sides of every edge agree.
  - A patch splits by `scale_max`, the exact maximum over its cells.
  - `scale_cap(level)` is 1× up to level 6, then rises ×1.41 per level to 4× from level 10. A full 4× on huge patches forced plain neighbours two levels apart, which popped at 250-2,000 km.
- **Selection (Q2 A).** A leaf splits when its box distance is under `range(ℓ + 1, s)`. A parent merges when its box distance is at least that. There is no hysteresis band, so a still camera causes 0 splits and 0 merges. #778's view culling and the neighbour rule stay.
- **Per-vertex blend on its own range (Q4 A).** A vertex's morph error is `(edge / 32) × min(s_v, cap)`, so the shader's band end is exactly its level's range, with the last 30% blended. Children therefore appear as their parent's shape, within one frame of travel: `children_appear_fully_blended`.
- **Edge masks against the whole tree** (a #778 fix). In the window, masks read the full tree's draw set (`PatchTasks::cover`), and a change in it also resyncs masks. Headless uses `blend_frame_in` and `crack_stats_in`. Before, a hidden coarser neighbour left a visible edge unpinned, and it snapped when the view moved.
- **Measures (Q6 A).**
  - `MORPH` drops the 3 km split: popping and normals over the whole path, run through the culled selection as the window draws it.
  - The `MORPH` unit test is removed; the golden `MORPH` line runs the same path, and its PASS carries every limit.

## Consequences

- Whole path: popping 0.635 px, normals 1.705° (were 0.680 px and 1.484° below 3 km, with 2.3 px and 14° above). Cracks 0. Mountains reach level 11 against 9 for plains.
- `CULL` at Normal: worst frame 907 patches, 2.09 M triangles (budgets 2,000 and 3 M).
- Near the ground the tree now refines fully: the `LOD` line reads 2,148 patches at 1 m, unculled.
- Frame time on the dev PC is level with `main` or slightly better in the same session (#819 still owns the floor).
- One `--verify` run takes about 70 s, and a full validation about 370 s (over the 300 s target, against 813 s in #777).
- ADR 0036's band rule (`err × focal / detail` from the patch error) is replaced by the level range. Its shader and edge rules stay.
