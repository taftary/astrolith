# 0046 — Stale planet sky after leaving

- Status: accepted
- Date: 2026-10-21
- Issue: #880 (Spec v1)

## Context

Diving into the planet (L10) and rising back out left a gray circle where the
planet was. It stayed forever, fixed on the screen from L9 up to L7, and
flashed on some frames. The owner dove in dive mode the whole time with a fast
mouse wheel, standing still or moving.

The background was the clue: the normal dive background is flat dark gray
(14, 15, 16), while the bug showed an almost-black purple-tinted background
with a filled shaded disc at the screen centre. That is the planet's atmosphere
sky still drawn from space, with the planet as a small gray disc. Leaving from
inside the air showed a blue day sky instead.

Main world: `air::sync_air` removes `AtmosphereSettings` and despawns
`PlanetAir` in the same frame L10 closes. Render world (Bevy 0.19):
`extract_atmosphere` only clears its `ExtractedAtmosphere` copy on cameras that
still have `AtmosphereSettings`, so the prepared atmosphere on the retained
camera entity (`ExtractedAtmosphere`, `GpuAtmosphere`, `AtmosphereTextures`,
`AtmosphereTransformsOffset`, `DynamicUniformIndex`) is never removed, and
`render_sky` keeps drawing with the last planet's data.

## Decision

- **Clear the engine's copy of the sky (`air.rs`, `E-PIN-BEVY`).**
  - New render-world system `clear_stale_sky` runs in the extract step after
    Bevy's own `extract_atmosphere`.
  - For every render entity holding sky data (`ExtractedAtmosphere`,
    `GpuAtmosphere`, `AtmosphereTextures`, `AtmosphereTransformsOffset`,
    `DynamicUniformIndex`) but no `GpuAtmosphereSettings` (which mirrors the
    game-side `AtmosphereSettings` through Bevy's `SyncComponent` and is
    already gone when L10 closes), it removes the prepared sky components.
  - Without them, Bevy's sky passes (`render_sky`, `atmosphere_luts`) no
    longer match the camera, so nothing is drawn. The private bind groups
    (`AtmosphereBindGroups`, `RenderSkyPipelineId`) are not named: the passes
    already require the removed public types.
  - Writes only when something is stale (`E-HOT-NOALLOC`): the query only
    matches entities holding sky data, and on-planet cameras still have
    settings so no command is queued at rest.
- **Pure rule plus tests.**
  - `stale_sky(has_settings, has_sky_data)` decides. Test
    `stale_sky_is_cleared` checks the rule and runs the system: stale sky
    data goes, on-planet sky stays, and a second frame writes nothing.
  - The existing `planet_air_comes_and_goes_with_the_planet` stays green
    (sky back on return).

## Proof run

Shot file (six shots in one window run, so the render world persists across
the L10 to L9/L7 transitions like the owner's dive):

```text
ground L10 lat=40 lon=106.4 h=2 heading=270 tilt=5
after-ground-l9 L9 x=0 y=0 z=3 dir=0,0,-1
after-ground-l7 L7 x=0 y=0 z=3 dir=0,0,-1
orbit L10 lat=40 lon=106.4 h=2000000 heading=0 tilt=-90
after-orbit-l9 L9 x=0 y=0 z=3 dir=0,0,-1
after-orbit-l7 L7 x=0 y=0 z=3 dir=0,0,-1
```

Release build, Intel UHD 620 (Vulkan), one run per branch.

After (this branch, with the fix): the L9/L7 corners are the normal dive
background, and ground-vs-orbit starts agree byte-for-byte.

| Picture | Top-left | Top-right | Bottom-left | Centre | File bytes |
|---|---|---|---|---|---|
| after-ground-l9 | (14,14,17) | (15,15,17) | (14,15,16) | (32,8,24) marker | 1149564 |
| after-orbit-l9 | (14,14,17) | (15,15,17) | (14,15,16) | (32,8,24) marker | 1149564 |
| after-ground-l7 | (14,14,17) | (13,14,17) | (14,15,16) | (16,8,6) marker | 1134060 |
| after-orbit-l7 | (14,14,17) | (13,14,17) | (14,15,16) | (16,8,6) marker | 1134060 |
| ground (L10) | (4,19,48) sky | (4,19,48) sky | (4,20,47) sky | (4,19,47) sky | 817277 |
| orbit (L10) | (15,16,19) | (15,16,19) | (15,16,19) | (23,50,93) planet | 1085888 |

Normal dive background is (14,15,16); the L9/L7 corners match within 1-2
(tonemap/bloom), and the centre pixel is the planet marker, not a sky disc.

Before (main `b1021dc` without the fix): leaving from the ground keeps the
blue day sky at L9 and L7; leaving from orbit is already clean (the orbit
start never reproduced, as in the issue notes). Ground and orbit L10 shots
match the after run (the sky itself is unchanged).

| Picture | Top-left | Top-right | Bottom-left | Centre | File bytes |
|---|---|---|---|---|---|
| after-ground-l9 | (35,50,80) sky | (40,59,91) sky | (59,84,120) sky | (34,12,29) | 1111458 |
| after-ground-l7 | (33,46,74) sky | (31,45,72) sky | (30,44,70) sky | (18,11,11) | 1089963 |
| after-orbit-l9 | (14,14,17) | (15,15,17) | (14,15,16) | (32,8,24) marker | 1150487 |
| after-orbit-l7 | (14,14,17) | (13,14,17) | (14,15,16) | (16,8,6) marker | 1134151 |
| ground (L10) | (4,19,48) sky | (4,19,48) sky | (4,20,47) sky | (4,19,47) sky | 817277 |
| orbit (L10) | (15,16,19) | (15,16,19) | (15,16,19) | (23,50,93) planet | 1085888 |

The ground-path L9/L7 corners (33-59) are far from the normal (14,15,16):
the planet's sky. After the fix the same shots read (14,15,16).

## Consequences

- After leaving L10 (ground, air, or orbit, slow or fast), no frame draws the
  planet's sky; the normal dive background returns at once and stays.
- Diving back brings the sky back as before.
- No change to how the sky looks on the planet, and no Bevy change
  (`E-PIN-BEVY`).
- The small black Sun square at L7 seen during testing is separate, not part
  of this fix.
