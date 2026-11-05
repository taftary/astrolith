# #1161 T4 (#1206): ground material

## What changed
- **Core:** each tile stores its patch centre in body-fixed metres, wrapped into the close detail's 4,096 m period (`grid_tile::P_NOISE`, `NOISE_PERIOD_M`, `noise_origin`).
- **Render:**
  - `GroundLook` (uniform 102) carries the world type's palette in linear colour, the radius, the relief scale, the close-detail reach and an airless flag. It is written once per planet or Terrain-tier change (`GridAtlas::set_look`).
  - **Vertex shader (`patch_grid.wgsl`):** passes each point's place on the body and a per-vertex mid-scale tone.
  - **Fragment shader (`patch_grid_fragment.wgsl`):** colours each point from the palette by height, slope and latitude (polar caps, snow up high on Earth-like and ice worlds). It adds a two-octave close detail that fades with the pixel size and with distance. Only the surface colour changes; the light inputs stay as in #1160 C5.
- **S1:** the detail's reach follows the effective Terrain tier: Low 150 m, Normal 300 m, High 600 m (`detail_reach_m`). No new setting.
- **Lava palette:** made lighter, to fresh basalt.
- **No new render pass and no texture.** Per planet, only the uniform's values change. So P7 needs no ADR.

## Tests
- `grid_tile_tests::the_close_detail_meets_across_patch_edges`: on every face, a shared edge point gets the same place in the close detail from both patches, within 1 cm (criterion 6).
- `ground_look_tests`: Terrain tiers order the reach (S1, criterion 11), every type has its own ground colour (criterion 1), and the shader's struct and constants mirror the Rust ones.
- **Checks:** workspace tests, clippy `-D warnings`, fmt and docs pass; `--verify` gives `VERIFY-OK`.

## Pictures
`pictures/` (`SCREENS-OK files=39`) and `after-sheet.png`, against `../before/before-sheet.png`.
- Each type now has its own ground: Earth-like green and brown, ochre desert and Mercury (with its craters), dark iron, white and blue-grey ice, dark basalt lava. The lava glow comes in T5 and the moon in T6.
- Up close the ground shows a fine grain; from far, a mid-scale tone variation.
- No seam is visible on the cube-edge views.

## Speed (same PC, routes and settings as `../before/`)
The first version cost 8 noise octaves plus the mid tone per pixel. It took the ground legs from about 40 to 75 ms, so it was replaced: the mid tone is now per vertex, the close detail is two octaves at the pixel's scale, and the lattice wraps once per read.

| route | interval before | after | change | render before | after | >100 ms before | after |
|---|---|---|---|---|---|---|---|

- **Normal (fixed):** the whole route is within 5%: 2.9% quicker, inside the VSync-lock noise. The orbit legs average about 2.6 ms more, the ground legs about 3 ms less (lock flips).
- **Low:** cheaper than High, and within noise of before.
- **High:** 14.6% slower than before. It is the opt-in tier for more detail.
- **Open:** the orbit-leg cost. The T8 paired repeats will check it.
