# 0027 — Planet squares shader

- Status: proposed
- Date: 2026-10-13
- Issue: #626 (Spec v2, owner answers R1-R6, Telegram 2026-10-13)

## Context

The owner asked to see the planets drawn in black-and-white squares as an
experiment, behind a setting (R1 A), as a 3D chessboard of 100 m cubes
(R2 A, R3) that fades to gray once a square is only a few pixels on screen
(R4 A). Until now every drawn surface used `StandardMaterial` with vertex
colours, and the game had no shader of its own. Squares glued to the
planet need the planet-space position of every pixel, which no standard
material provides. They also have to work at every level, from the whole
planet (cell units of about 10^7 m) down to the plot (about 10^2 m), and in
`f32` on the GPU.

## Decision

1. The planet surface and the moons draw with one material,
   `SurfaceMaterial = ExtendedMaterial<StandardMaterial, PlanetSquares>`.
   Its only shader is a fragment shader, `planet_squares.wgsl`, embedded
   in the binary (`embedded_asset!`), so there are no asset files to ship.
2. With the Planet look set to Normal, the shader is the standard unlit
   path (`pbr_input_from_standard_material`, then
   `main_pass_post_lighting_processing`), so the Normal look is
   unchanged, alpha fade included (#603).
3. With Squares, the shader maps the camera-relative world position (ADR
   0026) to planet metres with one uniform: the render-to-planet rotation,
   metres per render unit, and the render origin's planet position. The
   CPU computes that position in `f64` and reduces it by the pattern's
   200 m period (`universe_core::squares::origin_in_period`), so the GPU
   only sees small numbers at every level. `fwidth` of the square
   coordinate drives the fade to gray.
4. `universe_core::squares` is the pure `f64` reference of the pattern
   and the fade, pinned by tests. The shader mirrors it.
5. `PlanetSquaresPlugin` registers the material in the window app. The
   headless and test apps only keep its asset store.

## Consequences

- This is the first custom shader. It uses only `floor`, `fwidth`,
  `smoothstep` and `mix`, which every WebGPU, Vulkan, DirectX 12 and
  GLES backend supports. Running on phones is checked by the owner test.
- The squares look replaces the vertex tones while it is on (R5 B). The
  edge lines turn orange (`SURFACE_EDGE_SQUARES_TINT`, R6 A), and a look
  change rebuilds the surface.
- Any future surface look (for example texture tests) can extend
  `PlanetSquares` or add another extension in the same place.

## Note (#665): tunable values and debug views

- The shader takes the square size, two colours (`COLOUR_A`, `COLOUR_B` in
  `universe_core::squares`), the fade sizes and a debug view from
  `universe_render::SquaresTuning`, instead of writing them directly.
- The far fade goes to the mean of the two colours. With black and white
  that is the same 0.5 gray as before.
- The origin is reduced by twice the tuned square size
  (`origin_in_period_for`).
- The debug views (position, normal, distance) live in this shader because
  the engine lab draws the game's own surface material. The game's
  `SquaresTuning` is always the default (100 m, black, white, 8 px, 2 px,
  no debug view), so its picture is unchanged. Only the engine lab tunes
  it. A test pins the defaults.
