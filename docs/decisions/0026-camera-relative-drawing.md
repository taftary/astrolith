# 0026 — Camera-relative drawing

- Status: proposed
- Date: 2026-10-12
- Issue: #558 (owner decision A, Telegram 2026-10-12)

## Context

Normal mode (#558) puts a true-size body, about 1.8 m, at every level from
the whole planet to the room. Positions live in `f64` open-cell units, but
every drawn position was narrowed to `f32` from the cell centre
(`to_vec3`), and the camera sat at its own `f32` position. At L10 one cell
unit spans about 18,000 km, so the body is about 1e-7 units tall about
0.35 units from the centre. `f32` resolves about 3e-8 there, which leaves
the body only about three steps tall, and a third-person camera a few body
heights away jitters by the same steps. Region levels and below had
enough precision; the whole planet did not. The owner chose to fix drawing
for good (A) rather than draw the body larger at L10.

## Decision

1. `RenderOrigin` (a render resource) holds the camera's `f64` position in
   open-cell units. `sync_camera` sets it every frame before drawing, and
   puts the camera at the render origin (`Vec3::ZERO`).
2. Every drawn position is taken relative to the origin in `f64` and only
   then narrowed: `RenderOrigin::at` for points, `RenderOrigin::cell` for
   the cell centre (what `Vec3::ZERO` meant before), and
   `RenderOrigin::form` for position-valued form parameters (thread
   endpoints). `to_vec3` remains for directions and for vectors already
   taken relative.
3. Mesh entities carry `Anchored(point)`. `place_anchored` re-places them
   relative to the origin every frame. The surface mesh builds its vertices
   relative to the camera at build time (its anchor), so the ground near
   the camera keeps full precision too.
4. The near plane may go down to 1e-12 (infinite reverse-Z projection), so
   a body-scale gap is never clipped.

## Consequences

- A 1.8 m body on the whole planet keeps its shape. A test pins that the
  head-to-foot distance survives within 0.1% relative to the camera, and
  that it does not survive when drawn from the cell centre.
- Headless runs have no window, so the origin stays at the cell centre.
  Goldens and window tests are unchanged.
- New drawing code must use `RenderOrigin` for positions. A plain `to_vec3`
  of a position draws in the wrong place as soon as the camera moves.
