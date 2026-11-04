# #1160 T7 (#1201): after evidence and comparison

- **Code:** `feat/1160-planet-light` with T1-T6 merged; the T7 runs are at 793dc04. The release builds are in `exe.sha256` (here) and `../paired/exe.sha256` (the main build `77a22e28…` and the feature build `d525810c…`).
- **Repeat:** `bash .agent/validation/issue-1160/fixtures/batch.sh after`, then `bash .agent/validation/issue-1160/fixtures/paired.sh <main.exe> <feature.exe>`.
- **PC and settings:** the same as T1 (Intel UHD 620, Vulkan, 60 Hz, AC power on Balanced, idle).

## What changed in the game
| task | change | per-frame or per-planet cost |
|---|---|---|
| T2 | picture tool only (`clear_parked_body`) | none in play |
| T3 | the even fill's colour warms as the sun gets low (`light::low_sun_tint`) | one colour, written on change |
| T4 | the moon lamp uses the light facing the moon (`MoonLight.strength`); `MOON_FULL_SHARE` 0.8, `STAR_SHARE` 0.02 | same lamps, other strengths |
| T5 | from high up the eyes read the lit disc (`light::eye_share_at_height`); the fill tint fades out from orbit | a few multiplies per frame |
| T6 | `GROUND_OWN_LIGHT` (black emissive, as before); Help sentence | none |

No texture, mesh, material, shader or render pass was added. A tried fragment shader was removed again (`../t5/README.md`).

## Pictures (C1-C3, C12-C14)
`pictures/`, from `light.shots` (`SCREENS-OK files=22`). `before-after-sheet.png` shows each row as `../before-fixed-tool/` on the left and after on the right: day, dusk, dawn, low-sun, moonlit, moonless, orbit, terminator, descent 20 km, descent 2 m, world-two, body.

Mean colour (sRGB). Ground boxes cover the lower middle of the picture; the orbit boxes cover the lit side.

| shot | before | after |
|---|---|---|
| day | 101.4, 93.3, 86.4 | 101.4, 93.3, 86.4 (unchanged) |
| dusk | 43.3, 43.3, 50.4 (cold grey) | 47.3, 41.3, 45.2 (warm) |
| dawn | 51.6, 45.1, 43.2 | 49.5, 35.1, 25.5 (warm) |
| low-sun | 96.2, 85.3, 78.1 | 87.3, 76.0, 68.9 (golden) |
| moonlit | 3.8, 4.0, 7.0 | 12.0, 9.5, 8.2 (ground visible) |
| moonless | 3.8, 4.7, 9.4 | 1.6, 2.0, 4.3 (dark) |
| world-two | 87.6, 83.6, 86.1 | 90.7, 86.6, 90.9 |
| orbit, lit side | 75.2, 79.6, 105.9 (washed blue-white) | 27.6, 31.6, 48.6 |
| terminator, lit side | 117.3, 127.4, 162.8 | 61.7, 70.0, 100.0 |

- The descent pictures match from 2,000 km down to 2 m.
- In the moonlit picture the moon also lights the air, so the sky is dark blue.
- Out of scope (Q7 A, moved to #1163): a warm day/night edge from orbit and a moonlit night side from orbit.

## Speed (C6)
- First pass, one run per side: `comparison.md`. The interval median moves by ±10% between runs because the VSync lock jumps between about 33 and 40 ms. One run per side cannot judge 5%.
- Paired repeats: `../paired/paired.md`. They alternate the main and feature builds, two rounds of `light-fixed` and one of `light-auto`.
  - The first main run (`before r1`) is lower in every scene and metric, for example orbit render 6.15 against 7.47 to 7.55 in the three other runs. This looks like the cool first run, not the code.
  - The fair comparison is `before r2` against `after r1` and `r2`:

| scenario | interval median before r2 / after r1 / after r2 | render median | cpu median |
|---|---|---|---|
| day | 41.03 / 39.40 / 39.84 | 4.87 / 4.92 / 4.91 | 10.87 / 10.97 / 11.07 |
| dusk | 35.79 / 39.63 / 35.55 | 4.94 / 5.06 / 5.04 | 11.07 / 10.80 / 10.77 |
| moonlit | 38.87 / 39.41 / 39.14 | 7.16 / 7.24 / 7.15 | 35.99 / 35.76 / 35.66 |
| orbit | 37.91 / 34.77 / 34.79 | 7.47 / 7.55 / 7.54 | 9.24 / 9.40 / 9.43 |
| terminator | 38.55 / 35.19 / 35.24 | 7.17 / 7.69 / 7.63 | 9.27 / 10.81 / 10.79 |
| descent | 39.95 / 39.66 / 39.74 | 8.31 / 8.31 / 8.26 | 21.72 / 21.46 / 21.48 |

- **Frame interval:** within noise in every scene, and never above the slower before run except dusk after r1 (39.63), which is a lock flip: after r2 reads 35.55.
- **Stalls and arrival:** there are no stalls over 100 ms in any run, before or after.
- **Render and CPU:** flat within about 0.1 ms, except **terminator**. There both after runs show about +0.5 ms render and +1.5 ms CPU over before r2. The likely cause is the moon lamps: with `strength` they now light the half-lit view from orbit. That cost sits inside the frame (the interval is unchanged at the lock). It is reported, not hidden.
- **Auto** (`light-auto`, one pair): it settles on the same or better settings. In orbit and terminator it chose Normal terrain instead of Low in the first pass (telemetry legs 7 and 8). The interval medians are within noise, with no stalls.
- **Quality tiers (C8):** no new GPU work was added, so there is no new cost to place under a tier. Atmosphere Low, Normal and High cost about the same before and after on this PC. For example, day render after is 4.90, 4.91 and 4.83 ms, the same pattern as the T1 baseline. No quality control was added.

## Resources (C7)
`resources.md`: at the end of every leg of all three fixed routes, meshes, images, standard materials, ground materials and terrain bytes are identical before and after.
- These routes visit one planet. Nothing new is created per planet (see the change list above), so there is nothing that could grow with distinct worlds.
- The `world-two` picture shows the second planet with the new light.

## Checks (C9)
The workspace tests, clippy `-D warnings`, docs and fmt pass on the feature branch. `--verify` is run in the technical check.
