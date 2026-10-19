# 0039 — Near-field physics on the drawn ground

- Status: accepted
- Date: 2026-10-19
- Issue: #779

## Context

Since #770 (ADR 0031) the planet had no ground: the body floated and flew at true size around it. Steps 1-5 rebuilt the planet as a CDLOD cube-sphere with relief, morphing, depth and culling (ADRs 0032-0038). Step 6 puts the body back on the ground. Owner answers Q1-Q7 were all A, with G1 and G2 yes. F1 A fixed the selection height bug first, and F2 B moved that fix to its own Issue (#817, ADR 0038).

## Decision

- **One surface for the picture and the feet (Q1 A).**
  - `drawn_surface::DrawnSurface` holds the drawn patches (shared `Arc<PatchMesh>`), their edge masks and the morph camera.
  - `ground_at` takes the finest drawn patch under a point and blends its triangle with `blended_vertex`, exactly as the shader does. It then intersects the radial ray.
  - The window rebuilds the surface in `sync_patches` when the draw set changes, and moves its camera every frame.
- **A local frame near the ground (Q2 A).**
  - Below `LOCAL_BELOW_M` (100 m) above the ground, `body_frame::step_near_ground` steps the body in a `LocalFrame` (metres, east, north and up).
  - The frame re-anchors on the ground under the body once the body is `REANCHOR_M` (1 km) from the origin.
  - Above 100 m, or with no ground, the body steps in open-cell units as before.
  - `body_step_scaled` is the one step with a metre scale. `body_step` calls it.
- **Radial gravity, upright body (Q3 A, Q4 A).** The pull points to the planet's centre and the ground's up is radial, so the body never tilts. Any slope is walkable.
- **Start on the ground (Q7 A).**
  - Key 1 and the player shot place the body on the drawn ground.
  - When the surface is not built yet, the drop waits (`PlayerState::ground_pending`) and happens on the first frame that has ground.
- **Measure (Q6 A).**
  - The headless `GROUND` line walks 100 m and runs 100 m over the mountain slope and a plain at 60 fps, with one jump each, through the CDLOD selection and the drawn surface.
  - It records the feet gap every frame, the landing gap, and the local-frame step against the planet-frame step.
  - Limits: 1 cm for the feet, 1 mm for the frame switch.
- **Help (G1, G2).** Navigation and About get their walking sentences back.

## Consequences

- `GROUND walk_gap_mm=0.000 run_gap_mm=0.000 land_gap_mm=0.000 landings=2 frame_switch_mm=0.000 PASS`.
- The ground is the picture: a coarser patch or a mid-blend vertex moves the feet with the drawn surface, so the feet never float above or sink into what is drawn.
- Off L10, `ground_under` falls back to `level_ground` as before.
- Collisions with objects, slope limits and per-surface friction stay out of scope.
