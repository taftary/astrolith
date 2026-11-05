# #1160 T3 (#1197): warm low light

**Cause.** At a low sun, the direct sunlight is already reddened per pixel by Bevy's atmosphere transmittance (`pbr_lighting.wgsl`, `#ifdef ATMOSPHERE`). The even fill (`GlobalAmbientLight`), however, stayed white, and at sunset it is most of the light on flat ground, so the ground read cold grey.

**Change**
- `universe_core::light::low_sun_tint(height)` is new. It returns white with the sun at 30 degrees or higher, warms smoothly to `LOW_SUN_TINT` (1.0, 0.62, 0.38, linear) at the horizon, and stays there below the horizon. Below the horizon, the moonlight model's fill hands it over to the moon's cool tint, as before.
- `sun.rs::fill_colour` takes the sun's height at the camera.
- There is no new pass, texture or material: only the colour of the existing ambient light changes, once per frame on change.
- The fill strength is unchanged.

**Test:** `light::tests::the_fill_warms_smoothly_as_the_sun_gets_low` checks that the fill is white at noon, golden at and below the horizon, never cools as the sun rises, and moves less than 0.02 per channel for each step of 0.002 in height (no step). The workspace tests pass.

**Pictures** (`check.shots`, the same shots as `light.shots`; before is `../before-fixed-tool/`)
- `try1/` used the first tint (1.0, 0.7, 0.45 from 20 degrees). `try2/` uses the final tint (1.0, 0.62, 0.38 from 30 degrees).
- `try2-sheet.png` shows dusk and low-sun, before then after.
- Mean ground colour in the lower middle of the picture (sRGB 0-255):

| shot | before | after (try2) |
|---|---|---|
| dusk 17:52 | 43, 43, 49 (cold grey) | 41, 34, 35 (warm) |
| dawn 06:26 | 52, 45, 43 | 51, 36, 26 (warm) |
| low-sun 16:52 | 94, 84, 77 | 99, 85, 75 |
| day 08:23 | 100, 92, 85 | 100, 92, 85 (unchanged) |

Dusk stays dark, because the sun is at the horizon. It now reads as warm dusk instead of cold grey, and the fade into night is continuous by the test above. Moonlit night is T4; orbit is T5.
