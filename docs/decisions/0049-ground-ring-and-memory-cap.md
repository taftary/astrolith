# 0049 - Ground ring, nearest-first builds and the ground memory cap

- Status: accepted
- Date: 2026-10-22
- Issue: #918 (Spec v2)

## Context

On arrival at the ground the patches took seconds to build, the ground behind the camera rebuilt on every turn, and every patch cost about 350 KB (three copies: ours, Bevy's CPU copy and the GPU copy). The cache could hold 2,048 patches, up to 186 MB on the CPU alone: too much for a phone (#918 notion, review on the Issue).

Owner answers:

- Q2 A: keep ground already seen. Q3 A: nearest first, limited work per frame. Q5 A and P1 B: a 500 m ring all around the player.
- P2 A: a 64 MB ground memory cap; the cap wins over far detail.
- S1 yes: a Ground memory limit row under Settings > Display > Scene, 32 / 64 / 128 MB.
- The shared-grid mesh (about 4-8 KB per patch) is #920, after this.

## Decision

- **Ring (P1 B).** `lod_ring`: every leaf whose relief box is closer than `RING_M` (500 m) splits and builds as if seen, behind the camera too. The box distance does not depend on the view, so turning never changes the ring's leaves.
- **Nearest first (Q3 A).** The build list orders every wanted leaf (visible or in the ring) by box distance, so the ring's leaves come first. Far builds wait until the ring is complete, so workers stay free for ring leaves the refining tree still adds. Up to six builds per worker thread are in flight, and finished builds drain before new ones spawn (#918 T8).
- **Lean patches (review Step 1).** No second CPU copy in Bevy (`RenderAssetUsages::RENDER_WORLD`). The colour and edge flags live in the material and shader, not per vertex, and normals are packed: a drawn patch uses about 72 KB on the GPU.
- **Resident cache (Q2 A).** `patch_cache.rs`: every cached patch keeps its GPU mesh and a slim summary (centre, relief numbers, the F3 edge points). The full CPU mesh stays only within `NEAR_CPU_M` of the player, where the body stands on it. Near patches that lost it rebuild first.
- **Memory cap (P2 A, S1).** The cache counts its bytes (CPU and GPU, from the buffer sizes). Over the limit, the patches farthest from the camera that the frame does not need go first; drawn, wanted, ring patches and the roots never go. Build room counts only what the frame needs. Over the limit the whole selection coarsens in slow steps (detail bias, 1.15x per step after 10 blocked frames, back after 60 frames under 70% of the limit, at most 4x). The alternative, leaving far patches unbuilt, drew their coarse parent under the finer ones: double geometry, 8 ms more per frame looking down.
- **Merges under the cap.** Splits cascade, so a parent is often never built, and a merge needs the parent's relief numbers. An unbuilt parent with four cached children reads their numbers together (`relief_or_children`), draws them until its own patch is built (no pop), and builds even without room: one patch replaces four. Without this the far ground could never coarsen under the cap.
- **Setting (S1).** `GroundMemory` (`settings_ground`), saved as `ground_memory_limit = 32|64|128`, default 64, reset with Display. The ground reads it every frame, so a change applies at once.
- **F3.** The outlines pass through all 33 edge points of each side, pinned points at their pinned place. With F3 on at the planet, the HUD block adds `ground memory <held> / <limit> MB`, rounded down.

## Tests

- `ground_memory_stays_under_the_cap`: with a 6 MB limit, every frame stays under it or holds only patches the frame needs, and full CPU meshes stay near the player.
- `the_body_stands_on_held_ground`: near drawn patches always hold their CPU mesh.
- `a_merge_keeps_its_children_until_built` and `the_memory_bias_steps_slowly`.
- `ground_memory_limit_setting` and `ground_memory_limit_round_trips`.
- `outlines_follow_every_edge_point`; the headless `RING` line (0 builds and 0 ring changes over a 360 degree turn, nearest first).
- Window: `--screens <dir> --shots ground.shots --timing` prints `ring_ms` (ring ready) and `ready_ms` (everything wanted) per shot.

## Consequences

- On the dev PC (UHD 620, release), the 500 m ring is ready 0.38 to 0.97 s after a jump straight to the ground on all five test spots. Frame time is 29 to 35 ms, against 31 to 39 ms on main before #918.
- Peak ground memory stays under 64 MB. Looking toward the horizon, the far ground is a little coarser (detail bias up to about 1.3x) than without a cap.
- `ground-far` has the thinnest margin (about 0.95 s). More margin needs cheaper builds: #920.
