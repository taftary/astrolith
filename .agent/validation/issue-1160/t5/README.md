# #1160 T5 (#1199): the planet from orbit

**Cause of the washed-out day side.** The eyes adapted to the light under the camera. From orbit over the night side, that point is dark, so the eyes opened up by about 2.5 EV and the lit crescent blew out to white-blue.

**Change**
- `light::disc_lit_share(sun, camera)` is new: the lit share of the visible disc, `(1 + sun.camera)/2`.
- `light::disc_weight(height_m)` is new: 0 below 20 km, 1 from 400 km, smoothstep between.
- `light::eye_share_at_height(ground, disc, height)` is new. It mixes the light under the camera with the lit disc share by that weight, and is never darker than the light under the camera.
- `camera.rs::adapt_eyes` uses these with the camera's height over the planet radius. The eyes still ease with `ease_ev`, so the descent cannot jump.
- `sun.rs::fill_colour` makes the T3 warm fill tint fade to white with the same weight. From orbit the camera's own sun height says nothing about the disc it sees.

**Tests:** `light::tests::from_orbit_the_eyes_read_the_lit_disc` covers the disc share, the ground and orbit ends, "never darker than the ground", and a smooth, monotonic descent with steps under 0.005 per 500 m. The workspace tests and clippy pass.

**Pictures:** `orbit.shots` (orbit, terminator, descent from 2000 km to 2 m). `try1/` has the exposure change. `try2-sheet.png` shows each row before, then try1, then try2 (the try2 row was taken with the tried shader described below). The day side now shows darker blue and brown tones instead of white, and the descent rows match at every height.

**What Q4 A asked for and is not reached:**
1. **A warm edge at the day/night line.** Sampled across the terminator, the colour stays blue, for example (28, 26, 42) to (72, 81, 115). The blue is the air's in-scattered light, which Bevy's sky pass adds over the ground after the ground is drawn.
   - Tried: a ground fragment shader that warms each point's light by its own sun height from orbit (`tried/patch_grid_fragment.wgsl.txt`). It compiled and ran, but it changed the edge only from (25, 23, 36) to (28, 26, 42), because the haze on top stays blue.
   - It was removed: a new shader variant and per-pixel cost for no visible gain.
   - A visible warm edge needs a change to the air itself (Bevy's atmosphere medium or its sky pass). That is a rendering-architecture decision (P7, `E-PIN-BEVY`) and belongs with #1163 (atmospheric diversity). It goes back to the owner.
2. **A moonlit night side seen from orbit.** Exposed for the lit day side, the moonlit night (now about 6 times the moonless level on the ground, T4) is still crushed to black by the tone curve. Lifting it would need a separate night exposure or tone change. This also goes back to the owner.
