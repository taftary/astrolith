# 0040 — Atmosphere and light

- Status: accepted
- Date: 2026-10-20
- Issue: #780

## Context

Steps 1-6 of the planet rebuild (ADRs 0032-0039) drew the ground unlit in cell units, under a black sky. Step 7 gives the planet real light and air. The owner answered Q1-Q7 all A, said yes to S1 (a Shadows setting) and G1 (a day-and-night Help sentence), and then chose F1 A (arrive in daylight), F2 A (draw the planet in metres) and F3 A (Vulkan on Windows, ADR 0041).

## Decision

- **One sunlight (Q1 A).**
  - `light::sun_direction` is the planet's fixed sun at L10.
  - `sun.rs` points one directional light along it. The ground (with its morphed normals), the moons and the figure are lit by it.
  - Stars, galaxies and markers stay unlit glowing points.
  - `SUN_ILLUMINANCE` = 1.2π, so a lit surface shows its base colour at the default exposure.
- **Night (Q2 A).** The ambient floor is `NIGHT_SHARE` (7.5%) of the daylight at the planet and `OFF_PLANET_FLOOR` above it. Shapes stay readable at night.
- **Air (Q3 A).**
  - One `atmosphere::AirModel` per planet comes from the recipe's air: Rayleigh, Mie, ozone and ground albedo. Thick air is amber.
  - The air's bottom is the relief low bound, and it keeps sea-level density below sea level.
  - `air.rs` builds a Bevy `ScatteringMedium` from the same densities and anchors one `Atmosphere` at the planet centre.
  - The camera uses the lookup tables inside the air and raymarching from 10 km above its top (`SPACE_MODE_ABOVE_M`).
  - The aerial-haze reach follows the height (`aerial_reach_m`): 32 km on the ground, up to the horizon higher up.
- **Near shadows (Q4 A, S1).**
  - At the planet the sun casts cascaded shadows from the ground (through the morph prepass shader) and the figure, out to 3 km (`SHADOW_CASCADES_M`).
  - Settings > Display > Scene > Shadows (default On) turns them off. Far away, the night side is the shadow.
- **Fixed time of day (Q5 A).** The planet does not spin, so every run looks the same.
- **Measure (Q6 A).**
  - The headless `LIGHT` line runs the #777 path from 2,000 km to 1 m and back under the fixed sun. It checks the largest frame step of sky, ground and haze (under 2%), the night-to-day ratio (5-10%), and a bit-for-bit repeat.
  - Seven sky shots (`screens_sky.rs`) cover orbit to the night side.
- **Metres (F2 A).**
  - At L10, world units are metres (`RenderOrigin::scale`, `CellSized`, `l10_drawn_m_f32`).
  - The reason: Bevy's shadow cascades round their size to whole world units, so near shadows can't work in cell units.
- **Daylight arrival (F1 A).** The seeded spot and the dive's L10 entry sit on the morning side, 40° from the sub-solar point (`player::MORNING_RAD`), with at most 12° of hashed spread.

## Frame time

Measured on the dev PC (Intel UHD 620) at 1600x900 in `--screens` settle frames, as mean milliseconds per frame. The target was a steady 60 fps (16.7 ms).

| View | No air, no shadows (DX12) | Air + shadows (DX12) | Air, no shadows (DX12) | Lower quality, paced (Vulkan) |
|---|---|---|---|---|
| L9 approach | 46.5 | 46.5 | 47.6 | — |
| Planet from orbit | 44.6 | 55.3 | 53.6 | 54.8 |
| Walking, third person | 54.7 | 71.8 | 59.8 | 105.3 |
| Sky from orbit (raymarched) | 57.2 | 83.6 | 86.0 | 133.6 |
| Sky at 1 km | 53.7 | 61.6 | 70.0 | 109.3 |
| Sky on the ground | 54.6 | 67.9 | 69.6 | 95.9 |

- The target is not met. The base without air or shadows is already 45-57 ms: that is the existing frame-time floor, owned by #819.
- The air adds about 5-15 ms near the ground and about 29 ms raymarched from orbit. Shadows add about 10-12 ms on the ground.
- Quality was lowered first (Q7 A):
  - Bevy's sky table is halved to 200x100, with 10 samples.
  - Raymarching uses 8 samples.
  - Two shadow cascades replace four.
  - The aerial table keeps 64 slices: 32 showed bands from 100 km.
  - The steps save about 5-9 ms, and the pictures stay the same.
- The Vulkan column waits for the GPU every frame (ADR 0041). So it shows the full CPU-plus-GPU time with no overlap: the true cost of a frame on this GPU.
- No feature is turned off: that needs the owner's answer (Q7 A).

## Consequences

- `LIGHT frames=1743 sky_step_pct=0.894 ground_step_pct=0.689 haze_step_pct=1.512 night_ratio=0.078 repeat=identical PASS`.
- `PRECISION` reads 0.028615 mm in metres. The `MORPH`, `CULL`, `DEPTH` and `GROUND` lines still pass.
- The night side looks almost black over the dark ocean-floor tint, although the model gives 7.8% of daylight.
- In walking mode by day, faint dark dots can show in the bright sky. They were invisible against the old black sky, and their source is still open.
- On DirectX 12 the dev PC corrupts the atmosphere tables, so Windows now uses Vulkan (ADR 0041).
